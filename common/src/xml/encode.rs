use std::io::{Cursor, Write};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use quick_xml::Writer;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};

use crate::sdi::v1::*;
use super::XmlError;
use super::enum_map::*;

/// SDI XML namespace URI for FatturaPA v1.2.2.
const SDI_NS: &str = "http://ivaservizi.agenziaentrate.gov.it/docs/xsd/fatture/v1.2";

/// Encode a `FatturaElettronica` proto message into an SDI-compliant XML string.
///
/// The output follows the Agenzia delle Entrate FatturaPA v1.2.2 schema with:
/// - PascalCase element names matching the official XSD
/// - XML namespace prefix `p:` with the SDI schema URI
/// - Enum values emitted as their SDI string codes (e.g. `"FPA12"`, `"TD01"`)
/// - Optional fields omitted when `None`
/// - Repeated fields emitted as multiple same-named sibling elements
/// - `bytes` fields (attachments) base64-encoded
pub fn encode(invoice: &FatturaElettronica) -> Result<String, XmlError> {
    let mut buf = Cursor::new(Vec::new());
    encode_to_writer(invoice, &mut buf)?;
    String::from_utf8(buf.into_inner())
        .map_err(|e| XmlError::Serialize(e.to_string()))
}

/// Encode a `FatturaElettronica` and write directly to a writer (file, network stream).
///
/// More efficient than [`encode`] for large invoices as it avoids building
/// the full string in memory.
pub fn encode_to_writer<W: Write>(
    invoice: &FatturaElettronica,
    writer: W,
) -> Result<(), XmlError> {
    let mut w = Writer::new(writer);
    write_fattura(&mut w, invoice)
}

// ===========================================================================
// Private writer helpers — one per proto message type
// ===========================================================================

/// Write a simple text element: `<Tag>text</Tag>`.
fn write_text_elem<W: Write>(w: &mut Writer<W>, tag: &str, text: &str) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new(tag)))?;
    w.write_event(Event::Text(BytesText::new(text)))?;
    w.write_event(Event::End(BytesEnd::new(tag)))?;
    Ok(())
}

/// Write a text element only if the value is `Some`.
fn write_opt_text<W: Write>(w: &mut Writer<W>, tag: &str, val: &Option<String>) -> Result<(), XmlError> {
    if let Some(v) = val {
        write_text_elem(w, tag, v)?;
    }
    Ok(())
}

/// Write an enum field as its SDI string code. Skips if the value maps to `None` (UNSPECIFIED).
fn write_enum<W: Write>(
    w: &mut Writer<W>,
    tag: &str,
    value: i32,
    to_sdi: fn(i32) -> Option<&'static str>,
) -> Result<(), XmlError> {
    if let Some(s) = to_sdi(value) {
        write_text_elem(w, tag, s)?;
    }
    Ok(())
}

/// Write an optional enum field. Skips if `None` or UNSPECIFIED.
fn write_opt_enum<W: Write>(
    w: &mut Writer<W>,
    tag: &str,
    value: Option<i32>,
    to_sdi: fn(i32) -> Option<&'static str>,
) -> Result<(), XmlError> {
    if let Some(v) = value {
        write_enum(w, tag, v, to_sdi)?;
    }
    Ok(())
}

/// Write a `double` field formatted with 2 decimal places.
fn write_amount2<W: Write>(w: &mut Writer<W>, tag: &str, value: f64) -> Result<(), XmlError> {
    write_text_elem(w, tag, &format!("{:.2}", value))
}

/// Write an optional `double` field formatted with 2 decimal places.
fn write_opt_amount2<W: Write>(w: &mut Writer<W>, tag: &str, value: Option<f64>) -> Result<(), XmlError> {
    if let Some(v) = value {
        write_amount2(w, tag, v)?;
    }
    Ok(())
}

/// Write a `double` field formatted with up to 8 decimal places (trailing zeros stripped).
fn write_amount8<W: Write>(w: &mut Writer<W>, tag: &str, value: f64) -> Result<(), XmlError> {
    write_text_elem(w, tag, &format_amount8(value))
}

/// Write an optional `double` field formatted with up to 8 decimal places.
fn write_opt_amount8<W: Write>(w: &mut Writer<W>, tag: &str, value: Option<f64>) -> Result<(), XmlError> {
    if let Some(v) = value {
        write_amount8(w, tag, v)?;
    }
    Ok(())
}

