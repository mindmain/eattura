use std::collections::{BTreeMap, BTreeSet, HashMap};

use chrono::{Local, NaiveDate};

use super::{ValidationError, ValidationResult};
use crate::sdi::v1::{
    Anagrafica, DatiTrasmissione, FatturaElettronica, FatturaElettronicaBody,
    FatturaElettronicaHeader, IdFiscale, Indirizzo, ScontoMaggiorazione, anagrafica,
};
use crate::xml::enum_map::{
    bollo_virtuale_to_sdi, causale_pagamento_to_sdi, condizioni_pagamento_to_sdi,
    formato_trasmissione_to_sdi, modalita_pagamento_to_sdi, natura_to_sdi, regime_fiscale_to_sdi,
    tipo_cassa_to_sdi, tipo_documento_to_sdi, tipo_ritenuta_to_sdi,
    tipo_sconto_maggiorazione_to_sdi,
};

fn report(
    errors: &mut Vec<ValidationError>,
    code: &str,
    path: impl Into<String>,
    message: impl Into<String>,
) {
    errors.push(ValidationError {
        code: code.into(),
        message: message.into(),
        element_path: Some(path.into()),
    });
}

const HEADER: &str = "Header";
const TRANSMISSION: &str = "Header/DatiTrasmissione";
const CUSTOMER: &str = "Header/CessionarioCommittente/DatiAnagrafici";
const SUPPLIER: &str = "Header/CedentePrestatore/DatiAnagrafici";

// ===========================================================================
// Public API
// ===========================================================================

/// Validate a `FatturaElettronica` against the supported local SDI rules.
///
/// Runs header validation, body validation for each body element,
/// and cross-cutting checks (e.g. document-type-specific rules).
/// Returns a combined `ValidationResult` with all errors found.
pub fn validate(invoice: &FatturaElettronica) -> ValidationResult {
    validate_on(invoice, Local::now().date_naive())
}

/// Validate with an explicit receipt date. Local preflight uses the current date.
pub fn validate_on(invoice: &FatturaElettronica, receipt_date: NaiveDate) -> ValidationResult {
    let mut errors = Vec::new();

    if formato_trasmissione_to_sdi(invoice.versione).is_none() {
        report(
            &mut errors,
            "00200",
            "@versione",
            "Invoice version must be FPA12 or FPR12",
        );
    }

    if let Some(header) = &invoice.header {
        errors.extend(validate_header(header).errors);
        if let Some(dt) = &header.dati_trasmissione
            && formato_trasmissione_to_sdi(dt.formato_trasmissione).is_some()
            && formato_trasmissione_to_sdi(invoice.versione).is_some()
            && dt.formato_trasmissione != invoice.versione
        {
            report(
                &mut errors,
                "00428",
                format!("{TRANSMISSION}/FormatoTrasmissione"),
                "FormatoTrasmissione must match the root versione attribute",
            );
        }
    } else {
        report(
            &mut errors,
            "00200",
            HEADER,
            "FatturaElettronicaHeader is required",
        );
    }

    if invoice.body.is_empty() {
        report(
            &mut errors,
            "00200",
            "Body",
            "At least one FatturaElettronicaBody is required",
        );
    }
    for (i, body) in invoice.body.iter().enumerate() {
        errors.extend(validate_body_indexed(body, i).errors);
        if let Some(date) = body
            .dati_generali
            .as_ref()
            .and_then(|data| data.dati_generali_documento.as_ref())
            .filter(|doc| is_iso_date(&doc.data))
            .and_then(|doc| NaiveDate::parse_from_str(&doc.data, "%Y-%m-%d").ok())
            && date > receipt_date
        {
            report(
                &mut errors,
                "00403",
                format!("Body[{}]/DatiGenerali/DatiGeneraliDocumento/Data", i + 1),
                "Invoice date is later than the receipt date",
            );
        }
    }

    validate_batch(invoice, &mut errors);
    validate_tipo_documento_rules(invoice, &mut errors);
    if errors.iter().filter(|error| error.code == "00200").count() > 50 {
        report(
            &mut errors,
            "00201",
            "FatturaElettronica",
            "More than 50 XML format errors found",
        );
    }
    ValidationResult { errors }
}

