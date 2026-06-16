use std::io::{BufRead, BufReader, Read};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use quick_xml::Reader;
use quick_xml::events::Event;

use crate::sdi::v1::*;
use super::XmlError;
use super::enum_map::*;

/// Decode an SDI XML string into a `FatturaElettronica` proto message.
///
/// Handles:
/// - PascalCase XML element names mapped to snake_case proto fields
/// - SDI enum string codes parsed to proto integer values
/// - `oneof` disambiguation (`Denominazione` vs `Nome`+`Cognome`)
/// - Base64-decoded attachment bytes
/// - Missing optional elements mapped to `None`
pub fn decode(xml: &str) -> Result<FatturaElettronica, XmlError> {
    decode_from_reader(xml.as_bytes())
}

/// Decode from a reader, avoiding loading the full XML into memory.
pub fn decode_from_reader<R: Read>(reader: R) -> Result<FatturaElettronica, XmlError> {
    let buf_reader = BufReader::new(reader);
    let mut r = Reader::from_reader(buf_reader);
    r.config_mut().trim_text(true);

    let mut buf = Vec::new();

    // Skip to root element
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                let name = local_name(&e);
                if name == "FatturaElettronica" {
                    return read_fattura(&mut r, &e);
                }
            }
            Event::Eof => return Err(XmlError::MissingElement("FatturaElettronica".into())),
            _ => {}
        }
        buf.clear();
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

/// Extract the local name from an element (strip namespace prefix).
fn local_name(e: &quick_xml::events::BytesStart) -> String {
    let full = String::from_utf8_lossy(e.name().as_ref()).to_string();
    full.split(':').last().unwrap_or(&full).to_string()
}

/// Read the text content of the current element. Assumes the reader is positioned
/// right after `Event::Start`. Consumes up to the matching `Event::End`.
fn read_text<R: BufRead>(r: &mut Reader<R>) -> Result<String, XmlError> {
    let mut buf = Vec::new();
    let mut text = String::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Text(e) => {
                text.push_str(&e.unescape()?.into_owned());
            }
            Event::CData(e) => {
                text.push_str(&String::from_utf8_lossy(&e));
            }
            Event::End(_) => return Ok(text),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF in text element".into())),
            _ => {}
        }
        buf.clear();
    }
}

/// Skip the current element and all its children.
fn skip_element<R: BufRead>(r: &mut Reader<R>) -> Result<(), XmlError> {
    let mut depth = 1u32;
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(_) => depth += 1,
            Event::End(_) => {
                depth -= 1;
                if depth == 0 { return Ok(()); }
            }
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF while skipping".into())),
            _ => {}
        }
        buf.clear();
    }
}

/// Parse a text element as f64.
fn parse_f64(text: &str) -> Result<f64, XmlError> {
    text.trim().parse::<f64>()
        .map_err(|e| XmlError::Deserialize(format!("Invalid number '{}': {}", text, e)))
}

/// Parse a text element as i32.
fn parse_i32(text: &str) -> Result<i32, XmlError> {
    text.trim().parse::<i32>()
        .map_err(|e| XmlError::Deserialize(format!("Invalid integer '{}': {}", text, e)))
}

/// Parse an SDI enum string to its proto integer value.
fn parse_enum(text: &str, enum_type: &str, from_sdi: fn(&str) -> Option<i32>) -> Result<i32, XmlError> {
    from_sdi(text.trim()).ok_or_else(|| XmlError::UnknownEnumValue {
        enum_type: enum_type.into(),
        value: text.into(),
    })
}

// ===========================================================================
// Root
// ===========================================================================

fn read_fattura<R: BufRead>(r: &mut Reader<R>, start: &quick_xml::events::BytesStart) -> Result<FatturaElettronica, XmlError> {
    let mut inv = FatturaElettronica::default();

    // Parse versione attribute
    for attr in start.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
        if key == "versione" {
            let val = attr.unescape_value()?.into_owned();
            inv.versione = parse_enum(&val, "FormatoTrasmissione", formato_trasmissione_from_sdi)?;
        }
    }

    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "FatturaElettronicaHeader" => inv.header = Some(read_header(r)?),
                    "FatturaElettronicaBody" => inv.body.push(read_body(r)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(inv),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF in FatturaElettronica".into())),
            _ => {}
        }
        buf.clear();
    }
}