/// Format a number with up to 8 decimal places, keeping at least 2.
fn format_amount8(value: f64) -> String {
    // Guard against non-finite values (NaN/inf have no decimal point and would panic).
    if !value.is_finite() {
        return "0.00".to_string();
    }
    let s = format!("{:.8}", value);
    let s = s.trim_end_matches('0');
    // Ensure at least 2 decimal places
    let dot_pos = match s.find('.') {
        Some(p) => p,
        None => return format!("{:.2}", value),
    };
    let decimals = s.len() - dot_pos - 1;
    if decimals < 2 {
        format!("{:.2}", value)
    } else {
        s.to_string()
    }
}

/// Write an optional `int32` field.
fn write_opt_int<W: Write>(w: &mut Writer<W>, tag: &str, value: Option<i32>) -> Result<(), XmlError> {
    if let Some(v) = value {
        write_text_elem(w, tag, &v.to_string())?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Root element
// ---------------------------------------------------------------------------

fn write_fattura<W: Write>(w: &mut Writer<W>, inv: &FatturaElettronica) -> Result<(), XmlError> {
    // XML declaration
    w.write_event(Event::Decl(quick_xml::events::BytesDecl::new("1.0", Some("UTF-8"), None)))?;

    // Root element with namespace and versione attribute
    let versione = formato_trasmissione_to_sdi(inv.versione)
        .ok_or_else(|| XmlError::Serialize("Invalid FormatoTrasmissione value".into()))?;

    let mut root = BytesStart::new("p:FatturaElettronica");
    root.push_attribute(("xmlns:p", SDI_NS));
    root.push_attribute(("versione", versione));
    w.write_event(Event::Start(root))?;

    // Optional SistemaEmittente attribute is actually a child element in the XSD,
    // but the versione attribute is on the root. SistemaEmittente is NOT present
    // in the root attributes — it is not part of the standard schema root attributes.

    // FatturaElettronicaHeader
    if let Some(header) = &inv.header {
        write_header(w, header)?;
    }

    // FatturaElettronicaBody (repeated)
    for body in &inv.body {
        write_body(w, body)?;
    }

    w.write_event(Event::End(BytesEnd::new("p:FatturaElettronica")))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

fn write_header<W: Write>(w: &mut Writer<W>, h: &FatturaElettronicaHeader) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("FatturaElettronicaHeader")))?;

    if let Some(dt) = &h.dati_trasmissione {
        write_dati_trasmissione(w, dt)?;
    }
    if let Some(cp) = &h.cedente_prestatore {
        write_cedente_prestatore(w, cp)?;
    }
    if let Some(rf) = &h.rappresentante_fiscale {
        write_rappresentante_fiscale(w, rf)?;
    }
    if let Some(cc) = &h.cessionario_committente {
        write_cessionario_committente(w, cc)?;
    }
    if let Some(ti) = &h.terzo_intermediario_o_soggetto_emittente {
        write_terzo_intermediario(w, ti)?;
    }
    write_opt_enum(w, "SoggettoEmittente", h.soggetto_emittente, soggetto_emittente_to_sdi)?;

    w.write_event(Event::End(BytesEnd::new("FatturaElettronicaHeader")))?;
    Ok(())
}

fn write_dati_trasmissione<W: Write>(w: &mut Writer<W>, dt: &DatiTrasmissione) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("DatiTrasmissione")))?;

    if let Some(id) = &dt.id_trasmittente {
        write_id_fiscale(w, "IdTrasmittente", id)?;
    }
    write_text_elem(w, "ProgressivoInvio", &dt.progressivo_invio)?;
    write_enum(w, "FormatoTrasmissione", dt.formato_trasmissione, formato_trasmissione_to_sdi)?;
    write_text_elem(w, "CodiceDestinatario", &dt.codice_destinatario)?;
    if let Some(ct) = &dt.contatti_trasmittente {
        write_contatti_trasmittente(w, ct)?;
    }
    write_opt_text(w, "PECDestinatario", &dt.pec_destinatario)?;

    w.write_event(Event::End(BytesEnd::new("DatiTrasmissione")))?;
    Ok(())
}

fn write_id_fiscale<W: Write>(w: &mut Writer<W>, tag: &str, id: &IdFiscale) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new(tag)))?;
    write_text_elem(w, "IdPaese", &id.id_paese)?;
    write_text_elem(w, "IdCodice", &id.id_codice)?;
    w.write_event(Event::End(BytesEnd::new(tag)))?;
    Ok(())
}

fn write_contatti_trasmittente<W: Write>(w: &mut Writer<W>, ct: &ContattiTrasmittente) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("ContattiTrasmittente")))?;
    write_opt_text(w, "Telefono", &ct.telefono)?;
    write_opt_text(w, "Email", &ct.email)?;
    w.write_event(Event::End(BytesEnd::new("ContattiTrasmittente")))?;
    Ok(())
}