/// Validate the invoice header, including tax identifiers and routing.
pub fn validate_header(header: &FatturaElettronicaHeader) -> ValidationResult {
    let mut errors = Vec::new();
    let customer = header
        .cessionario_committente
        .as_ref()
        .and_then(|party| party.dati_anagrafici.as_ref());

    if let Some(dt) = &header.dati_trasmissione {
        if let Err(error) = validate_transmitter(Some(dt)) {
            errors.push(error);
        }
        if !(1..=10).contains(&dt.progressivo_invio.chars().count()) {
            report(
                &mut errors,
                "00200",
                format!("{TRANSMISSION}/ProgressivoInvio"),
                "ProgressivoInvio must contain 1 to 10 characters",
            );
        }
        let cd = &dt.codice_destinatario;
        let path = format!("{TRANSMISSION}/CodiceDestinatario");
        if !matches!(cd.len(), 6 | 7)
            || !cd
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
        {
            report(
                &mut errors,
                "00200",
                &path,
                "CodiceDestinatario must contain 6 or 7 uppercase alphanumeric characters",
            );
        } else if matches!((dt.formato_trasmissione, cd.len()), (10, 7) | (20, 6)) {
            report(
                &mut errors,
                "00427",
                &path,
                "CodiceDestinatario length does not match FormatoTrasmissione",
            );
        } else if cd == "XXXXXXX"
            && !customer
                .and_then(|party| party.id_fiscale_iva.as_ref())
                .is_some_and(|id| !id.id_paese.is_empty() && id.id_paese != "IT")
        {
            report(
                &mut errors,
                "00313",
                &path,
                "XXXXXXX requires a foreign IdFiscaleIVA for the customer",
            );
        }
        if formato_trasmissione_to_sdi(dt.formato_trasmissione).is_none() {
            report(
                &mut errors,
                "00200",
                format!("{TRANSMISSION}/FormatoTrasmissione"),
                "FormatoTrasmissione must be FPA12 or FPR12",
            );
        }
        if dt.pec_destinatario.as_deref().is_some_and(is_sdi_mailbox) {
            report(
                &mut errors,
                "00330",
                format!("{TRANSMISSION}/PECDestinatario"),
                "PECDestinatario cannot be an SdI mailbox",
            );
        }
    } else {
        report(
            &mut errors,
            "00200",
            TRANSMISSION,
            "DatiTrasmissione is required",
        );
    }

    if let Some(supplier) = header
        .cedente_prestatore
        .as_ref()
        .and_then(|party| party.dati_anagrafici.as_ref())
    {
        validate_anagrafica(supplier.anagrafica.as_ref(), SUPPLIER, &mut errors);
        if regime_fiscale_to_sdi(supplier.regime_fiscale).is_none() {
            report(
                &mut errors,
                "00200",
                format!("{SUPPLIER}/RegimeFiscale"),
                "RegimeFiscale is required",
            );
        }
        if let Some(id) = &supplier.id_fiscale_iva {
            errors.extend(
                validate_tax_id(id, "00301", &format!("{SUPPLIER}/IdFiscaleIVA"), false).err(),
            );
        } else {
            report(
                &mut errors,
                "00200",
                format!("{SUPPLIER}/IdFiscaleIVA"),
                "Supplier IdFiscaleIVA is required",
            );
        }
        if let Some(cf) = &supplier.codice_fiscale {
            validate_codice_fiscale(cf, "00302", SUPPLIER, &mut errors);
        }
    } else {
        report(
            &mut errors,
            "00200",
            SUPPLIER,
            "Supplier tax details are required",
        );
    }

    if let Some(customer) = customer {
        validate_anagrafica(customer.anagrafica.as_ref(), CUSTOMER, &mut errors);
        if let Some(id) = &customer.id_fiscale_iva {
            errors.extend(
                validate_tax_id(id, "00305", &format!("{CUSTOMER}/IdFiscaleIVA"), false).err(),
            );
        }
        if let Some(cf) = &customer.codice_fiscale {
            validate_codice_fiscale(cf, "00306", CUSTOMER, &mut errors);
        }
        if customer.id_fiscale_iva.is_none() && customer.codice_fiscale.is_none() {
            report(
                &mut errors,
                "00417",
                CUSTOMER,
                "Customer requires IdFiscaleIVA or CodiceFiscale",
            );
        }
    } else {
        report(
            &mut errors,
            "00200",
            CUSTOMER,
            "Customer tax details are required",
        );
    }

    if let Some(party) = &header.cedente_prestatore {
        validate_sede(
            party.sede.as_ref(),
            "Header/CedentePrestatore/Sede",
            &mut errors,
        );
    }
    if let Some(party) = &header.cessionario_committente {
        validate_sede(
            party.sede.as_ref(),
            "Header/CessionarioCommittente/Sede",
            &mut errors,
        );
    }

    if let Some(party) = header
        .rappresentante_fiscale
        .as_ref()
        .and_then(|party| party.dati_anagrafici.as_ref())
    {
        let path = "Header/RappresentanteFiscale/DatiAnagrafici";
        if let Some(id) = &party.id_fiscale_iva {
            errors
                .extend(validate_tax_id(id, "00303", &format!("{path}/IdFiscaleIVA"), false).err());
        }
        if let Some(cf) = &party.codice_fiscale {
            validate_codice_fiscale(cf, "00304", path, &mut errors);
        }
    }
    if let Some(id) = header
        .cessionario_committente
        .as_ref()
        .and_then(|party| party.rappresentante_fiscale.as_ref())
        .and_then(|party| party.id_fiscale_iva.as_ref())
    {
        errors.extend(
            validate_tax_id(
                id,
                "00303",
                "Header/CessionarioCommittente/RappresentanteFiscale/IdFiscaleIVA",
                false,
            )
            .err(),
        );
    }
    ValidationResult { errors }
}

fn valid_text(value: &str, max_length: usize) -> bool {
    (1..=max_length).contains(&value.chars().count())
        && value
            .chars()
            .all(|character| matches!(character, '\t' | '\n' | '\r' | '\u{20}'..='\u{ff}'))
}

fn validate_anagrafica(party: Option<&Anagrafica>, path: &str, errors: &mut Vec<ValidationError>) {
    let valid = match party.and_then(|party| party.soggetto.as_ref()) {
        Some(anagrafica::Soggetto::Denominazione(name)) => valid_text(name, 80),
        Some(anagrafica::Soggetto::PersonaFisica(person)) => {
            valid_text(&person.nome, 60) && valid_text(&person.cognome, 60)
        }
        None => false,
    };
    if !valid {
        report(
            errors,
            "00200",
            format!("{path}/Anagrafica"),
            "Anagrafica requires a valid company name or first and last name",
        );
    }
}

fn validate_sede(address: Option<&Indirizzo>, path: &str, errors: &mut Vec<ValidationError>) {
    let Some(address) = address else {
        report(errors, "00200", path, "Sede is required");
        return;
    };
    if !valid_text(&address.indirizzo, 60) {
        report(
            errors,
            "00200",
            format!("{path}/Indirizzo"),
            "Indirizzo must contain 1 to 60 Latin-1 characters",
        );
    }
    if !valid_text(&address.comune, 60) {
        report(
            errors,
            "00200",
            format!("{path}/Comune"),
            "Comune must contain 1 to 60 Latin-1 characters",
        );
    }
    if address.cap.len() != 5 || !address.cap.bytes().all(|byte| byte.is_ascii_digit()) {
        report(
            errors,
            "00200",
            format!("{path}/CAP"),
            "CAP must contain five digits",
        );
    }
    if address.nazione.len() != 2
        || !address
            .nazione
            .bytes()
            .all(|byte| byte.is_ascii_uppercase())
    {
        report(
            errors,
            "00200",
            format!("{path}/Nazione"),
            "Nazione must contain two uppercase letters",
        );
    }
}

/// Check the transmitter identifier before export or transmission.
pub fn validate_transmitter(
    transmission: Option<&DatiTrasmissione>,
) -> Result<(), ValidationError> {
    let path = format!("{TRANSMISSION}/IdTrasmittente");
    let Some(id) = transmission.and_then(|data| data.id_trasmittente.as_ref()) else {
        return Err(ValidationError {
            code: "00200".into(),
            message: "IdTrasmittente is required".into(),
            element_path: Some(path),
        });
    };
    if id.id_codice.trim().is_empty() && id.id_paese == "IT" {
        return Err(ValidationError {
            code: "00300".into(),
            message: "Il codice fiscale del trasmittente italiano è obbligatorio.".into(),
            element_path: Some(format!("{path}/IdCodice")),
        });
    }
    validate_tax_id(id, "00300", &path, true)
}

fn is_sdi_mailbox(address: &str) -> bool {
    let address = address.trim().to_ascii_lowercase();
    address.split_once('@').is_some_and(|(user, domain)| {
        domain == "pec.fatturapa.it"
            && user.strip_prefix("sdi").is_some_and(|suffix| {
                suffix.len() == 2 && suffix.bytes().all(|byte| byte.is_ascii_digit())
            })
    })
}

/// Validate a single body element of an invoice.
///
/// Checks Natura/AliquotaIVA coherence, line item price calculations,
/// riepilogo consistency, and payment IBAN formats.
pub fn validate_body(body: &FatturaElettronicaBody) -> ValidationResult {
    validate_body_indexed(body, 0)
}

// ===========================================================================
// Internal helpers
// ===========================================================================