// ===========================================================================
// Header
// ===========================================================================

fn read_header<R: BufRead>(r: &mut Reader<R>) -> Result<FatturaElettronicaHeader, XmlError> {
    let mut h = FatturaElettronicaHeader::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "DatiTrasmissione" => h.dati_trasmissione = Some(read_dati_trasmissione(r)?),
                    "CedentePrestatore" => h.cedente_prestatore = Some(read_cedente_prestatore(r)?),
                    "RappresentanteFiscale" => h.rappresentante_fiscale = Some(read_rappresentante_fiscale(r)?),
                    "CessionarioCommittente" => h.cessionario_committente = Some(read_cessionario_committente(r)?),
                    "TerzoIntermediarioOSoggettoEmittente" => h.terzo_intermediario_o_soggetto_emittente = Some(read_terzo_intermediario(r)?),
                    "SoggettoEmittente" => {
                        let text = read_text(r)?;
                        h.soggetto_emittente = Some(parse_enum(&text, "SoggettoEmittente", soggetto_emittente_from_sdi)?);
                    }
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(h),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF in Header".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_dati_trasmissione<R: BufRead>(r: &mut Reader<R>) -> Result<DatiTrasmissione, XmlError> {
    let mut dt = DatiTrasmissione::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "IdTrasmittente" => dt.id_trasmittente = Some(read_id_fiscale(r)?),
                    "ProgressivoInvio" => dt.progressivo_invio = read_text(r)?,
                    "FormatoTrasmissione" => {
                        let text = read_text(r)?;
                        dt.formato_trasmissione = parse_enum(&text, "FormatoTrasmissione", formato_trasmissione_from_sdi)?;
                    }
                    "CodiceDestinatario" => dt.codice_destinatario = read_text(r)?,
                    "ContattiTrasmittente" => dt.contatti_trasmittente = Some(read_contatti_trasmittente(r)?),
                    "PECDestinatario" => dt.pec_destinatario = Some(read_text(r)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(dt),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_id_fiscale<R: BufRead>(r: &mut Reader<R>) -> Result<IdFiscale, XmlError> {
    let mut id = IdFiscale::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "IdPaese" => id.id_paese = read_text(r)?,
                    "IdCodice" => id.id_codice = read_text(r)?,
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(id),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_contatti_trasmittente<R: BufRead>(r: &mut Reader<R>) -> Result<ContattiTrasmittente, XmlError> {
    let mut ct = ContattiTrasmittente::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "Telefono" => ct.telefono = Some(read_text(r)?),
                    "Email" => ct.email = Some(read_text(r)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(ct),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_cedente_prestatore<R: BufRead>(r: &mut Reader<R>) -> Result<CedentePrestatore, XmlError> {
    let mut cp = CedentePrestatore::default();
    let mut buf = Vec::new();
    let mut is_first_sede = true;
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "DatiAnagrafici" => cp.dati_anagrafici = Some(read_dati_anagrafici_cedente(r)?),
                    "Sede" if is_first_sede => { cp.sede = Some(read_indirizzo(r)?); is_first_sede = false; }
                    "StabileOrganizzazione" | "Sede" => cp.stabile_organizzazione = Some(read_indirizzo(r)?),
                    "IscrizioneREA" => cp.iscrizione_rea = Some(read_iscrizione_rea(r)?),
                    "Contatti" => cp.contatti = Some(read_contatti(r)?),
                    "RiferimentoAmministrazione" => cp.riferimento_amministrazione = Some(read_text(r)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(cp),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_dati_anagrafici_cedente<R: BufRead>(r: &mut Reader<R>) -> Result<DatiAnagraficiCedente, XmlError> {
    let mut da = DatiAnagraficiCedente::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "IdFiscaleIVA" => da.id_fiscale_iva = Some(read_id_fiscale(r)?),
                    "CodiceFiscale" => da.codice_fiscale = Some(read_text(r)?),
                    "Anagrafica" => da.anagrafica = Some(read_anagrafica(r)?),
                    "AlboProfessionale" => da.albo_professionale = Some(read_text(r)?),
                    "ProvinciaAlbo" => da.provincia_albo = Some(read_text(r)?),
                    "NumeroIscrizioneAlbo" => da.numero_iscrizione_albo = Some(read_text(r)?),
                    "DataIscrizioneAlbo" => da.data_iscrizione_albo = Some(read_text(r)?),
                    "RegimeFiscale" => {
                        let text = read_text(r)?;
                        da.regime_fiscale = parse_enum(&text, "RegimeFiscale", regime_fiscale_from_sdi)?;
                    }
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(da),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_anagrafica<R: BufRead>(r: &mut Reader<R>) -> Result<Anagrafica, XmlError> {
    let mut a = Anagrafica::default();
    let mut nome: Option<String> = None;
    let mut cognome: Option<String> = None;
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "Denominazione" => {
                        a.soggetto = Some(anagrafica::Soggetto::Denominazione(read_text(r)?));
                    }
                    "Nome" => nome = Some(read_text(r)?),
                    "Cognome" => cognome = Some(read_text(r)?),
                    "Titolo" => a.titolo = Some(read_text(r)?),
                    "CodEORI" => a.cod_eori = Some(read_text(r)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => {
                // If Nome+Cognome were present, build PersonaFisica
                if let (Some(n), Some(c)) = (nome, cognome) {
                    a.soggetto = Some(anagrafica::Soggetto::PersonaFisica(PersonaFisica {
                        nome: n,
                        cognome: c,
                    }));
                }
                return Ok(a);
            }
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_indirizzo<R: BufRead>(r: &mut Reader<R>) -> Result<Indirizzo, XmlError> {
    let mut addr = Indirizzo::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "Indirizzo" => addr.indirizzo = read_text(r)?,
                    "NumeroCivico" => addr.numero_civico = Some(read_text(r)?),
                    "CAP" => addr.cap = read_text(r)?,
                    "Comune" => addr.comune = read_text(r)?,
                    "Provincia" => addr.provincia = Some(read_text(r)?),
                    "Nazione" => addr.nazione = read_text(r)?,
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(addr),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_iscrizione_rea<R: BufRead>(r: &mut Reader<R>) -> Result<IscrizioneRea, XmlError> {
    let mut rea = IscrizioneRea::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "Ufficio" => rea.ufficio = read_text(r)?,
                    "NumeroREA" => rea.numero_rea = read_text(r)?,
                    "CapitaleSociale" => rea.capitale_sociale = Some(parse_f64(&read_text(r)?)?),
                    "SocioUnico" => rea.socio_unico = Some(parse_enum(&read_text(r)?, "SocioUnico", socio_unico_from_sdi)?),
                    "StatoLiquidazione" => rea.stato_liquidazione = parse_enum(&read_text(r)?, "StatoLiquidazione", stato_liquidazione_from_sdi)?,
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(rea),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_contatti<R: BufRead>(r: &mut Reader<R>) -> Result<Contatti, XmlError> {
    let mut c = Contatti::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "Telefono" => c.telefono = Some(read_text(r)?),
                    "Fax" => c.fax = Some(read_text(r)?),
                    "Email" => c.email = Some(read_text(r)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(c),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_rappresentante_fiscale<R: BufRead>(r: &mut Reader<R>) -> Result<RappresentanteFiscale, XmlError> {
    let mut rf = RappresentanteFiscale::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "DatiAnagrafici" => rf.dati_anagrafici = Some(read_dati_anagrafici_rappresentante(r)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(rf),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_dati_anagrafici_rappresentante<R: BufRead>(r: &mut Reader<R>) -> Result<DatiAnagraficiRappresentante, XmlError> {
    let mut da = DatiAnagraficiRappresentante::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "IdFiscaleIVA" => da.id_fiscale_iva = Some(read_id_fiscale(r)?),
                    "CodiceFiscale" => da.codice_fiscale = Some(read_text(r)?),
                    "Anagrafica" => da.anagrafica = Some(read_anagrafica(r)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(da),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_cessionario_committente<R: BufRead>(r: &mut Reader<R>) -> Result<CessionarioCommittente, XmlError> {
    let mut cc = CessionarioCommittente::default();
    let mut buf = Vec::new();
    let mut is_first_sede = true;
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "DatiAnagrafici" => cc.dati_anagrafici = Some(read_dati_anagrafici_cessionario(r)?),
                    "Sede" if is_first_sede => { cc.sede = Some(read_indirizzo(r)?); is_first_sede = false; }
                    "StabileOrganizzazione" | "Sede" => cc.stabile_organizzazione = Some(read_indirizzo(r)?),
                    "RappresentanteFiscale" => cc.rappresentante_fiscale = Some(read_rappresentante_fiscale_cessionario(r)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(cc),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_dati_anagrafici_cessionario<R: BufRead>(r: &mut Reader<R>) -> Result<DatiAnagraficiCessionario, XmlError> {
    let mut da = DatiAnagraficiCessionario::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "IdFiscaleIVA" => da.id_fiscale_iva = Some(read_id_fiscale(r)?),
                    "CodiceFiscale" => da.codice_fiscale = Some(read_text(r)?),
                    "Anagrafica" => da.anagrafica = Some(read_anagrafica(r)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(da),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_rappresentante_fiscale_cessionario<R: BufRead>(r: &mut Reader<R>) -> Result<RappresentanteFiscaleCessionario, XmlError> {
    let mut rf = RappresentanteFiscaleCessionario::default();
    let mut nome: Option<String> = None;
    let mut cognome: Option<String> = None;
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "IdFiscaleIVA" => rf.id_fiscale_iva = Some(read_id_fiscale(r)?),
                    "Denominazione" => {
                        rf.soggetto = Some(rappresentante_fiscale_cessionario::Soggetto::Denominazione(read_text(r)?));
                    }
                    "Nome" => nome = Some(read_text(r)?),
                    "Cognome" => cognome = Some(read_text(r)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => {
                if let (Some(n), Some(c)) = (nome, cognome) {
                    rf.soggetto = Some(rappresentante_fiscale_cessionario::Soggetto::PersonaFisica(PersonaFisica {
                        nome: n,
                        cognome: c,
                    }));
                }
                return Ok(rf);
            }
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_terzo_intermediario<R: BufRead>(r: &mut Reader<R>) -> Result<TerzoIntermediarioOSoggettoEmittente, XmlError> {
    let mut ti = TerzoIntermediarioOSoggettoEmittente::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "DatiAnagrafici" => ti.dati_anagrafici = Some(read_dati_anagrafici_terzo(r)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(ti),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_dati_anagrafici_terzo<R: BufRead>(r: &mut Reader<R>) -> Result<DatiAnagraficiTerzoIntermediario, XmlError> {
    let mut da = DatiAnagraficiTerzoIntermediario::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "IdFiscaleIVA" => da.id_fiscale_iva = Some(read_id_fiscale(r)?),
                    "CodiceFiscale" => da.codice_fiscale = Some(read_text(r)?),
                    "Anagrafica" => da.anagrafica = Some(read_anagrafica(r)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(da),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

// ===========================================================================
// Body
// ===========================================================================

fn read_body<R: BufRead>(r: &mut Reader<R>) -> Result<FatturaElettronicaBody, XmlError> {
    let mut body = FatturaElettronicaBody::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "DatiGenerali" => body.dati_generali = Some(read_dati_generali(r)?),
                    "DatiBeniServizi" => body.dati_beni_servizi = Some(read_dati_beni_servizi(r)?),
                    "DatiVeicoli" => body.dati_veicoli = Some(read_dati_veicoli(r)?),
                    "DatiPagamento" => body.dati_pagamento.push(read_dati_pagamento(r)?),
                    "Allegati" => body.allegati.push(read_allegati(r)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(body),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_dati_generali<R: BufRead>(r: &mut Reader<R>) -> Result<DatiGenerali, XmlError> {
    let mut dg = DatiGenerali::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "DatiGeneraliDocumento" => dg.dati_generali_documento = Some(read_dati_generali_documento(r)?),
                    "DatiOrdineAcquisto" => dg.dati_ordine_acquisto.push(read_dati_documenti_correlati(r)?),
                    "DatiContratto" => dg.dati_contratto.push(read_dati_documenti_correlati(r)?),
                    "DatiConvenzione" => dg.dati_convenzione.push(read_dati_documenti_correlati(r)?),
                    "DatiRicezione" => dg.dati_ricezione.push(read_dati_documenti_correlati(r)?),
                    "DatiFattureCollegate" => dg.dati_fatture_collegate.push(read_dati_documenti_correlati(r)?),
                    "DatiSAL" => dg.dati_sal.push(read_dati_sal(r)?),
                    "DatiDDT" => dg.dati_ddt.push(read_dati_ddt(r)?),
                    "DatiTrasporto" => dg.dati_trasporto = Some(read_dati_trasporto(r)?),
                    "FatturaPrincipale" => dg.fattura_principale = Some(read_fattura_principale(r)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(dg),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_dati_generali_documento<R: BufRead>(r: &mut Reader<R>) -> Result<DatiGeneraliDocumento, XmlError> {
    let mut dgd = DatiGeneraliDocumento::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "TipoDocumento" => dgd.tipo_documento = parse_enum(&read_text(r)?, "TipoDocumento", tipo_documento_from_sdi)?,
                    "Divisa" => dgd.divisa = read_text(r)?,
                    "Data" => dgd.data = read_text(r)?,
                    "Numero" => dgd.numero = read_text(r)?,
                    "DatiRitenuta" => dgd.dati_ritenuta.push(read_dati_ritenuta(r)?),
                    "DatiBollo" => dgd.dati_bollo = Some(read_dati_bollo(r)?),
                    "DatiCassaPrevidenziale" => dgd.dati_cassa_previdenziale.push(read_dati_cassa_previdenziale(r)?),
                    "ScontoMaggiorazione" => dgd.sconto_maggiorazione.push(read_sconto_maggiorazione(r)?),
                    "ImportoTotaleDocumento" => dgd.importo_totale_documento = Some(parse_f64(&read_text(r)?)?),
                    "Arrotondamento" => dgd.arrotondamento = Some(parse_f64(&read_text(r)?)?),
                    "Causale" => dgd.causale.push(read_text(r)?),
                    "Art73" => dgd.art73 = Some(parse_enum(&read_text(r)?, "Art73", art73_from_sdi)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(dgd),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_dati_ritenuta<R: BufRead>(r: &mut Reader<R>) -> Result<DatiRitenuta, XmlError> {
    let mut dr = DatiRitenuta::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "TipoRitenuta" => dr.tipo_ritenuta = parse_enum(&read_text(r)?, "TipoRitenuta", tipo_ritenuta_from_sdi)?,
                    "ImportoRitenuta" => dr.importo_ritenuta = parse_f64(&read_text(r)?)?,
                    "AliquotaRitenuta" => dr.aliquota_ritenuta = parse_f64(&read_text(r)?)?,
                    "CausalePagamento" => dr.causale_pagamento = parse_enum(&read_text(r)?, "CausalePagamento", causale_pagamento_from_sdi)?,
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(dr),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_dati_bollo<R: BufRead>(r: &mut Reader<R>) -> Result<DatiBollo, XmlError> {
    let mut db = DatiBollo::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "BolloVirtuale" => db.bollo_virtuale = parse_enum(&read_text(r)?, "BolloVirtuale", bollo_virtuale_from_sdi)?,
                    "ImportoBollo" => db.importo_bollo = Some(parse_f64(&read_text(r)?)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(db),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_dati_cassa_previdenziale<R: BufRead>(r: &mut Reader<R>) -> Result<DatiCassaPrevidenziale, XmlError> {
    let mut dcp = DatiCassaPrevidenziale::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "TipoCassa" => dcp.tipo_cassa = parse_enum(&read_text(r)?, "TipoCassa", tipo_cassa_from_sdi)?,
                    "AlCassa" => dcp.al_cassa = parse_f64(&read_text(r)?)?,
                    "ImportoContributoCassa" => dcp.importo_contributo_cassa = parse_f64(&read_text(r)?)?,
                    "ImponibileCassa" => dcp.imponibile_cassa = Some(parse_f64(&read_text(r)?)?),
                    "AliquotaIVA" => dcp.aliquota_iva = parse_f64(&read_text(r)?)?,
                    "Ritenuta" => dcp.ritenuta = Some(parse_enum(&read_text(r)?, "Ritenuta", ritenuta_from_sdi)?),
                    "Natura" => dcp.natura = Some(parse_enum(&read_text(r)?, "Natura", natura_from_sdi)?),
                    "RiferimentoAmministrazione" => dcp.riferimento_amministrazione = Some(read_text(r)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(dcp),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_sconto_maggiorazione<R: BufRead>(r: &mut Reader<R>) -> Result<ScontoMaggiorazione, XmlError> {
    let mut sm = ScontoMaggiorazione::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "Tipo" => sm.tipo = parse_enum(&read_text(r)?, "TipoScontoMaggiorazione", tipo_sconto_maggiorazione_from_sdi)?,
                    "Percentuale" => sm.percentuale = Some(parse_f64(&read_text(r)?)?),
                    "Importo" => sm.importo = Some(parse_f64(&read_text(r)?)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(sm),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_dati_documenti_correlati<R: BufRead>(r: &mut Reader<R>) -> Result<DatiDocumentiCorrelati, XmlError> {
    let mut d = DatiDocumentiCorrelati::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "RiferimentoNumeroLinea" => d.riferimento_numero_linea.push(parse_i32(&read_text(r)?)?),
                    "IdDocumento" => d.id_documento = read_text(r)?,
                    "Data" => d.data = Some(read_text(r)?),
                    "NumItem" => d.num_item = Some(read_text(r)?),
                    "CodiceCommessaConvenzione" => d.codice_commessa_convenzione = Some(read_text(r)?),
                    "CodiceCUP" => d.codice_cup = Some(read_text(r)?),
                    "CodiceCIG" => d.codice_cig = Some(read_text(r)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(d),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_dati_sal<R: BufRead>(r: &mut Reader<R>) -> Result<DatiSal, XmlError> {
    let mut sal = DatiSal::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "RiferimentoFase" => sal.riferimento_fase = parse_i32(&read_text(r)?)?,
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(sal),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_dati_ddt<R: BufRead>(r: &mut Reader<R>) -> Result<DatiDdt, XmlError> {
    let mut ddt = DatiDdt::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "NumeroDDT" => ddt.numero_ddt = read_text(r)?,
                    "DataDDT" => ddt.data_ddt = read_text(r)?,
                    "RiferimentoNumeroLinea" => ddt.riferimento_numero_linea.push(parse_i32(&read_text(r)?)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(ddt),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_dati_trasporto<R: BufRead>(r: &mut Reader<R>) -> Result<DatiTrasporto, XmlError> {
    let mut dt = DatiTrasporto::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "DatiAnagraficiVettore" => dt.dati_anagrafici_vettore = Some(read_dati_anagrafici_vettore(r)?),
                    "MezzoTrasporto" => dt.mezzo_trasporto = Some(read_text(r)?),
                    "CausaleTrasporto" => dt.causale_trasporto = Some(read_text(r)?),
                    "NumeroColli" => dt.numero_colli = Some(parse_i32(&read_text(r)?)?),
                    "Descrizione" => dt.descrizione = Some(read_text(r)?),
                    "UnitaMisuraPeso" => dt.unita_misura_peso = Some(read_text(r)?),
                    "PesoLordo" => dt.peso_lordo = Some(parse_f64(&read_text(r)?)?),
                    "PesoNetto" => dt.peso_netto = Some(parse_f64(&read_text(r)?)?),
                    "DataOraRitiro" => dt.data_ora_ritiro = Some(read_text(r)?),
                    "DataInizioTrasporto" => dt.data_inizio_trasporto = Some(read_text(r)?),
                    "TipoResa" => dt.tipo_resa = Some(read_text(r)?),
                    "IndirizzoResa" => dt.indirizzo_resa = Some(read_indirizzo(r)?),
                    "DataOraConsegna" => dt.data_ora_consegna = Some(read_text(r)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(dt),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_dati_anagrafici_vettore<R: BufRead>(r: &mut Reader<R>) -> Result<DatiAnagraficiVettore, XmlError> {
    let mut dav = DatiAnagraficiVettore::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "IdFiscaleIVA" => dav.id_fiscale_iva = Some(read_id_fiscale(r)?),
                    "CodiceFiscale" => dav.codice_fiscale = Some(read_text(r)?),
                    "Anagrafica" => dav.anagrafica = Some(read_anagrafica(r)?),
                    "NumeroLicenzaGuida" => dav.numero_licenza_guida = Some(read_text(r)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(dav),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_fattura_principale<R: BufRead>(r: &mut Reader<R>) -> Result<FatturaPrincipale, XmlError> {
    let mut fp = FatturaPrincipale::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "NumeroFatturaPrincipale" => fp.numero_fattura_principale = read_text(r)?,
                    "DataFatturaPrincipale" => fp.data_fattura_principale = read_text(r)?,
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(fp),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_dati_beni_servizi<R: BufRead>(r: &mut Reader<R>) -> Result<DatiBeniServizi, XmlError> {
    let mut dbs = DatiBeniServizi::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "DettaglioLinee" => dbs.dettaglio_linee.push(read_dettaglio_linee(r)?),
                    "DatiRiepilogo" => dbs.dati_riepilogo.push(read_dati_riepilogo(r)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(dbs),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_dettaglio_linee<R: BufRead>(r: &mut Reader<R>) -> Result<DettaglioLinee, XmlError> {
    let mut dl = DettaglioLinee::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "NumeroLinea" => dl.numero_linea = parse_i32(&read_text(r)?)?,
                    "TipoCessionePrestazione" => dl.tipo_cessione_prestazione = Some(parse_enum(&read_text(r)?, "TipoCessionePrestazione", tipo_cessione_prestazione_from_sdi)?),
                    "CodiceArticolo" => dl.codice_articolo.push(read_codice_articolo(r)?),
                    "Descrizione" => dl.descrizione = read_text(r)?,
                    "Quantita" => dl.quantita = Some(parse_f64(&read_text(r)?)?),
                    "UnitaMisura" => dl.unita_misura = Some(read_text(r)?),
                    "DataInizioPeriodo" => dl.data_inizio_periodo = Some(read_text(r)?),
                    "DataFinePeriodo" => dl.data_fine_periodo = Some(read_text(r)?),
                    "PrezzoUnitario" => dl.prezzo_unitario = parse_f64(&read_text(r)?)?,
                    "ScontoMaggiorazione" => dl.sconto_maggiorazione.push(read_sconto_maggiorazione(r)?),
                    "PrezzoTotale" => dl.prezzo_totale = parse_f64(&read_text(r)?)?,
                    "AliquotaIVA" => dl.aliquota_iva = parse_f64(&read_text(r)?)?,
                    "Ritenuta" => dl.ritenuta = Some(parse_enum(&read_text(r)?, "Ritenuta", ritenuta_from_sdi)?),
                    "Natura" => dl.natura = Some(parse_enum(&read_text(r)?, "Natura", natura_from_sdi)?),
                    "RiferimentoAmministrazione" => dl.riferimento_amministrazione = Some(read_text(r)?),
                    "AltriDatiGestionali" => dl.altri_dati_gestionali.push(read_altri_dati_gestionali(r)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(dl),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_codice_articolo<R: BufRead>(r: &mut Reader<R>) -> Result<CodiceArticolo, XmlError> {
    let mut ca = CodiceArticolo::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "CodiceTipo" => ca.codice_tipo = read_text(r)?,
                    "CodiceValore" => ca.codice_valore = read_text(r)?,
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(ca),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_altri_dati_gestionali<R: BufRead>(r: &mut Reader<R>) -> Result<AltriDatiGestionali, XmlError> {
    let mut adg = AltriDatiGestionali::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "TipoDato" => adg.tipo_dato = read_text(r)?,
                    "RiferimentoTesto" => adg.riferimento_testo = Some(read_text(r)?),
                    "RiferimentoNumero" => adg.riferimento_numero = Some(parse_f64(&read_text(r)?)?),
                    "RiferimentoData" => adg.riferimento_data = Some(read_text(r)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(adg),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_dati_riepilogo<R: BufRead>(r: &mut Reader<R>) -> Result<DatiRiepilogo, XmlError> {
    let mut dr = DatiRiepilogo::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "AliquotaIVA" => dr.aliquota_iva = parse_f64(&read_text(r)?)?,
                    "Natura" => dr.natura = Some(parse_enum(&read_text(r)?, "Natura", natura_from_sdi)?),
                    "SpeseAccessorie" => dr.spese_accessorie = Some(parse_f64(&read_text(r)?)?),
                    "Arrotondamento" => dr.arrotondamento = Some(parse_f64(&read_text(r)?)?),
                    "ImponibileImporto" => dr.imponibile_importo = parse_f64(&read_text(r)?)?,
                    "Imposta" => dr.imposta = parse_f64(&read_text(r)?)?,
                    "EsigibilitaIVA" => dr.esigibilita_iva = Some(parse_enum(&read_text(r)?, "EsigibilitaIVA", esigibilita_iva_from_sdi)?),
                    "RiferimentoNormativo" => dr.riferimento_normativo = Some(read_text(r)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(dr),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_dati_veicoli<R: BufRead>(r: &mut Reader<R>) -> Result<DatiVeicoli, XmlError> {
    let mut dv = DatiVeicoli::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "Data" => dv.data = read_text(r)?,
                    "TotalePercorso" => dv.totale_percorso = read_text(r)?,
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(dv),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_dati_pagamento<R: BufRead>(r: &mut Reader<R>) -> Result<DatiPagamento, XmlError> {
    let mut dp = DatiPagamento::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "CondizioniPagamento" => dp.condizioni_pagamento = parse_enum(&read_text(r)?, "CondizioniPagamento", condizioni_pagamento_from_sdi)?,
                    "DettaglioPagamento" => dp.dettaglio_pagamento.push(read_dettaglio_pagamento(r)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(dp),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_dettaglio_pagamento<R: BufRead>(r: &mut Reader<R>) -> Result<DettaglioPagamento, XmlError> {
    let mut det = DettaglioPagamento::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "Beneficiario" => det.beneficiario = Some(read_text(r)?),
                    "ModalitaPagamento" => det.modalita_pagamento = parse_enum(&read_text(r)?, "ModalitaPagamento", modalita_pagamento_from_sdi)?,
                    "DataRiferimentoTerminiPagamento" => det.data_riferimento_termini_pagamento = Some(read_text(r)?),
                    "GiorniTerminiPagamento" => det.giorni_termini_pagamento = Some(parse_i32(&read_text(r)?)?),
                    "DataScadenzaPagamento" => det.data_scadenza_pagamento = Some(read_text(r)?),
                    "ImportoPagamento" => det.importo_pagamento = parse_f64(&read_text(r)?)?,
                    "CodUfficioPostale" => det.cod_ufficio_postale = Some(read_text(r)?),
                    "CognomeQuietanzante" => det.cognome_quietanzante = Some(read_text(r)?),
                    "NomeQuietanzante" => det.nome_quietanzante = Some(read_text(r)?),
                    "CFQuietanzante" => det.cf_quietanzante = Some(read_text(r)?),
                    "TitoloQuietanzante" => det.titolo_quietanzante = Some(read_text(r)?),
                    "IstitutoFinanziario" => det.istituto_finanziario = Some(read_text(r)?),
                    "IBAN" => det.iban = Some(read_text(r)?),
                    "ABI" => det.abi = Some(read_text(r)?),
                    "CAB" => det.cab = Some(read_text(r)?),
                    "BIC" => det.bic = Some(read_text(r)?),
                    "ScontoPagamentoAnticipato" => det.sconto_pagamento_anticipato = Some(parse_f64(&read_text(r)?)?),
                    "DataLimitePagamentoAnticipato" => det.data_limite_pagamento_anticipato = Some(read_text(r)?),
                    "PenalitaPagamentiRitardati" => det.penalita_pagamenti_ritardati = Some(parse_f64(&read_text(r)?)?),
                    "DataDecorrenzaPenale" => det.data_decorrenza_penale = Some(read_text(r)?),
                    "CodicePagamento" => det.codice_pagamento = Some(read_text(r)?),
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(det),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_allegati<R: BufRead>(r: &mut Reader<R>) -> Result<Allegati, XmlError> {
    let mut al = Allegati::default();
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) => {
                match local_name(&e).as_str() {
                    "NomeAttachment" => al.nome_attachment = read_text(r)?,
                    "AlgoritmoCompressione" => al.algoritmo_compressione = Some(read_text(r)?),
                    "FormatoAttachment" => al.formato_attachment = Some(read_text(r)?),
                    "DescrizioneAttachment" => al.descrizione_attachment = Some(read_text(r)?),
                    "Attachment" => {
                        let b64 = read_text(r)?;
                        al.attachment = BASE64.decode(b64.trim().as_bytes())
                            .map_err(|e| XmlError::Deserialize(format!("Invalid base64 in Attachment: {}", e)))?;
                    }
                    _ => skip_element(r)?,
                }
            }
            Event::End(_) => return Ok(al),
            Event::Eof => return Err(XmlError::Deserialize("Unexpected EOF".into())),
            _ => {}
        }
        buf.clear();
    }
}