fn write_cedente_prestatore<W: Write>(w: &mut Writer<W>, cp: &CedentePrestatore) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("CedentePrestatore")))?;

    if let Some(da) = &cp.dati_anagrafici {
        write_dati_anagrafici_cedente(w, da)?;
    }
    if let Some(sede) = &cp.sede {
        write_indirizzo(w, "Sede", sede)?;
    }
    if let Some(so) = &cp.stabile_organizzazione {
        write_indirizzo(w, "StabileOrganizzazione", so)?;
    }
    if let Some(rea) = &cp.iscrizione_rea {
        write_iscrizione_rea(w, rea)?;
    }
    if let Some(c) = &cp.contatti {
        write_contatti(w, c)?;
    }
    write_opt_text(w, "RiferimentoAmministrazione", &cp.riferimento_amministrazione)?;

    w.write_event(Event::End(BytesEnd::new("CedentePrestatore")))?;
    Ok(())
}

fn write_dati_anagrafici_cedente<W: Write>(w: &mut Writer<W>, da: &DatiAnagraficiCedente) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("DatiAnagrafici")))?;

    if let Some(id) = &da.id_fiscale_iva {
        write_id_fiscale(w, "IdFiscaleIVA", id)?;
    }
    write_opt_text(w, "CodiceFiscale", &da.codice_fiscale)?;
    if let Some(a) = &da.anagrafica {
        write_anagrafica(w, a)?;
    }
    write_opt_text(w, "AlboProfessionale", &da.albo_professionale)?;
    write_opt_text(w, "ProvinciaAlbo", &da.provincia_albo)?;
    write_opt_text(w, "NumeroIscrizioneAlbo", &da.numero_iscrizione_albo)?;
    write_opt_text(w, "DataIscrizioneAlbo", &da.data_iscrizione_albo)?;
    write_enum(w, "RegimeFiscale", da.regime_fiscale, regime_fiscale_to_sdi)?;

    w.write_event(Event::End(BytesEnd::new("DatiAnagrafici")))?;
    Ok(())
}

fn write_anagrafica<W: Write>(w: &mut Writer<W>, a: &Anagrafica) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("Anagrafica")))?;

    // Oneof soggetto: Denominazione or Nome+Cognome
    if let Some(soggetto) = &a.soggetto {
        match soggetto {
            anagrafica::Soggetto::Denominazione(d) => {
                write_text_elem(w, "Denominazione", d)?;
            }
            anagrafica::Soggetto::PersonaFisica(pf) => {
                write_text_elem(w, "Nome", &pf.nome)?;
                write_text_elem(w, "Cognome", &pf.cognome)?;
            }
        }
    }
    write_opt_text(w, "Titolo", &a.titolo)?;
    write_opt_text(w, "CodEORI", &a.cod_eori)?;

    w.write_event(Event::End(BytesEnd::new("Anagrafica")))?;
    Ok(())
}

fn write_indirizzo<W: Write>(w: &mut Writer<W>, tag: &str, addr: &Indirizzo) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new(tag)))?;
    write_text_elem(w, "Indirizzo", &addr.indirizzo)?;
    write_opt_text(w, "NumeroCivico", &addr.numero_civico)?;
    write_text_elem(w, "CAP", &addr.cap)?;
    write_text_elem(w, "Comune", &addr.comune)?;
    write_opt_text(w, "Provincia", &addr.provincia)?;
    write_text_elem(w, "Nazione", &addr.nazione)?;
    w.write_event(Event::End(BytesEnd::new(tag)))?;
    Ok(())
}

fn write_iscrizione_rea<W: Write>(w: &mut Writer<W>, rea: &IscrizioneRea) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("IscrizioneREA")))?;
    write_text_elem(w, "Ufficio", &rea.ufficio)?;
    write_text_elem(w, "NumeroREA", &rea.numero_rea)?;
    write_opt_amount2(w, "CapitaleSociale", rea.capitale_sociale)?;
    write_opt_enum(w, "SocioUnico", rea.socio_unico, socio_unico_to_sdi)?;
    write_enum(w, "StatoLiquidazione", rea.stato_liquidazione, stato_liquidazione_to_sdi)?;
    w.write_event(Event::End(BytesEnd::new("IscrizioneREA")))?;
    Ok(())
}

fn write_contatti<W: Write>(w: &mut Writer<W>, c: &Contatti) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("Contatti")))?;
    write_opt_text(w, "Telefono", &c.telefono)?;
    write_opt_text(w, "Fax", &c.fax)?;
    write_opt_text(w, "Email", &c.email)?;
    w.write_event(Event::End(BytesEnd::new("Contatti")))?;
    Ok(())
}

