use std::collections::HashMap;

use crate::sdi::v1::{
    DettaglioLinee, FatturaElettronica, FatturaElettronicaBody, FatturaElettronicaHeader,
    ScontoMaggiorazione,
};
use crate::xml::enum_map::{natura_to_sdi, tipo_sconto_maggiorazione_to_sdi};
use super::{ValidationError, ValidationResult};

/// Rounding tolerance for 2-decimal financial amounts (SDI 00421/00422).
const TOLERANCE_2: f64 = 0.01;

/// Rounding tolerance for 8-decimal price calculations (SDI 00423).
const TOLERANCE_8: f64 = 1e-4;

// ===========================================================================
// Public API
// ===========================================================================

/// Validate a `FatturaElettronica` against all SDI rules.
///
/// Runs header validation, body validation for each body element,
/// and cross-cutting checks (e.g. document-type-specific rules).
/// Returns a combined `ValidationResult` with all errors found.
pub fn validate(invoice: &FatturaElettronica) -> ValidationResult {
    let mut errors = Vec::new();

    if let Some(header) = &invoice.header {
        let r = validate_header(header);
        errors.extend(r.errors);
    }

    for (i, body) in invoice.body.iter().enumerate() {
        let r = validate_body_indexed(body, i);
        errors.extend(r.errors);
    }

    errors.extend(validate_tipo_documento_rules(invoice));

    ValidationResult { errors }
}

/// Validate only the header portion of an invoice.
///
/// Checks transmission data fields, cedente/prestatore completeness,
/// cessionario/committente required fields, and format validators for
/// codice fiscale, partita IVA, and codice destinatario.
pub fn validate_header(header: &FatturaElettronicaHeader) -> ValidationResult {
    let mut errors = Vec::new();

    // Validate DatiTrasmissione
    if let Some(dt) = &header.dati_trasmissione {
        if let Some(id) = &dt.id_trasmittente {
            errors.extend(validate_id_fiscale(&id.id_paese, &id.id_codice));
        }
    }

    // Validate CedentePrestatore
    if let Some(cp) = &header.cedente_prestatore {
        if let Some(da) = &cp.dati_anagrafici {
            if let Some(id) = &da.id_fiscale_iva {
                errors.extend(validate_id_fiscale(&id.id_paese, &id.id_codice));
            }
            if let Some(cf) = &da.codice_fiscale {
                if let Some(e) = validate_codice_fiscale(cf) {
                    errors.push(e);
                }
            }
        }
    }

    // Validate CessionarioCommittente
    if let Some(cc) = &header.cessionario_committente {
        if let Some(da) = &cc.dati_anagrafici {
            if let Some(id) = &da.id_fiscale_iva {
                errors.extend(validate_id_fiscale(&id.id_paese, &id.id_codice));
            }
            if let Some(cf) = &da.codice_fiscale {
                if let Some(e) = validate_codice_fiscale(cf) {
                    errors.push(e);
                }
            }
            // 00417: the cessionario must be identified by IdFiscaleIVA or CodiceFiscale.
            if da.id_fiscale_iva.is_none() && da.codice_fiscale.is_none() {
                errors.push(ValidationError {
                    code: "00417".into(),
                    message: "CessionarioCommittente requires either IdFiscaleIVA or CodiceFiscale".into(),
                    element_path: Some("Header/CessionarioCommittente/DatiAnagrafici".into()),
                });
            }
        }
    }

    // 00305: CodiceDestinatario must be 7 chars (private) or 6 chars (PA).
    if let Some(dt) = &header.dati_trasmissione {
        let cd = &dt.codice_destinatario;
        if !cd.is_empty() && cd.len() != 7 && cd.len() != 6 {
            errors.push(ValidationError {
                code: "00305".into(),
                message: format!(
                    "CodiceDestinatario must be 6 (PA) or 7 (private) characters, got {}",
                    cd.len()
                ),
                element_path: Some("Header/DatiTrasmissione/CodiceDestinatario".into()),
            });
        }
    }

    ValidationResult { errors }
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

/// Validate document-level general data: date format and ritenuta/bollo coherence.
///
/// The date-format and amount-coherence checks are not single official SDI
/// codes (the SDI XSD enforces the date type structurally), so they carry
/// `EATTURA-*` codes to distinguish them from canonical SDI controls.
fn validate_dati_generali(body: &FatturaElettronicaBody) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let Some(dgd) = body
        .dati_generali
        .as_ref()
        .and_then(|dg| dg.dati_generali_documento.as_ref())
    else {
        return errors;
    };

    // Date must be present and in ISO `YYYY-MM-DD` form.
    if !is_iso_date(&dgd.data) {
        errors.push(ValidationError {
            code: "EATTURA-DATE".into(),
            message: format!("Data must be in YYYY-MM-DD format, got '{}'", dgd.data),
            element_path: Some("Body/DatiGenerali/DatiGeneraliDocumento/Data".into()),
        });
    }

    // Ritenuta: when present, importo and aliquota must be positive.
    for (i, rit) in dgd.dati_ritenuta.iter().enumerate() {
        if rit.importo_ritenuta <= 0.0 || rit.aliquota_ritenuta <= 0.0 {
            errors.push(ValidationError {
                code: "EATTURA-RITENUTA".into(),
                message: "DatiRitenuta requires a positive ImportoRitenuta and AliquotaRitenuta".into(),
                element_path: Some(format!(
                    "Body/DatiGenerali/DatiGeneraliDocumento/DatiRitenuta[{}]",
                    i + 1
                )),
            });
        }
    }

    // Bollo: when present, ImportoBollo must be positive.
    if let Some(bollo) = &dgd.dati_bollo {
        if bollo.importo_bollo.unwrap_or(0.0) <= 0.0 {
            errors.push(ValidationError {
                code: "EATTURA-BOLLO".into(),
                message: "DatiBollo requires a positive ImportoBollo".into(),
                element_path: Some("Body/DatiGenerali/DatiGeneraliDocumento/DatiBollo/ImportoBollo".into()),
            });
        }
    }

    errors
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
    let month: u32 = s[5..7].parse().unwrap_or(0);
    let day: u32 = s[8..10].parse().unwrap_or(0);
    (1..=12).contains(&month) && (1..=31).contains(&day)
}