fn validate_dati_generali(
    body: &FatturaElettronicaBody,
    path: &str,
    errors: &mut Vec<ValidationError>,
) {
    let Some(dg) = &body.dati_generali else {
        report(errors, "00200", path, "DatiGenerali is required");
        return;
    };
    let doc_path = format!("{path}/DatiGeneraliDocumento");
    let Some(doc) = &dg.dati_generali_documento else {
        report(
            errors,
            "00200",
            doc_path,
            "DatiGeneraliDocumento is required",
        );
        return;
    };

    if tipo_documento_to_sdi(doc.tipo_documento).is_none() {
        report(
            errors,
            "00200",
            format!("{doc_path}/TipoDocumento"),
            "TipoDocumento is unsupported",
        );
    }
    if !is_iso_date(&doc.data) || doc.data.as_str() < "1970-01-01" {
        report(
            errors,
            "00200",
            format!("{doc_path}/Data"),
            "Data must be a calendar date in YYYY-MM-DD format on or after 1970-01-01",
        );
    } else {
        for (index, linked) in dg.dati_fatture_collegate.iter().enumerate() {
            if let Some(date) = &linked.data {
                let linked_path = format!("{path}/DatiFattureCollegate[{}]/Data", index + 1);
                if !is_iso_date(date) {
                    report(
                        errors,
                        "00200",
                        linked_path,
                        "Linked invoice date must use YYYY-MM-DD",
                    );
                } else if doc.data < *date {
                    report(
                        errors,
                        "00418",
                        linked_path,
                        "Linked invoice date is later than the document date",
                    );
                }
            }
        }
    }
    if !(1..=20).contains(&doc.numero.chars().count()) {
        report(
            errors,
            "00200",
            format!("{doc_path}/Numero"),
            "Numero must contain 1 to 20 characters",
        );
    } else if !doc.numero.bytes().any(|byte| byte.is_ascii_digit()) {
        report(
            errors,
            "00425",
            format!("{doc_path}/Numero"),
            "Numero must contain at least one digit",
        );
    }
    if doc.divisa.len() != 3 || !doc.divisa.bytes().all(|byte| byte.is_ascii_uppercase()) {
        report(
            errors,
            "00200",
            format!("{doc_path}/Divisa"),
            "Divisa must contain three uppercase letters",
        );
    }

    validate_adjustments(&doc.sconto_maggiorazione, "00437", &doc_path, errors);
    for (index, withholding) in doc.dati_ritenuta.iter().enumerate() {
        let path = format!("{doc_path}/DatiRitenuta[{}]", index + 1);
        if tipo_ritenuta_to_sdi(withholding.tipo_ritenuta).is_none() {
            report(
                errors,
                "00200",
                format!("{path}/TipoRitenuta"),
                "TipoRitenuta is required",
            );
        }
        if causale_pagamento_to_sdi(withholding.causale_pagamento).is_none() {
            report(
                errors,
                "00200",
                format!("{path}/CausalePagamento"),
                "CausalePagamento is required",
            );
        }
        if withholding.importo_ritenuta <= 0.0 || withholding.aliquota_ritenuta <= 0.0 {
            report(
                errors,
                "EATTURA-RITENUTA",
                path,
                "DatiRitenuta requires a positive amount and rate",
            );
        }
    }
    if let Some(bollo) = &doc.dati_bollo {
        if bollo_virtuale_to_sdi(bollo.bollo_virtuale).is_none() {
            report(
                errors,
                "00200",
                format!("{doc_path}/DatiBollo/BolloVirtuale"),
                "BolloVirtuale is required",
            );
        }
        if bollo.importo_bollo.is_some_and(|amount| amount <= 0.0) {
            report(
                errors,
                "EATTURA-BOLLO",
                format!("{doc_path}/DatiBollo/ImportoBollo"),
                "ImportoBollo must be positive when present",
            );
        }
    }
}

/// Whether `s` is a syntactically valid ISO date `YYYY-MM-DD`.
fn is_iso_date(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let digits = |range: std::ops::Range<usize>| range.clone().all(|i| bytes[i].is_ascii_digit());
    if !(digits(0..4) && digits(5..7) && digits(8..10)) {
        return false;
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok()
}

#[derive(Debug, thiserror::Error)]
pub enum LineDescriptionError {
    #[error(
        "Descrizione contains an unsupported character {character:?} (U+{codepoint:04X}). Use Basic Latin or Latin-1 Supplement, with straight quotes and hyphens."
    )]
    UnsupportedCharacter { character: char, codepoint: u32 },
    #[error("Descrizione must contain 1 to 1000 characters and cannot be blank.")]
    InvalidLength,
}

/// Validate DettaglioLinee/Descrizione against the XSD String1000LatinType.
pub fn validate_line_description(description: &str) -> Result<(), LineDescriptionError> {
    let mut length = 0;
    for character in description.chars() {
        // Intersect Basic Latin and Latin-1 Supplement with XML 1.0 characters.
        if !matches!(character, '\t' | '\n' | '\r' | '\u{20}'..='\u{ff}') {
            return Err(LineDescriptionError::UnsupportedCharacter {
                character,
                codepoint: u32::from(character),
            });
        }
        length += 1;
    }
    if !(1..=1000).contains(&length) || description.trim().is_empty() {
        return Err(LineDescriptionError::InvalidLength);
    }
    Ok(())
}

fn validate_body_indexed(body: &FatturaElettronicaBody, body_idx: usize) -> ValidationResult {
    let mut errors = Vec::new();
    let path = format!("Body[{}]", body_idx + 1);
    validate_dati_generali(body, &format!("{path}/DatiGenerali"), &mut errors);
    validate_dati_beni_servizi(body, &path, &mut errors);
    errors.extend(validate_dati_documenti_correlati(body));

    for (payment_idx, payment) in body.dati_pagamento.iter().enumerate() {
        let payment_path = format!("{path}/DatiPagamento[{}]", payment_idx + 1);
        if condizioni_pagamento_to_sdi(payment.condizioni_pagamento).is_none() {
            report(
                &mut errors,
                "00200",
                format!("{payment_path}/CondizioniPagamento"),
                "CondizioniPagamento is required",
            );
        }
        if payment.dettaglio_pagamento.is_empty() {
            report(
                &mut errors,
                "00200",
                format!("{payment_path}/DettaglioPagamento"),
                "At least one payment detail is required",
            );
        }
        for (detail_idx, detail) in payment.dettaglio_pagamento.iter().enumerate() {
            let detail_path = format!("{payment_path}/DettaglioPagamento[{}]", detail_idx + 1);
            if modalita_pagamento_to_sdi(detail.modalita_pagamento).is_none() {
                report(
                    &mut errors,
                    "00200",
                    format!("{detail_path}/ModalitaPagamento"),
                    "ModalitaPagamento is required",
                );
            }
            if let Some(iban) = &detail.iban
                && !valid_iban(iban)
            {
                report(
                    &mut errors,
                    "00200",
                    format!("{detail_path}/IBAN"),
                    "IBAN must contain a two-letter country code, two digits and 11 to 30 alphanumeric characters",
                );
            }
        }
    }

    ValidationResult { errors }
}