fn write_rappresentante_fiscale<W: Write>(w: &mut Writer<W>, rf: &RappresentanteFiscale) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("RappresentanteFiscale")))?;
    if let Some(da) = &rf.dati_anagrafici {
        write_dati_anagrafici_rappresentante(w, da)?;
    }
    w.write_event(Event::End(BytesEnd::new("RappresentanteFiscale")))?;
    Ok(())
}

fn write_dati_anagrafici_rappresentante<W: Write>(w: &mut Writer<W>, da: &DatiAnagraficiRappresentante) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("DatiAnagrafici")))?;
    if let Some(id) = &da.id_fiscale_iva {
        write_id_fiscale(w, "IdFiscaleIVA", id)?;
    }
    write_opt_text(w, "CodiceFiscale", &da.codice_fiscale)?;
    if let Some(a) = &da.anagrafica {
        write_anagrafica(w, a)?;
    }
    w.write_event(Event::End(BytesEnd::new("DatiAnagrafici")))?;
    Ok(())
}

fn write_cessionario_committente<W: Write>(w: &mut Writer<W>, cc: &CessionarioCommittente) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("CessionarioCommittente")))?;

    if let Some(da) = &cc.dati_anagrafici {
        write_dati_anagrafici_cessionario(w, da)?;
    }
    if let Some(sede) = &cc.sede {
        write_indirizzo(w, "Sede", sede)?;
    }
    if let Some(so) = &cc.stabile_organizzazione {
        write_indirizzo(w, "StabileOrganizzazione", so)?;
    }
    if let Some(rf) = &cc.rappresentante_fiscale {
        write_rappresentante_fiscale_cessionario(w, rf)?;
    }

    w.write_event(Event::End(BytesEnd::new("CessionarioCommittente")))?;
    Ok(())
}

fn write_dati_anagrafici_cessionario<W: Write>(w: &mut Writer<W>, da: &DatiAnagraficiCessionario) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("DatiAnagrafici")))?;
    if let Some(id) = &da.id_fiscale_iva {
        write_id_fiscale(w, "IdFiscaleIVA", id)?;
    }
    write_opt_text(w, "CodiceFiscale", &da.codice_fiscale)?;
    if let Some(a) = &da.anagrafica {
        write_anagrafica(w, a)?;
    }
    w.write_event(Event::End(BytesEnd::new("DatiAnagrafici")))?;
    Ok(())
}

fn write_rappresentante_fiscale_cessionario<W: Write>(w: &mut Writer<W>, rf: &RappresentanteFiscaleCessionario) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("RappresentanteFiscale")))?;
    if let Some(id) = &rf.id_fiscale_iva {
        write_id_fiscale(w, "IdFiscaleIVA", id)?;
    }
    // Oneof soggetto
    if let Some(soggetto) = &rf.soggetto {
        match soggetto {
            rappresentante_fiscale_cessionario::Soggetto::Denominazione(d) => {
                write_text_elem(w, "Denominazione", d)?;
            }
            rappresentante_fiscale_cessionario::Soggetto::PersonaFisica(pf) => {
                write_text_elem(w, "Nome", &pf.nome)?;
                write_text_elem(w, "Cognome", &pf.cognome)?;
            }
        }
    }
    w.write_event(Event::End(BytesEnd::new("RappresentanteFiscale")))?;
    Ok(())
}

fn write_terzo_intermediario<W: Write>(w: &mut Writer<W>, ti: &TerzoIntermediarioOSoggettoEmittente) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("TerzoIntermediarioOSoggettoEmittente")))?;
    if let Some(da) = &ti.dati_anagrafici {
        write_dati_anagrafici_terzo(w, da)?;
    }
    w.write_event(Event::End(BytesEnd::new("TerzoIntermediarioOSoggettoEmittente")))?;
    Ok(())
}

fn write_dati_anagrafici_terzo<W: Write>(w: &mut Writer<W>, da: &DatiAnagraficiTerzoIntermediario) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("DatiAnagrafici")))?;
    if let Some(id) = &da.id_fiscale_iva {
        write_id_fiscale(w, "IdFiscaleIVA", id)?;
    }
    write_opt_text(w, "CodiceFiscale", &da.codice_fiscale)?;
    if let Some(a) = &da.anagrafica {
        write_anagrafica(w, a)?;
    }
    w.write_event(Event::End(BytesEnd::new("DatiAnagrafici")))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Body
// ---------------------------------------------------------------------------