fn validate_body_indexed(body: &FatturaElettronicaBody, _body_idx: usize) -> ValidationResult {
    let mut errors = Vec::new();

    errors.extend(validate_dati_generali(body));
    errors.extend(validate_natura_aliquota_coherence(body));
    errors.extend(validate_dati_riepilogo(body));
    errors.extend(validate_linee_riepilogo_match(body));

    // Validate each line item's price calculation
    if let Some(dbs) = &body.dati_beni_servizi {
        for linea in &dbs.dettaglio_linee {
            errors.extend(validate_prezzo_totale(linea));
        }
    }

    // Validate IBAN in payment details
    for dp in &body.dati_pagamento {
        for det in &dp.dettaglio_pagamento {
            if let Some(iban) = &det.iban {
                if let Some(e) = validate_iban(iban) {
                    errors.push(e);
                }
            }
        }
    }

    ValidationResult { errors }
}

/// SDI 00400: Natura must be present when AliquotaIVA is 0.
/// SDI 00401: Natura must NOT be present when AliquotaIVA is > 0.
///
/// Applies to both DettaglioLinee and DatiRiepilogo.
fn validate_natura_aliquota_coherence(body: &FatturaElettronicaBody) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if let Some(dbs) = &body.dati_beni_servizi {
        // Check DettaglioLinee
        for (i, linea) in dbs.dettaglio_linee.iter().enumerate() {
            let has_natura = linea.natura.is_some() && linea.natura != Some(0);
            let aliquota_zero = linea.aliquota_iva == 0.0;

            if aliquota_zero && !has_natura {
                errors.push(ValidationError {
                    code: "00400".into(),
                    message: format!("Natura is required when AliquotaIVA is 0 (line {})", linea.numero_linea),
                    element_path: Some(format!("Body/DatiBeniServizi/DettaglioLinee[{}]/Natura", i + 1)),
                });
            }
            if !aliquota_zero && has_natura {
                errors.push(ValidationError {
                    code: "00401".into(),
                    message: format!("Natura must not be present when AliquotaIVA > 0 (line {})", linea.numero_linea),
                    element_path: Some(format!("Body/DatiBeniServizi/DettaglioLinee[{}]/Natura", i + 1)),
                });
            }
        }

        // Check DatiRiepilogo
        for (i, riep) in dbs.dati_riepilogo.iter().enumerate() {
            let has_natura = riep.natura.is_some() && riep.natura != Some(0);
            let aliquota_zero = riep.aliquota_iva == 0.0;

            if aliquota_zero && !has_natura {
                errors.push(ValidationError {
                    code: "00400".into(),
                    message: "Natura is required in DatiRiepilogo when AliquotaIVA is 0".into(),
                    element_path: Some(format!("Body/DatiBeniServizi/DatiRiepilogo[{}]/Natura", i + 1)),
                });
            }
            if !aliquota_zero && has_natura {
                errors.push(ValidationError {
                    code: "00401".into(),
                    message: "Natura must not be present in DatiRiepilogo when AliquotaIVA > 0".into(),
                    element_path: Some(format!("Body/DatiBeniServizi/DatiRiepilogo[{}]/Natura", i + 1)),
                });
            }
        }
    }

    errors
}