fn validate_dati_beni_servizi(
    body: &FatturaElettronicaBody,
    path: &str,
    errors: &mut Vec<ValidationError>,
) {
    let goods_path = format!("{path}/DatiBeniServizi");
    let doc = body
        .dati_generali
        .as_ref()
        .and_then(|dg| dg.dati_generali_documento.as_ref());
    let td16 = doc.is_some_and(|doc| doc.tipo_documento == 160);
    let td21 = doc.is_some_and(|doc| doc.tipo_documento == 210);
    let has_withholding = doc.is_some_and(|doc| !doc.dati_ritenuta.is_empty());

    let Some(goods) = &body.dati_beni_servizi else {
        report(errors, "00200", goods_path, "DatiBeniServizi is required");
        return;
    };
    if goods.dettaglio_linee.is_empty() {
        report(
            errors,
            "00200",
            format!("{goods_path}/DettaglioLinee"),
            "At least one DettaglioLinee is required",
        );
    }
    if goods.dati_riepilogo.is_empty() {
        report(
            errors,
            "00200",
            format!("{goods_path}/DatiRiepilogo"),
            "At least one DatiRiepilogo is required",
        );
    }

    let mut summary_rates = BTreeSet::new();
    let mut summary_natures = BTreeSet::new();
    let mut totals_by_rate = BTreeMap::<String, f64>::new();
    for (index, summary) in goods.dati_riepilogo.iter().enumerate() {
        let summary_path = format!("{goods_path}/DatiRiepilogo[{}]", index + 1);
        validate_rate_and_nature(
            summary.aliquota_iva,
            summary.natura,
            ("00429", "00430"),
            td16,
            &summary_path,
            errors,
        );
        let rate = rate_key(summary.aliquota_iva);
        summary_rates.insert(rate.clone());
        if let Some(nature) = summary.natura.and_then(natura_to_sdi) {
            summary_natures.insert(nature);
            if summary.esigibilita_iva == Some(30) && nature.starts_with("N6") {
                report(
                    errors,
                    "00420",
                    format!("{summary_path}/EsigibilitaIVA"),
                    "Split payment is incompatible with reverse charge Natura N6",
                );
            }
        }
        let taxable = xml_amount2(summary.imponibile_importo);
        let tax_in_cents = xml_amount2(summary.aliquota_iva) * taxable;
        let expected_tax = (tax_in_cents + f64::EPSILON * tax_in_cents).round() / 100.0;
        if exceeds_tolerance(xml_amount2(summary.imposta), expected_tax, 0.01) {
            report(
                errors,
                "00421",
                format!("{summary_path}/Imposta"),
                format!(
                    "Imposta is {:.2}, expected {expected_tax:.2}",
                    summary.imposta
                ),
            );
        }
        *totals_by_rate.entry(rate).or_default() +=
            taxable - summary.arrotondamento.unwrap_or_default();
    }

    let mut taxable_by_rate = BTreeMap::new();
    for line in &goods.dettaglio_linee {
        let line_path = format!(
            "{goods_path}/DettaglioLinee[NumeroLinea={}]",
            line.numero_linea
        );
        if !(1..=9999).contains(&line.numero_linea) {
            report(
                errors,
                "00200",
                format!("{line_path}/NumeroLinea"),
                "NumeroLinea must be between 1 and 9999",
            );
        }
        if let Err(message) = validate_line_description(&line.descrizione) {
            report(
                errors,
                "00200",
                format!("{line_path}/Descrizione"),
                format!("Line {}: {message}", line.numero_linea),
            );
        }
        validate_rate_and_nature(
            line.aliquota_iva,
            line.natura,
            ("00400", "00401"),
            td16,
            &line_path,
            errors,
        );
        validate_adjustments(&line.sconto_maggiorazione, "00438", &line_path, errors);
        if !has_withholding && line.ritenuta == Some(10) {
            report(
                errors,
                "00411",
                format!("{line_path}/Ritenuta"),
                "Line withholding requires DatiRitenuta",
            );
        }
        if td21 && xml_amount2(line.aliquota_iva) == 0.0 {
            report(
                errors,
                "00474",
                format!("{line_path}/AliquotaIVA"),
                "TD21 does not allow a zero VAT rate on lines",
            );
        }
        let expected_price =
            apply_sconto_maggiorazione(line.prezzo_unitario, &line.sconto_maggiorazione)
                * line.quantita.unwrap_or(1.0);
        if exceeds_tolerance(line.prezzo_totale, expected_price, 0.01) {
            report(
                errors,
                "00423",
                format!("{line_path}/PrezzoTotale"),
                format!(
                    "PrezzoTotale is {}, expected {expected_price:.8}",
                    line.prezzo_totale
                ),
            );
        }
        let rate = rate_key(line.aliquota_iva);
        validate_summary_match(
            &rate,
            line.natura,
            &summary_rates,
            &summary_natures,
            &line_path,
            errors,
        );
        *taxable_by_rate.entry(rate).or_default() += line.prezzo_totale;
    }

    if let Some(doc) = doc {
        for (index, cassa) in doc.dati_cassa_previdenziale.iter().enumerate() {
            let cassa_path = format!(
                "{path}/DatiGenerali/DatiGeneraliDocumento/DatiCassaPrevidenziale[{}]",
                index + 1
            );
            if tipo_cassa_to_sdi(cassa.tipo_cassa).is_none() {
                report(
                    errors,
                    "00200",
                    format!("{cassa_path}/TipoCassa"),
                    "TipoCassa is required",
                );
            }
            validate_rate_and_nature(
                cassa.aliquota_iva,
                cassa.natura,
                ("00413", "00414"),
                false,
                &cassa_path,
                errors,
            );
            if !has_withholding && cassa.ritenuta == Some(10) {
                report(
                    errors,
                    "00415",
                    format!("{cassa_path}/Ritenuta"),
                    "Pension contribution withholding requires DatiRitenuta",
                );
            }
            let rate = rate_key(cassa.aliquota_iva);
            validate_summary_match(
                &rate,
                cassa.natura,
                &summary_rates,
                &summary_natures,
                &cassa_path,
                errors,
            );
            *taxable_by_rate.entry(rate).or_default() +=
                xml_amount2(cassa.importo_contributo_cassa);
        }
    }

    if goods.dati_riepilogo.len() < taxable_by_rate.len() {
        report(
            errors,
            "00419",
            format!("{goods_path}/DatiRiepilogo"),
            "At least one DatiRiepilogo is required for each distinct VAT rate",
        );
    }
    for (rate, expected) in taxable_by_rate {
        if let Some(&actual) = totals_by_rate.get(&rate)
            && exceeds_tolerance(actual, expected, 1.0)
        {
            report(
                errors,
                "00422",
                format!("{goods_path}/DatiRiepilogo[AliquotaIVA={rate}]/ImponibileImporto"),
                format!(
                    "Taxable amount net of rounding is {actual:.2}, expected {expected:.2} at VAT rate {rate}"
                ),
            );
        }
    }
}