fn write_body<W: Write>(w: &mut Writer<W>, body: &FatturaElettronicaBody) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("FatturaElettronicaBody")))?;

    if let Some(dg) = &body.dati_generali {
        write_dati_generali(w, dg)?;
    }
    if let Some(dbs) = &body.dati_beni_servizi {
        write_dati_beni_servizi(w, dbs)?;
    }
    if let Some(dv) = &body.dati_veicoli {
        write_dati_veicoli(w, dv)?;
    }
    for dp in &body.dati_pagamento {
        write_dati_pagamento(w, dp)?;
    }
    for al in &body.allegati {
        write_allegati(w, al)?;
    }

    w.write_event(Event::End(BytesEnd::new("FatturaElettronicaBody")))?;
    Ok(())
}

// --- DatiGenerali ---

fn write_dati_generali<W: Write>(w: &mut Writer<W>, dg: &DatiGenerali) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("DatiGenerali")))?;

    if let Some(dgd) = &dg.dati_generali_documento {
        write_dati_generali_documento(w, dgd)?;
    }
    for d in &dg.dati_ordine_acquisto { write_dati_documenti_correlati(w, "DatiOrdineAcquisto", d)?; }
    for d in &dg.dati_contratto { write_dati_documenti_correlati(w, "DatiContratto", d)?; }
    for d in &dg.dati_convenzione { write_dati_documenti_correlati(w, "DatiConvenzione", d)?; }
    for d in &dg.dati_ricezione { write_dati_documenti_correlati(w, "DatiRicezione", d)?; }
    for d in &dg.dati_fatture_collegate { write_dati_documenti_correlati(w, "DatiFattureCollegate", d)?; }
    for sal in &dg.dati_sal { write_dati_sal(w, sal)?; }
    for ddt in &dg.dati_ddt { write_dati_ddt(w, ddt)?; }
    if let Some(dt) = &dg.dati_trasporto {
        write_dati_trasporto(w, dt)?;
    }
    if let Some(fp) = &dg.fattura_principale {
        write_fattura_principale(w, fp)?;
    }

    w.write_event(Event::End(BytesEnd::new("DatiGenerali")))?;
    Ok(())
}

fn write_dati_generali_documento<W: Write>(w: &mut Writer<W>, dgd: &DatiGeneraliDocumento) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("DatiGeneraliDocumento")))?;

    write_enum(w, "TipoDocumento", dgd.tipo_documento, tipo_documento_to_sdi)?;
    write_text_elem(w, "Divisa", &dgd.divisa)?;
    write_text_elem(w, "Data", &dgd.data)?;
    write_text_elem(w, "Numero", &dgd.numero)?;
    for dr in &dgd.dati_ritenuta { write_dati_ritenuta(w, dr)?; }
    if let Some(db) = &dgd.dati_bollo { write_dati_bollo(w, db)?; }
    for dcp in &dgd.dati_cassa_previdenziale { write_dati_cassa_previdenziale(w, dcp)?; }
    for sm in &dgd.sconto_maggiorazione { write_sconto_maggiorazione(w, sm)?; }
    write_opt_amount2(w, "ImportoTotaleDocumento", dgd.importo_totale_documento)?;
    write_opt_amount2(w, "Arrotondamento", dgd.arrotondamento)?;
    for c in &dgd.causale { write_text_elem(w, "Causale", c)?; }
    write_opt_enum(w, "Art73", dgd.art73, art73_to_sdi)?;

    w.write_event(Event::End(BytesEnd::new("DatiGeneraliDocumento")))?;
    Ok(())
}

fn write_dati_ritenuta<W: Write>(w: &mut Writer<W>, dr: &DatiRitenuta) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("DatiRitenuta")))?;
    write_enum(w, "TipoRitenuta", dr.tipo_ritenuta, tipo_ritenuta_to_sdi)?;
    write_amount2(w, "ImportoRitenuta", dr.importo_ritenuta)?;
    write_amount2(w, "AliquotaRitenuta", dr.aliquota_ritenuta)?;
    write_enum(w, "CausalePagamento", dr.causale_pagamento, causale_pagamento_to_sdi)?;
    w.write_event(Event::End(BytesEnd::new("DatiRitenuta")))?;
    Ok(())
}

fn write_dati_bollo<W: Write>(w: &mut Writer<W>, db: &DatiBollo) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("DatiBollo")))?;
    write_enum(w, "BolloVirtuale", db.bollo_virtuale, bollo_virtuale_to_sdi)?;
    write_opt_amount2(w, "ImportoBollo", db.importo_bollo)?;
    w.write_event(Event::End(BytesEnd::new("DatiBollo")))?;
    Ok(())
}