/// SDI 00419: For each AliquotaIVA in DettaglioLinee, a DatiRiepilogo must exist.
/// SDI 00421: Imposta must equal AliquotaIVA * ImponibileImporto / 100.
/// SDI 00422: ImponibileImporto must equal the sum of PrezzoTotale for matching lines.
fn validate_dati_riepilogo(body: &FatturaElettronicaBody) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let Some(dbs) = &body.dati_beni_servizi else { return errors };

    // Group line totals by aliquota (using string key to avoid float comparison issues)
    let mut line_totals_by_aliquota: HashMap<String, f64> = HashMap::new();
    for linea in &dbs.dettaglio_linee {
        let key = format!("{:.2}", linea.aliquota_iva);
        *line_totals_by_aliquota.entry(key).or_default() += linea.prezzo_totale;
    }

    // Build a set of aliquota values present in riepilogo
    let mut riepilogo_aliquote: HashMap<String, usize> = HashMap::new();
    for (i, riep) in dbs.dati_riepilogo.iter().enumerate() {
        let key = format!("{:.2}", riep.aliquota_iva);
        riepilogo_aliquote.insert(key.clone(), i);

        // SDI 00421: Check imposta calculation
        let expected_imposta = riep.aliquota_iva * riep.imponibile_importo / 100.0;
        if (riep.imposta - expected_imposta).abs() > TOLERANCE_2 {
            errors.push(ValidationError {
                code: "00421".into(),
                message: format!(
                    "Imposta ({:.2}) does not match AliquotaIVA * ImponibileImporto / 100 ({:.2})",
                    riep.imposta, expected_imposta
                ),
                element_path: Some(format!("Body/DatiBeniServizi/DatiRiepilogo[{}]/Imposta", i + 1)),
            });
        }

        // SDI 00422: Check imponibile_importo matches sum of line totals
        if let Some(&sum) = line_totals_by_aliquota.get(&key) {
            if (riep.imponibile_importo - sum).abs() > TOLERANCE_2 {
                errors.push(ValidationError {
                    code: "00422".into(),
                    message: format!(
                        "ImponibileImporto ({:.2}) does not match sum of PrezzoTotale for AliquotaIVA {} ({:.2})",
                        riep.imponibile_importo, key, sum
                    ),
                    element_path: Some(format!("Body/DatiBeniServizi/DatiRiepilogo[{}]/ImponibileImporto", i + 1)),
                });
            }
        }
    }

    // SDI 00419: Check that every aliquota in lines has a riepilogo
    for key in line_totals_by_aliquota.keys() {
        if !riepilogo_aliquote.contains_key(key) {
            errors.push(ValidationError {
                code: "00419".into(),
                message: format!("No DatiRiepilogo found for AliquotaIVA {}", key),
                element_path: Some("Body/DatiBeniServizi/DatiRiepilogo".into()),
            });
        }
    }

    errors
}