// The XML writer serializes VAT rates and two-decimal amounts with this precision.
fn xml_amount2(value: f64) -> f64 {
    format!("{value:.2}").parse().unwrap_or(value)
}

fn rate_key(rate: f64) -> String {
    format!("{:.2}", rate + 0.0)
}

fn exceeds_tolerance(actual: f64, expected: f64, tolerance: f64) -> bool {
    if !actual.is_finite() || !expected.is_finite() {
        return true;
    }
    // Account for representation error at the inclusive monetary boundary.
    let float_error = 2.0 * f64::EPSILON * actual.abs().max(expected.abs());
    (actual - expected).abs() > tolerance + float_error
}

fn validate_rate_and_nature(
    rate: f64,
    nature: Option<i32>,
    (missing_code, unexpected_code): (&str, &str),
    allow_taxable_nature: bool,
    path: &str,
    errors: &mut Vec<ValidationError>,
) {
    let rate = xml_amount2(rate + 0.0);
    let nature_code = nature.and_then(natura_to_sdi);
    if !(0.0..=100.0).contains(&rate) {
        report(
            errors,
            "00200",
            format!("{path}/AliquotaIVA"),
            "AliquotaIVA must be between 0 and 100",
        );
    } else if rate != 0.0 && rate < 1.0 {
        report(
            errors,
            "00424",
            format!("{path}/AliquotaIVA"),
            "AliquotaIVA must be expressed as a percentage",
        );
    }
    if nature.is_some_and(|value| value != 0) && nature_code.is_none() {
        report(errors, "00200", format!("{path}/Natura"), "Unknown Natura");
    }
    if matches!(nature_code, Some("N2" | "N3" | "N6")) {
        report(
            errors,
            "00445",
            format!("{path}/Natura"),
            "Use a specific Natura subcode",
        );
    }
    if rate == 0.0 && nature_code.is_none() {
        report(
            errors,
            missing_code,
            format!("{path}/Natura"),
            "Natura is required when AliquotaIVA is zero",
        );
    } else if rate != 0.0 && nature_code.is_some() && !allow_taxable_nature {
        report(
            errors,
            unexpected_code,
            format!("{path}/Natura"),
            "Natura is not allowed with a nonzero AliquotaIVA",
        );
    }
}

fn validate_adjustments(
    adjustments: &[ScontoMaggiorazione],
    code: &str,
    path: &str,
    errors: &mut Vec<ValidationError>,
) {
    for (index, adjustment) in adjustments.iter().enumerate() {
        let path = format!("{path}/ScontoMaggiorazione[{}]", index + 1);
        if tipo_sconto_maggiorazione_to_sdi(adjustment.tipo).is_none() {
            report(
                errors,
                "00200",
                format!("{path}/Tipo"),
                "ScontoMaggiorazione requires SC or MG",
            );
        } else if adjustment.percentuale.is_none() && adjustment.importo.is_none() {
            report(
                errors,
                code,
                path,
                "ScontoMaggiorazione requires Percentuale or Importo",
            );
        }
    }
}

fn validate_summary_match(
    rate: &str,
    nature: Option<i32>,
    summary_rates: &BTreeSet<String>,
    summary_natures: &BTreeSet<&str>,
    path: &str,
    errors: &mut Vec<ValidationError>,
) {
    if !summary_rates.contains(rate) {
        report(
            errors,
            "00443",
            format!("{path}/AliquotaIVA"),
            format!("No DatiRiepilogo for AliquotaIVA {rate}"),
        );
    }
    if let Some(nature) = nature.and_then(natura_to_sdi)
        && !summary_natures.contains(nature)
    {
        report(
            errors,
            "00444",
            format!("{path}/Natura"),
            format!("No DatiRiepilogo for Natura {nature}"),
        );
    }
}

/// Apply discount/surcharge adjustments to a unit price before multiplying by quantity.
pub fn apply_sconto_maggiorazione(base: f64, adjustments: &[ScontoMaggiorazione]) -> f64 {
    let mut result = base;
    for sm in adjustments {
        let is_sconto = tipo_sconto_maggiorazione_to_sdi(sm.tipo) == Some("SC");
        let adjustment = if let Some(imp) = sm.importo {
            imp
        } else if let Some(pct) = sm.percentuale {
            result * pct / 100.0
        } else {
            continue;
        };

        if is_sconto {
            result -= adjustment;
        } else {
            result += adjustment;
        }
    }
    result
}

fn validate_batch(invoice: &FatturaElettronica, errors: &mut Vec<ValidationError>) {
    let mut seen = HashMap::<_, (bool, BTreeSet<&str>)>::new();
    for (index, body) in invoice.body.iter().enumerate() {
        let Some(doc) = body
            .dati_generali
            .as_ref()
            .and_then(|dg| dg.dati_generali_documento.as_ref())
        else {
            continue;
        };
        if !is_iso_date(&doc.data) || doc.numero.is_empty() {
            continue;
        }
        let key = (
            &doc.data[..4],
            doc.numero.as_str(),
            doc.tipo_documento == 40,
            uses_customer_numbering(doc.tipo_documento),
        );
        let (has_standard_numbering, dates) = seen.entry(key).or_default();
        let repeated_date = !dates.insert(&doc.data);
        let article_73 = doc.art73 == Some(10);
        if repeated_date || *has_standard_numbering || (!article_73 && dates.len() > 1) {
            report(
                errors,
                "00409",
                format!(
                    "Body[{}]/DatiGenerali/DatiGeneraliDocumento/Numero",
                    index + 1
                ),
                format!("Duplicate invoice number {} in the same batch", doc.numero),
            );
        }
        *has_standard_numbering |= !article_73;
    }
}

fn uses_customer_numbering(kind: i32) -> bool {
    matches!(kind, 160 | 170 | 180 | 190 | 200 | 220 | 230 | 280)
}