fn write_dati_cassa_previdenziale<W: Write>(w: &mut Writer<W>, dcp: &DatiCassaPrevidenziale) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("DatiCassaPrevidenziale")))?;
    write_enum(w, "TipoCassa", dcp.tipo_cassa, tipo_cassa_to_sdi)?;
    write_amount2(w, "AlCassa", dcp.al_cassa)?;
    write_amount2(w, "ImportoContributoCassa", dcp.importo_contributo_cassa)?;
    write_opt_amount2(w, "ImponibileCassa", dcp.imponibile_cassa)?;
    write_amount2(w, "AliquotaIVA", dcp.aliquota_iva)?;
    write_opt_enum(w, "Ritenuta", dcp.ritenuta, ritenuta_to_sdi)?;
    write_opt_enum(w, "Natura", dcp.natura, natura_to_sdi)?;
    write_opt_text(w, "RiferimentoAmministrazione", &dcp.riferimento_amministrazione)?;
    w.write_event(Event::End(BytesEnd::new("DatiCassaPrevidenziale")))?;
    Ok(())
}

fn write_sconto_maggiorazione<W: Write>(w: &mut Writer<W>, sm: &ScontoMaggiorazione) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("ScontoMaggiorazione")))?;
    write_enum(w, "Tipo", sm.tipo, tipo_sconto_maggiorazione_to_sdi)?;
    write_opt_amount2(w, "Percentuale", sm.percentuale)?;
    write_opt_amount8(w, "Importo", sm.importo)?;
    w.write_event(Event::End(BytesEnd::new("ScontoMaggiorazione")))?;
    Ok(())
}

fn write_dati_documenti_correlati<W: Write>(w: &mut Writer<W>, tag: &str, d: &DatiDocumentiCorrelati) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new(tag)))?;
    for num in &d.riferimento_numero_linea {
        write_text_elem(w, "RiferimentoNumeroLinea", &num.to_string())?;
    }
    write_text_elem(w, "IdDocumento", &d.id_documento)?;
    write_opt_text(w, "Data", &d.data)?;
    write_opt_text(w, "NumItem", &d.num_item)?;
    write_opt_text(w, "CodiceCommessaConvenzione", &d.codice_commessa_convenzione)?;
    write_opt_text(w, "CodiceCUP", &d.codice_cup)?;
    write_opt_text(w, "CodiceCIG", &d.codice_cig)?;
    w.write_event(Event::End(BytesEnd::new(tag)))?;
    Ok(())
}

fn write_dati_sal<W: Write>(w: &mut Writer<W>, sal: &DatiSal) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("DatiSAL")))?;
    write_text_elem(w, "RiferimentoFase", &sal.riferimento_fase.to_string())?;
    w.write_event(Event::End(BytesEnd::new("DatiSAL")))?;
    Ok(())
}

fn write_dati_ddt<W: Write>(w: &mut Writer<W>, ddt: &DatiDdt) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("DatiDDT")))?;
    write_text_elem(w, "NumeroDDT", &ddt.numero_ddt)?;
    write_text_elem(w, "DataDDT", &ddt.data_ddt)?;
    for num in &ddt.riferimento_numero_linea {
        write_text_elem(w, "RiferimentoNumeroLinea", &num.to_string())?;
    }
    w.write_event(Event::End(BytesEnd::new("DatiDDT")))?;
    Ok(())
}

fn write_dati_trasporto<W: Write>(w: &mut Writer<W>, dt: &DatiTrasporto) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("DatiTrasporto")))?;
    if let Some(dav) = &dt.dati_anagrafici_vettore {
        write_dati_anagrafici_vettore(w, dav)?;
    }
    write_opt_text(w, "MezzoTrasporto", &dt.mezzo_trasporto)?;
    write_opt_text(w, "CausaleTrasporto", &dt.causale_trasporto)?;
    write_opt_int(w, "NumeroColli", dt.numero_colli)?;
    write_opt_text(w, "Descrizione", &dt.descrizione)?;
    write_opt_text(w, "UnitaMisuraPeso", &dt.unita_misura_peso)?;
    write_opt_amount2(w, "PesoLordo", dt.peso_lordo)?;
    write_opt_amount2(w, "PesoNetto", dt.peso_netto)?;
    write_opt_text(w, "DataOraRitiro", &dt.data_ora_ritiro)?;
    write_opt_text(w, "DataInizioTrasporto", &dt.data_inizio_trasporto)?;
    write_opt_text(w, "TipoResa", &dt.tipo_resa)?;
    if let Some(ir) = &dt.indirizzo_resa {
        write_indirizzo(w, "IndirizzoResa", ir)?;
    }
    write_opt_text(w, "DataOraConsegna", &dt.data_ora_consegna)?;
    w.write_event(Event::End(BytesEnd::new("DatiTrasporto")))?;
    Ok(())
}