/// SDI 00423: PrezzoTotale must equal PrezzoUnitario * Quantita +/- ScontoMaggiorazione.
///
/// Rounding tolerance is applied to account for floating-point precision.
fn validate_prezzo_totale(linea: &DettaglioLinee) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // If quantita is not set, we cannot verify the calculation
    let Some(quantita) = linea.quantita else { return errors };

    let base = linea.prezzo_unitario * quantita;
    let adjusted = apply_sconto_maggiorazione(base, &linea.sconto_maggiorazione);

    if (linea.prezzo_totale - adjusted).abs() > TOLERANCE_8 {
        errors.push(ValidationError {
            code: "00423".into(),
            message: format!(
                "PrezzoTotale ({}) does not match PrezzoUnitario * Quantita adjusted by ScontoMaggiorazione ({:.8}), line {}",
                linea.prezzo_totale, adjusted, linea.numero_linea
            ),
            element_path: Some(format!("Body/DatiBeniServizi/DettaglioLinee[{}]/PrezzoTotale", linea.numero_linea)),
        });
    }

    errors
}

/// Apply discount/surcharge adjustments to a base amount.
fn apply_sconto_maggiorazione(base: f64, adjustments: &[ScontoMaggiorazione]) -> f64 {
    let mut result = base;
    for sm in adjustments {
        let is_sconto = tipo_sconto_maggiorazione_to_sdi(sm.tipo) == Some("SC");
        let adjustment = if let Some(pct) = sm.percentuale {
            result * pct / 100.0
        } else if let Some(imp) = sm.importo {
            imp
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

/// SDI 00443: If DettaglioLinee has a specific AliquotaIVA,
/// a DatiRiepilogo with the same AliquotaIVA must exist.
///
/// SDI 00444: If DettaglioLinee has Natura set,
/// a DatiRiepilogo with the same Natura must exist.
fn validate_linee_riepilogo_match(body: &FatturaElettronicaBody) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let Some(dbs) = &body.dati_beni_servizi else { return errors };

    // Collect aliquota+natura pairs from riepilogo
    let riepilogo_keys: Vec<(String, Option<String>)> = dbs.dati_riepilogo.iter().map(|r| {
        let aliq = format!("{:.2}", r.aliquota_iva);
        let nat = r.natura.and_then(|n| natura_to_sdi(n).map(String::from));
        (aliq, nat)
    }).collect();

    for linea in &dbs.dettaglio_linee {
        let aliq = format!("{:.2}", linea.aliquota_iva);
        let nat = linea.natura.and_then(|n| natura_to_sdi(n).map(String::from));

        // SDI 00443: aliquota match
        if !riepilogo_keys.iter().any(|(a, _)| a == &aliq) {
            errors.push(ValidationError {
                code: "00443".into(),
                message: format!("No DatiRiepilogo with AliquotaIVA {} for line {}", aliq, linea.numero_linea),
                element_path: Some(format!("Body/DatiBeniServizi/DettaglioLinee[{}]/AliquotaIVA", linea.numero_linea)),
            });
        }

        // SDI 00444: natura match (only if natura is set on the line)
        if let Some(ref line_nat) = nat {
            if !riepilogo_keys.iter().any(|(_, rn)| rn.as_deref() == Some(line_nat)) {
                errors.push(ValidationError {
                    code: "00444".into(),
                    message: format!("No DatiRiepilogo with Natura {} for line {}", line_nat, linea.numero_linea),
                    element_path: Some(format!("Body/DatiBeniServizi/DettaglioLinee[{}]/Natura", linea.numero_linea)),
                });
            }
        }
    }

    errors
}

/// SDI 00471-00476: TipoDocumento-specific validation rules.
///
/// Verifies constraints based on the document type, such as:
/// - TD16-TD19 (reverse charge / self-invoicing): cedente/cessionario P.IVA constraints
/// - TD20-TD21 (self-invoicing): cedente must match cessionario
/// - TD27 (auto-consumption): cedente must match cessionario
fn validate_tipo_documento_rules(invoice: &FatturaElettronica) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let Some(header) = &invoice.header else { return errors };

    for body in &invoice.body {
        let Some(dg) = &body.dati_generali else { continue };
        let Some(dgd) = &dg.dati_generali_documento else { continue };

        let tipo = dgd.tipo_documento;

        // TD20, TD21, TD27: cedente and cessionario must be the same entity
        // (self-invoicing documents)
        if matches!(tipo, 200 | 210 | 270) {
            let cedente_piva = header.cedente_prestatore.as_ref()
                .and_then(|cp| cp.dati_anagrafici.as_ref())
                .and_then(|da| da.id_fiscale_iva.as_ref())
                .map(|id| (&id.id_paese, &id.id_codice));

            let cessionario_piva = header.cessionario_committente.as_ref()
                .and_then(|cc| cc.dati_anagrafici.as_ref())
                .and_then(|da| da.id_fiscale_iva.as_ref())
                .map(|id| (&id.id_paese, &id.id_codice));

            if cedente_piva != cessionario_piva {
                let tipo_str = crate::xml::enum_map::tipo_documento_to_sdi(tipo).unwrap_or("?");
                errors.push(ValidationError {
                    code: "00471".into(),
                    message: format!(
                        "For document type {} (self-invoicing), CedentePrestatore and CessionarioCommittente must have the same P.IVA",
                        tipo_str
                    ),
                    element_path: Some("Header/CedentePrestatore/DatiAnagrafici/IdFiscaleIVA".into()),
                });
            }
        }

        // TD17, TD18, TD19, TD28: cedente must be a foreign entity (IdPaese != "IT")
        if matches!(tipo, 170 | 180 | 190 | 280) {
            let cedente_paese = header.cedente_prestatore.as_ref()
                .and_then(|cp| cp.dati_anagrafici.as_ref())
                .and_then(|da| da.id_fiscale_iva.as_ref())
                .map(|id| id.id_paese.as_str());

            if cedente_paese == Some("IT") {
                let tipo_str = crate::xml::enum_map::tipo_documento_to_sdi(tipo).unwrap_or("?");
                errors.push(ValidationError {
                    code: "00473".into(),
                    message: format!(
                        "For document type {} (foreign acquisition), CedentePrestatore must be a foreign entity (IdPaese != IT)",
                        tipo_str
                    ),
                    element_path: Some("Header/CedentePrestatore/DatiAnagrafici/IdFiscaleIVA/IdPaese".into()),
                });
            }
        }
    }

    errors
}

// ===========================================================================
// Format validators
// ===========================================================================

/// Validate codice fiscale format: 11 digits (company) or 16 alphanumeric (individual).
fn validate_codice_fiscale(cf: &str) -> Option<ValidationError> {
    let len = cf.len();
    let all_alnum = cf.chars().all(|c| c.is_ascii_alphanumeric());

    if !all_alnum || (len != 11 && len != 16) {
        Some(ValidationError {
            code: "00100".into(),
            message: format!("Invalid CodiceFiscale format: '{}' (must be 11 or 16 alphanumeric characters)", cf),
            element_path: None,
        })
    } else {
        None
    }
}

/// Validate IdFiscale: IdPaese must be 2 uppercase letters, IdCodice 1-28 characters.
fn validate_id_fiscale(id_paese: &str, id_codice: &str) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if id_paese.len() != 2 || !id_paese.chars().all(|c| c.is_ascii_uppercase()) {
        errors.push(ValidationError {
            code: "00200".into(),
            message: format!("Invalid IdPaese: '{}' (must be 2 uppercase letters)", id_paese),
            element_path: None,
        });
    }

    if id_codice.is_empty() || id_codice.len() > 28 {
        errors.push(ValidationError {
            code: "00201".into(),
            message: format!("Invalid IdCodice: '{}' (must be 1-28 characters)", id_codice),
            element_path: None,
        });
    }

    errors
}