fn validate_tipo_documento_rules(invoice: &FatturaElettronica, errors: &mut Vec<ValidationError>) {
    let Some(header) = &invoice.header else {
        return;
    };
    let supplier = header
        .cedente_prestatore
        .as_ref()
        .and_then(|party| party.dati_anagrafici.as_ref())
        .and_then(|party| party.id_fiscale_iva.as_ref());
    let customer = header
        .cessionario_committente
        .as_ref()
        .and_then(|party| party.dati_anagrafici.as_ref())
        .and_then(|party| party.id_fiscale_iva.as_ref());
    if let (Some(supplier), Some(customer)) = (supplier, customer)
        && [&supplier.id_paese, &customer.id_paese]
            .iter()
            .all(|country| !country.is_empty() && *country != "IT")
    {
        report(
            errors,
            "00476",
            format!("{CUSTOMER}/IdFiscaleIVA/IdPaese"),
            "Supplier and customer cannot both have foreign VAT identifiers",
        );
    }

    for (index, body) in invoice.body.iter().enumerate() {
        let Some(doc) = body
            .dati_generali
            .as_ref()
            .and_then(|dg| dg.dati_generali_documento.as_ref())
        else {
            continue;
        };
        let kind = doc.tipo_documento;
        let path = format!(
            "Body[{}]/DatiGenerali/DatiGeneraliDocumento/TipoDocumento",
            index + 1
        );
        if let (Some(supplier), Some(customer)) = (supplier, customer) {
            if matches!(
                kind,
                10 | 20 | 30 | 60 | 160 | 170 | 180 | 190 | 200 | 240 | 250 | 280
            ) && supplier == customer
            {
                report(
                    errors,
                    "00471",
                    &path,
                    "This document type requires different supplier and customer VAT identifiers",
                );
            }
            if matches!(kind, 210 | 270) && supplier != customer {
                report(
                    errors,
                    "00472",
                    &path,
                    "This document type requires the same supplier and customer VAT identifier",
                );
            }
        }
        if matches!(kind, 170 | 180 | 190 | 280) && supplier.is_some_and(|id| id.id_paese == "IT") {
            report(
                errors,
                "00473",
                &path,
                "This document type requires a foreign supplier VAT identifier",
            );
        }
        if uses_customer_numbering(kind) && customer.is_none() {
            report(
                errors,
                "00475",
                &path,
                "This document type requires customer IdFiscaleIVA",
            );
        }
    }
}

fn validate_tax_id(
    id: &IdFiscale,
    code: &str,
    path: &str,
    allow_personal_id: bool,
) -> Result<(), ValidationError> {
    let (code, field, message) =
        if id.id_paese.len() != 2 || !id.id_paese.bytes().all(|byte| byte.is_ascii_uppercase()) {
            (
                "00200",
                "IdPaese",
                "IdPaese must contain two uppercase letters",
            )
        } else if id.id_codice.trim().is_empty() || id.id_codice.chars().count() > 28 {
            (
                "00200",
                "IdCodice",
                "IdCodice must contain 1 to 28 characters",
            )
        } else if id.id_paese == "IT"
            && !(valid_numeric_tax_id(&id.id_codice)
                || allow_personal_id && valid_personal_tax_id(&id.id_codice))
        {
            (
                code,
                "IdCodice",
                "Italian tax identifier has invalid structure or check digit",
            )
        } else {
            return Ok(());
        };
    Err(ValidationError {
        code: code.into(),
        message: message.into(),
        element_path: Some(format!("{path}/{field}")),
    })
}

fn validate_codice_fiscale(value: &str, code: &str, path: &str, errors: &mut Vec<ValidationError>) {
    let path = format!("{path}/CodiceFiscale");
    if !(11..=16).contains(&value.len())
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
    {
        report(
            errors,
            "00200",
            path,
            "CodiceFiscale must contain 11 to 16 uppercase alphanumeric characters",
        );
    } else if !valid_numeric_tax_id(value) && !valid_personal_tax_id(value) {
        report(
            errors,
            code,
            path,
            "CodiceFiscale has invalid structure or check digit",
        );
    }
}

fn valid_numeric_tax_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 11 || !bytes.iter().all(u8::is_ascii_digit) || value == "00000000000" {
        return false;
    }
    let sum: u8 = bytes[..10]
        .iter()
        .enumerate()
        .map(|(index, byte)| {
            let digit = byte - b'0';
            if index % 2 == 0 {
                digit
            } else {
                (digit * 2) % 10 + (digit * 2) / 10
            }
        })
        .sum();
    (10 - sum % 10) % 10 == bytes[10] - b'0'
}

fn valid_personal_tax_id(value: &str) -> bool {
    const ODD_DIGITS: [u8; 10] = [1, 0, 5, 7, 9, 13, 15, 17, 19, 21];
    const ODD_LETTERS: [u8; 26] = [
        1, 0, 5, 7, 9, 13, 15, 17, 19, 21, 2, 4, 18, 20, 11, 3, 6, 8, 12, 14, 16, 10, 22, 25, 24,
        23,
    ];
    let bytes = value.as_bytes();
    if bytes.len() != 16 {
        return false;
    }
    for (index, &byte) in bytes.iter().enumerate() {
        let valid = match index {
            6 | 7 | 9 | 10 | 12 | 13 | 14 => byte.is_ascii_digit() || b"LMNPQRSTUV".contains(&byte),
            8 => b"ABCDEHLMPRST".contains(&byte),
            _ => byte.is_ascii_uppercase(),
        };
        if !valid {
            return false;
        }
    }
    let sum: u16 = bytes[..15]
        .iter()
        .enumerate()
        .map(|(index, &byte)| {
            let points = match (index % 2 == 0, byte.is_ascii_digit()) {
                (true, true) => ODD_DIGITS[usize::from(byte - b'0')],
                (true, false) => ODD_LETTERS[usize::from(byte - b'A')],
                (false, true) => byte - b'0',
                (false, false) => byte - b'A',
            };
            u16::from(points)
        })
        .sum();
    sum % 26 == u16::from(bytes[15] - b'A')
}

fn valid_iban(iban: &str) -> bool {
    let bytes = iban.as_bytes();
    iban.is_ascii()
        && (15..=34).contains(&bytes.len())
        && bytes[..2].iter().all(u8::is_ascii_alphabetic)
        && bytes[2..4].iter().all(u8::is_ascii_digit)
        && bytes[4..].iter().all(u8::is_ascii_alphanumeric)
}