fn write_dati_anagrafici_vettore<W: Write>(w: &mut Writer<W>, dav: &DatiAnagraficiVettore) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("DatiAnagraficiVettore")))?;
    if let Some(id) = &dav.id_fiscale_iva {
        write_id_fiscale(w, "IdFiscaleIVA", id)?;
    }
    write_opt_text(w, "CodiceFiscale", &dav.codice_fiscale)?;
    if let Some(a) = &dav.anagrafica {
        write_anagrafica(w, a)?;
    }
    write_opt_text(w, "NumeroLicenzaGuida", &dav.numero_licenza_guida)?;
    w.write_event(Event::End(BytesEnd::new("DatiAnagraficiVettore")))?;
    Ok(())
}

fn write_fattura_principale<W: Write>(w: &mut Writer<W>, fp: &FatturaPrincipale) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("FatturaPrincipale")))?;
    write_text_elem(w, "NumeroFatturaPrincipale", &fp.numero_fattura_principale)?;
    write_text_elem(w, "DataFatturaPrincipale", &fp.data_fattura_principale)?;
    w.write_event(Event::End(BytesEnd::new("FatturaPrincipale")))?;
    Ok(())
}

// --- DatiBeniServizi ---

fn write_dati_beni_servizi<W: Write>(w: &mut Writer<W>, dbs: &DatiBeniServizi) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("DatiBeniServizi")))?;
    for dl in &dbs.dettaglio_linee { write_dettaglio_linee(w, dl)?; }
    for dr in &dbs.dati_riepilogo { write_dati_riepilogo(w, dr)?; }
    w.write_event(Event::End(BytesEnd::new("DatiBeniServizi")))?;
    Ok(())
}

fn write_dettaglio_linee<W: Write>(w: &mut Writer<W>, dl: &DettaglioLinee) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("DettaglioLinee")))?;

    write_text_elem(w, "NumeroLinea", &dl.numero_linea.to_string())?;
    write_opt_enum(w, "TipoCessionePrestazione", dl.tipo_cessione_prestazione, tipo_cessione_prestazione_to_sdi)?;
    for ca in &dl.codice_articolo { write_codice_articolo(w, ca)?; }
    write_text_elem(w, "Descrizione", &dl.descrizione)?;
    write_opt_amount8(w, "Quantita", dl.quantita)?;
    write_opt_text(w, "UnitaMisura", &dl.unita_misura)?;
    write_opt_text(w, "DataInizioPeriodo", &dl.data_inizio_periodo)?;
    write_opt_text(w, "DataFinePeriodo", &dl.data_fine_periodo)?;
    write_amount8(w, "PrezzoUnitario", dl.prezzo_unitario)?;
    for sm in &dl.sconto_maggiorazione { write_sconto_maggiorazione(w, sm)?; }
    write_amount8(w, "PrezzoTotale", dl.prezzo_totale)?;
    write_amount2(w, "AliquotaIVA", dl.aliquota_iva)?;
    write_opt_enum(w, "Ritenuta", dl.ritenuta, ritenuta_to_sdi)?;
    write_opt_enum(w, "Natura", dl.natura, natura_to_sdi)?;
    write_opt_text(w, "RiferimentoAmministrazione", &dl.riferimento_amministrazione)?;
    for adg in &dl.altri_dati_gestionali { write_altri_dati_gestionali(w, adg)?; }

    w.write_event(Event::End(BytesEnd::new("DettaglioLinee")))?;
    Ok(())
}

fn write_codice_articolo<W: Write>(w: &mut Writer<W>, ca: &CodiceArticolo) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("CodiceArticolo")))?;
    write_text_elem(w, "CodiceTipo", &ca.codice_tipo)?;
    write_text_elem(w, "CodiceValore", &ca.codice_valore)?;
    w.write_event(Event::End(BytesEnd::new("CodiceArticolo")))?;
    Ok(())
}

fn write_altri_dati_gestionali<W: Write>(w: &mut Writer<W>, adg: &AltriDatiGestionali) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("AltriDatiGestionali")))?;
    write_text_elem(w, "TipoDato", &adg.tipo_dato)?;
    write_opt_text(w, "RiferimentoTesto", &adg.riferimento_testo)?;
    write_opt_amount8(w, "RiferimentoNumero", adg.riferimento_numero)?;
    write_opt_text(w, "RiferimentoData", &adg.riferimento_data)?;
    w.write_event(Event::End(BytesEnd::new("AltriDatiGestionali")))?;
    Ok(())
}