/// Validate IBAN format: 2 letters + 2 digits + 11-30 alphanumeric characters.
fn validate_iban(iban: &str) -> Option<ValidationError> {
    let valid = iban.len() >= 15
        && iban.len() <= 34
        && iban[..2].chars().all(|c| c.is_ascii_alphabetic())
        && iban[2..4].chars().all(|c| c.is_ascii_digit())
        && iban[4..].chars().all(|c| c.is_ascii_alphanumeric());

    if !valid {
        Some(ValidationError {
            code: "00300".into(),
            message: format!("Invalid IBAN format: '{}'", iban),
            element_path: None,
        })
    } else {
        None
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdi::v1::*;

    /// Build a minimal valid invoice for testing.
    fn minimal_invoice() -> FatturaElettronica {
        FatturaElettronica {
            versione: 10, // FPA12
            sistema_emittente: None,
            header: Some(FatturaElettronicaHeader {
                dati_trasmissione: Some(DatiTrasmissione {
                    id_trasmittente: Some(IdFiscale {
                        id_paese: "IT".into(),
                        id_codice: "01234567890".into(),
                    }),
                    progressivo_invio: "00001".into(),
                    formato_trasmissione: 10,
                    codice_destinatario: "0000000".into(),
                    ..Default::default()
                }),
                cedente_prestatore: Some(CedentePrestatore {
                    dati_anagrafici: Some(DatiAnagraficiCedente {
                        id_fiscale_iva: Some(IdFiscale {
                            id_paese: "IT".into(),
                            id_codice: "01234567890".into(),
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
                            id_codice: "09876543210".into(),
                        }),
                        anagrafica: Some(Anagrafica {
                            soggetto: Some(anagrafica::Soggetto::Denominazione("Cliente SRL".into())),
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
        assert!(result.is_valid(), "Expected valid, got: {:?}", result.errors);
    }

    #[test]
    fn test_sdi_00400_natura_required_when_aliquota_zero() {
        let mut inv = minimal_invoice();
        let body = &mut inv.body[0];
        let dbs = body.dati_beni_servizi.as_mut().unwrap();
        dbs.dettaglio_linee[0].aliquota_iva = 0.0;
        dbs.dettaglio_linee[0].natura = None; // Missing!
        dbs.dati_riepilogo[0].aliquota_iva = 0.0;
        dbs.dati_riepilogo[0].imposta = 0.0;

        let result = validate(&inv);
        assert!(result.errors.iter().any(|e| e.code == "00400"));
    }

    #[test]
    fn test_sdi_00401_natura_forbidden_when_aliquota_nonzero() {
        let mut inv = minimal_invoice();
        let body = &mut inv.body[0];
        let dbs = body.dati_beni_servizi.as_mut().unwrap();
        dbs.dettaglio_linee[0].natura = Some(10); // N1, but aliquota is 22%

        let result = validate(&inv);
        assert!(result.errors.iter().any(|e| e.code == "00401"));
    }

    #[test]
    fn test_sdi_00421_imposta_calculation() {
        let mut inv = minimal_invoice();
        let body = &mut inv.body[0];
        let dbs = body.dati_beni_servizi.as_mut().unwrap();
        dbs.dati_riepilogo[0].imposta = 99.0; // Wrong: should be 22.0

        let result = validate(&inv);
        assert!(result.errors.iter().any(|e| e.code == "00421"));
    }

    #[test]
    fn test_sdi_00423_prezzo_totale() {
        let mut inv = minimal_invoice();
        let body = &mut inv.body[0];
        let dbs = body.dati_beni_servizi.as_mut().unwrap();
        dbs.dettaglio_linee[0].prezzo_totale = 999.0; // Wrong

        let result = validate(&inv);
        assert!(result.errors.iter().any(|e| e.code == "00423"));
    }

    #[test]
    fn test_codice_fiscale_valid() {
        assert!(validate_codice_fiscale("01234567890").is_none());
        assert!(validate_codice_fiscale("RSSMRA80A01H501U").is_none());
    }

    #[test]
    fn test_codice_fiscale_invalid() {
        assert!(validate_codice_fiscale("123").is_some());
        assert!(validate_codice_fiscale("invalid!chars!!").is_some());
    }

    #[test]
    fn test_iban_valid() {
        assert!(validate_iban("IT60X0542811101000000123456").is_none());
    }

    #[test]
    fn test_iban_invalid() {
        assert!(validate_iban("INVALID").is_some());
        assert!(validate_iban("12345").is_some());
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
}