fn validate_dati_documenti_correlati(body: &FatturaElettronicaBody) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let Some(dg) = &body.dati_generali else {
        return errors;
    };

    let mut all_docs = Vec::new();
    all_docs.extend(
        dg.dati_ordine_acquisto
            .iter()
            .map(|d| ("DatiOrdineAcquisto", d)),
    );
    all_docs.extend(dg.dati_contratto.iter().map(|d| ("DatiContratto", d)));
    all_docs.extend(dg.dati_convenzione.iter().map(|d| ("DatiConvenzione", d)));
    all_docs.extend(
        dg.dati_fatture_collegate
            .iter()
            .map(|d| ("DatiFattureCollegate", d)),
    );

    for (tag, doc) in all_docs {
        if let Some(cig) = &doc.codice_cig {
            let trimmed = cig.trim();
            if !trimmed.is_empty()
                && (trimmed.len() != 10 || !trimmed.chars().all(|c| c.is_ascii_alphanumeric()))
            {
                errors.push(ValidationError {
                    code: "EATTURA-CIG".into(),
                    message: format!(
                        "Codice CIG must be 10 alphanumeric characters, got '{}'",
                        trimmed
                    ),
                    element_path: Some(format!("Body/DatiGenerali/{}/CodiceCIG", tag)),
                });
            }
        }
        if let Some(cup) = &doc.codice_cup {
            let trimmed = cup.trim();
            if !trimmed.is_empty()
                && (trimmed.len() != 15 || !trimmed.chars().all(|c| c.is_ascii_alphanumeric()))
            {
                errors.push(ValidationError {
                    code: "EATTURA-CUP".into(),
                    message: format!(
                        "Codice CUP must be 15 alphanumeric characters, got '{}'",
                        trimmed
                    ),
                    element_path: Some(format!("Body/DatiGenerali/{}/CodiceCUP", tag)),
                });
            }
        }
    }

    errors
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdi::v1::*;

    /// Build a minimal valid invoice for testing.
    pub(super) fn minimal_invoice() -> FatturaElettronica {
        FatturaElettronica {
            versione: 20, // FPR12
            sistema_emittente: None,
            header: Some(FatturaElettronicaHeader {
                dati_trasmissione: Some(DatiTrasmissione {
                    id_trasmittente: Some(IdFiscale {
                        id_paese: "IT".into(),
                        id_codice: "01234567897".into(),
                    }),
                    progressivo_invio: "00001".into(),
                    formato_trasmissione: 20,
                    codice_destinatario: "0000000".into(),
                    ..Default::default()
                }),
                cedente_prestatore: Some(CedentePrestatore {
                    dati_anagrafici: Some(DatiAnagraficiCedente {
                        id_fiscale_iva: Some(IdFiscale {
                            id_paese: "IT".into(),
                            id_codice: "01234567897".into(),
                        }),
                        anagrafica: Some(Anagrafica {
                            soggetto: Some(anagrafica::Soggetto::Denominazione("Test SRL".into())),
                            ..Default::default()
                        }),
                        regime_fiscale: 10, // RF01
                        ..Default::default()
                    }),
                    sede: Some(Indirizzo {
                        indirizzo: "Via Test 1".into(),
                        cap: "00100".into(),
                        comune: "Roma".into(),
                        nazione: "IT".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                cessionario_committente: Some(CessionarioCommittente {
                    dati_anagrafici: Some(DatiAnagraficiCessionario {
                        id_fiscale_iva: Some(IdFiscale {
                            id_paese: "IT".into(),
                            id_codice: "09876543217".into(),
                        }),
                        anagrafica: Some(Anagrafica {
                            soggetto: Some(anagrafica::Soggetto::Denominazione(
                                "Cliente SRL".into(),
                            )),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                    sede: Some(Indirizzo {
                        indirizzo: "Via Cliente 2".into(),
                        cap: "20100".into(),
                        comune: "Milano".into(),
                        nazione: "IT".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            body: vec![FatturaElettronicaBody {
                dati_generali: Some(DatiGenerali {
                    dati_generali_documento: Some(DatiGeneraliDocumento {
                        tipo_documento: 10, // TD01
                        divisa: "EUR".into(),
                        data: "2024-01-15".into(),
                        numero: "1".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                dati_beni_servizi: Some(DatiBeniServizi {
                    dettaglio_linee: vec![DettaglioLinee {
                        numero_linea: 1,
                        descrizione: "Test service".into(),
                        quantita: Some(1.0),
                        prezzo_unitario: 100.0,
                        prezzo_totale: 100.0,
                        aliquota_iva: 22.0,
                        ..Default::default()
                    }],
                    dati_riepilogo: vec![DatiRiepilogo {
                        aliquota_iva: 22.0,
                        imponibile_importo: 100.0,
                        imposta: 22.0,
                        ..Default::default()
                    }],
                }),
                ..Default::default()
            }],
        }
    }

    #[test]
    fn test_valid_invoice_passes() {
        let inv = minimal_invoice();
        let result = validate(&inv);
        assert!(
            result.is_valid(),
            "Expected valid, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn rejects_non_sdi_line_descriptions_with_the_line_number() {
        let mut invoice = minimal_invoice();
        let description = |invoice: &mut FatturaElettronica, value: String| {
            invoice.body[0]
                .dati_beni_servizi
                .as_mut()
                .unwrap()
                .dettaglio_linee[0]
                .descrizione = value;
        };
        for (value, codepoint) in [
            ("Aggiornamento dell’immagine", "U+2019"),
            ("Supporto “cloud”", "U+201C"),
            ("Servizi – ottobre", "U+2013"),
            ("Tariffa in €", "U+20AC"),
            ("Supporto 🚀", "U+1F680"),
            ("Invalid\u{0000} XML", "U+0000"),
        ] {
            description(&mut invoice, value.into());
            let result = validate(&invoice);
            assert_eq!(result.errors.len(), 1);
            let error = &result.errors[0];
            assert_eq!(error.code, "00200");
            assert!(error.message.contains(codepoint));
            assert_eq!(
                error.element_path.as_deref(),
                Some("Body[1]/DatiBeniServizi/DettaglioLinee[NumeroLinea=1]/Descrizione")
            );
            let mut output = Vec::new();
            assert!(crate::xml::encode::encode_to_writer(&invoice, &mut output).is_err());
            assert!(output.is_empty());
        }
        for value in [String::new(), " ".into(), "è".repeat(1001)] {
            description(&mut invoice, value);
            assert!(!validate(&invoice).is_valid());
        }
        for value in [
            "Qualità dell'immagine - supporto & rilascio".into(),
            "è".repeat(1000),
        ] {
            description(&mut invoice, value);
            assert!(validate(&invoice).is_valid());
            assert!(crate::xml::encode::encode(&invoice).is_ok());
        }
    }

    #[test]
    fn test_sdi_00400_natura_required_when_aliquota_zero() {
        let mut inv = minimal_invoice();
        let body = &mut inv.body[0];
        let dbs = body.dati_beni_servizi.as_mut().unwrap();
        dbs.dettaglio_linee[0].aliquota_iva = 0.0;
        dbs.dettaglio_linee[0].natura = None;
        dbs.dati_riepilogo[0].aliquota_iva = 0.0;
        dbs.dati_riepilogo[0].imposta = 0.0;
        dbs.dati_riepilogo[0].natura = None;

        let result = validate(&inv);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "00400"));
    }

    #[test]
    fn test_sdi_00401_natura_forbidden_when_aliquota_nonzero() {
        let mut inv = minimal_invoice();
        let body = &mut inv.body[0];
        let dbs = body.dati_beni_servizi.as_mut().unwrap();
        dbs.dettaglio_linee[0].natura = Some(10); // N1
        dbs.dati_riepilogo[0].natura = Some(10);

        let result = validate(&inv);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "00401"));
    }

    #[test]
    fn test_sdi_00421_imposta_calculation() {
        let mut inv = minimal_invoice();
        let body = &mut inv.body[0];
        let dbs = body.dati_beni_servizi.as_mut().unwrap();
        // 22% of 100 is 22.0, set it to 25.0 to trigger 00421
        dbs.dati_riepilogo[0].imposta = 25.0;

        let result = validate(&inv);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "00421"));
    }

    #[test]
    fn test_sdi_00423_prezzo_totale() {
        let mut inv = minimal_invoice();
        let body = &mut inv.body[0];
        let dbs = body.dati_beni_servizi.as_mut().unwrap();
        // 1 * 100 != 99.0
        dbs.dettaglio_linee[0].prezzo_totale = 99.0;

        let result = validate(&inv);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "00423"));
    }

    #[test]
    fn test_codice_fiscale_valid() {
        assert!(valid_personal_tax_id("RSSMRA80A01H501U"));
        assert!(valid_numeric_tax_id("01234567897"));
    }

    #[test]
    fn test_codice_fiscale_invalid() {
        assert!(!valid_personal_tax_id("SHORT"));
        assert!(!valid_personal_tax_id("RSSMRA80A01H501V"));
    }

    #[test]
    fn test_iban_valid() {
        assert!(valid_iban("IT60X0542811101000000123456"));
    }

    #[test]
    fn test_iban_invalid() {
        assert!(!valid_iban("SHORT"));
        assert!(!valid_iban("123456789012345")); // starts with digits
    }

    #[test]
    fn test_is_iso_date() {
        assert!(is_iso_date("2026-06-16"));
        assert!(is_iso_date("2026-12-31"));
        assert!(!is_iso_date("2026-13-01")); // bad month
        assert!(!is_iso_date("2026-6-16")); // not zero-padded
        assert!(!is_iso_date("16/06/2026")); // wrong separator
        assert!(!is_iso_date(""));
    }

    #[test]
    fn test_fpa12_and_fpr12_codice_destinatario() {
        let mut inv = minimal_invoice();
        inv.header
            .as_mut()
            .unwrap()
            .dati_trasmissione
            .as_mut()
            .unwrap()
            .formato_trasmissione = 10; // FPA12
        inv.header
            .as_mut()
            .unwrap()
            .dati_trasmissione
            .as_mut()
            .unwrap()
            .codice_destinatario = "UF1234".into(); // 6 chars
        inv.versione = 10;
        assert!(validate(&inv).is_valid());

        // Seven characters on FPA12 trigger 00427.
        inv.header
            .as_mut()
            .unwrap()
            .dati_trasmissione
            .as_mut()
            .unwrap()
            .codice_destinatario = "0000000".into();
        let res = validate(&inv);
        assert!(!res.is_valid());
        assert!(res.errors.iter().any(|e| e.code == "00427"));

        // Seven characters on FPR12 pass.
        inv.versione = 20;
        inv.header
            .as_mut()
            .unwrap()
            .dati_trasmissione
            .as_mut()
            .unwrap()
            .formato_trasmissione = 20;
        inv.header
            .as_mut()
            .unwrap()
            .dati_trasmissione
            .as_mut()
            .unwrap()
            .codice_destinatario = "M5UXCR1".into();
        assert!(validate(&inv).is_valid());
    }

    #[test]
    fn test_split_payment_sdi_00420() {
        let mut inv = minimal_invoice();
        let dbs = inv.body[0].dati_beni_servizi.as_mut().unwrap();
        dbs.dati_riepilogo[0].esigibilita_iva = Some(30); // S = Split Payment
        dbs.dati_riepilogo[0].natura = Some(290); // N6.1 = Reverse charge
        let res = validate(&inv);
        assert!(!res.is_valid());
        assert!(res.errors.iter().any(|e| e.code == "00420"));
    }

    #[test]
    fn test_cig_cup_validation() {
        let mut inv = minimal_invoice();
        inv.body[0]
            .dati_generali
            .as_mut()
            .unwrap()
            .dati_ordine_acquisto
            .push(DatiDocumentiCorrelati {
                id_documento: "ORD-2026-1".into(),
                codice_cig: Some("1234567890".into()), // 10 chars -> ok
                codice_cup: Some("123456789012345".into()), // 15 chars -> ok
                ..Default::default()
            });
        assert!(validate(&inv).is_valid());

        // Invalid CIG length triggers the local CIG check.
        inv.body[0]
            .dati_generali
            .as_mut()
            .unwrap()
            .dati_ordine_acquisto[0]
            .codice_cig = Some("SHORT".into());
        let res = validate(&inv);
        assert!(!res.is_valid());
        assert!(res.errors.iter().any(|e| e.code == "EATTURA-CIG"));
    }

    #[test]
    fn test_multiple_natura_codes_and_cassa_in_riepilogo() {
        let mut inv = minimal_invoice();
        let body = &mut inv.body[0];
        body.dati_generali
            .as_mut()
            .unwrap()
            .dati_generali_documento
            .as_mut()
            .unwrap()
            .dati_cassa_previdenziale
            .push(DatiCassaPrevidenziale {
                tipo_cassa: 40, // TC04
                al_cassa: 4.0,
                importo_contributo_cassa: 4.0,
                imponibile_cassa: Some(100.0),
                aliquota_iva: 22.0,
                ..Default::default()
            });
        let dbs = body.dati_beni_servizi.as_mut().unwrap();
        // Line 1: 100 EUR @ 22% + 4 EUR cassa @ 22% = 104 EUR imponibile
        dbs.dati_riepilogo[0].imponibile_importo = 104.0;
        dbs.dati_riepilogo[0].imposta = 22.88;

        // Add two 0% lines with different Natura codes (N1 and N2.2)
        dbs.dettaglio_linee.push(DettaglioLinee {
            numero_linea: 2,
            descrizione: "Riaddebito bollo".into(),
            quantita: Some(1.0),
            prezzo_unitario: 2.0,
            prezzo_totale: 2.0,
            aliquota_iva: 0.0,
            natura: Some(10), // N1
            ..Default::default()
        });
        dbs.dettaglio_linee.push(DettaglioLinee {
            numero_linea: 3,
            descrizione: "Operazione non soggetta".into(),
            quantita: Some(1.0),
            prezzo_unitario: 50.0,
            prezzo_totale: 50.0,
            aliquota_iva: 0.0,
            natura: Some(70), // N2.2
            ..Default::default()
        });
        dbs.dati_riepilogo.push(DatiRiepilogo {
            aliquota_iva: 0.0,
            imponibile_importo: 2.0,
            imposta: 0.0,
            natura: Some(10), // N1
            ..Default::default()
        });
        dbs.dati_riepilogo.push(DatiRiepilogo {
            aliquota_iva: 0.0,
            imponibile_importo: 50.0,
            imposta: 0.0,
            natura: Some(70), // N2.2
            ..Default::default()
        });

        let res = validate(&inv);
        assert!(res.is_valid(), "Expected valid, got: {:?}", res.errors);
    }
}

#[cfg(test)]
mod sdi_tests;