fn write_dati_riepilogo<W: Write>(w: &mut Writer<W>, dr: &DatiRiepilogo) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("DatiRiepilogo")))?;
    write_amount2(w, "AliquotaIVA", dr.aliquota_iva)?;
    write_opt_enum(w, "Natura", dr.natura, natura_to_sdi)?;
    write_opt_amount2(w, "SpeseAccessorie", dr.spese_accessorie)?;
    write_opt_amount8(w, "Arrotondamento", dr.arrotondamento)?;
    write_amount2(w, "ImponibileImporto", dr.imponibile_importo)?;
    write_amount2(w, "Imposta", dr.imposta)?;
    write_opt_enum(w, "EsigibilitaIVA", dr.esigibilita_iva, esigibilita_iva_to_sdi)?;
    write_opt_text(w, "RiferimentoNormativo", &dr.riferimento_normativo)?;
    w.write_event(Event::End(BytesEnd::new("DatiRiepilogo")))?;
    Ok(())
}

// --- DatiVeicoli ---

fn write_dati_veicoli<W: Write>(w: &mut Writer<W>, dv: &DatiVeicoli) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("DatiVeicoli")))?;
    write_text_elem(w, "Data", &dv.data)?;
    write_text_elem(w, "TotalePercorso", &dv.totale_percorso)?;
    w.write_event(Event::End(BytesEnd::new("DatiVeicoli")))?;
    Ok(())
}

// --- DatiPagamento ---

fn write_dati_pagamento<W: Write>(w: &mut Writer<W>, dp: &DatiPagamento) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("DatiPagamento")))?;
    write_enum(w, "CondizioniPagamento", dp.condizioni_pagamento, condizioni_pagamento_to_sdi)?;
    for det in &dp.dettaglio_pagamento { write_dettaglio_pagamento(w, det)?; }
    w.write_event(Event::End(BytesEnd::new("DatiPagamento")))?;
    Ok(())
}

fn write_dettaglio_pagamento<W: Write>(w: &mut Writer<W>, det: &DettaglioPagamento) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("DettaglioPagamento")))?;

    write_opt_text(w, "Beneficiario", &det.beneficiario)?;
    write_enum(w, "ModalitaPagamento", det.modalita_pagamento, modalita_pagamento_to_sdi)?;
    write_opt_text(w, "DataRiferimentoTerminiPagamento", &det.data_riferimento_termini_pagamento)?;
    write_opt_int(w, "GiorniTerminiPagamento", det.giorni_termini_pagamento)?;
    write_opt_text(w, "DataScadenzaPagamento", &det.data_scadenza_pagamento)?;
    write_amount2(w, "ImportoPagamento", det.importo_pagamento)?;
    write_opt_text(w, "CodUfficioPostale", &det.cod_ufficio_postale)?;
    write_opt_text(w, "CognomeQuietanzante", &det.cognome_quietanzante)?;
    write_opt_text(w, "NomeQuietanzante", &det.nome_quietanzante)?;
    write_opt_text(w, "CFQuietanzante", &det.cf_quietanzante)?;
    write_opt_text(w, "TitoloQuietanzante", &det.titolo_quietanzante)?;
    write_opt_text(w, "IstitutoFinanziario", &det.istituto_finanziario)?;
    write_opt_text(w, "IBAN", &det.iban)?;
    write_opt_text(w, "ABI", &det.abi)?;
    write_opt_text(w, "CAB", &det.cab)?;
    write_opt_text(w, "BIC", &det.bic)?;
    write_opt_amount2(w, "ScontoPagamentoAnticipato", det.sconto_pagamento_anticipato)?;
    write_opt_text(w, "DataLimitePagamentoAnticipato", &det.data_limite_pagamento_anticipato)?;
    write_opt_amount2(w, "PenalitaPagamentiRitardati", det.penalita_pagamenti_ritardati)?;
    write_opt_text(w, "DataDecorrenzaPenale", &det.data_decorrenza_penale)?;
    write_opt_text(w, "CodicePagamento", &det.codice_pagamento)?;

    w.write_event(Event::End(BytesEnd::new("DettaglioPagamento")))?;
    Ok(())
}

// --- Allegati ---

fn write_allegati<W: Write>(w: &mut Writer<W>, al: &Allegati) -> Result<(), XmlError> {
    w.write_event(Event::Start(BytesStart::new("Allegati")))?;
    write_text_elem(w, "NomeAttachment", &al.nome_attachment)?;
    write_opt_text(w, "AlgoritmoCompressione", &al.algoritmo_compressione)?;
    write_opt_text(w, "FormatoAttachment", &al.formato_attachment)?;
    write_opt_text(w, "DescrizioneAttachment", &al.descrizione_attachment)?;
    // bytes field: base64 encode
    if !al.attachment.is_empty() {
        let encoded = BASE64.encode(&al.attachment);
        write_text_elem(w, "Attachment", &encoded)?;
    }
    w.write_event(Event::End(BytesEnd::new("Allegati")))?;
    Ok(())
}

