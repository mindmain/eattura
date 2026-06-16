//! Shared PEC/SDI helpers used by both the server and the desktop app.
//!
//! This module is transport-agnostic: it knows how to name an outgoing invoice
//! file per SDI conventions and how to parse the notification messages SDI
//! returns over PEC. The actual SMTP/IMAP transport lives in each backend.

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::invoice_status::InvoiceStatus;
use crate::sdi::v1::FatturaElettronica;
use crate::xml::XmlError;

/// The PEC address of the SDI receiving system for private/B2B invoices.
pub const SDI_PEC_ADDRESS: &str = "sdi01@pec.fatturapa.it";

/// Build the SDI-compliant file name for an invoice XML.
///
/// Format: `{IdPaese}{IdCodice}_{Progressivo}.xml` — e.g. `IT01234567890_00001.xml`.
/// The progressivo is taken from `DatiTrasmissione.ProgressivoInvio`.
pub fn sdi_filename(fattura: &FatturaElettronica) -> String {
    let dt = fattura
        .header
        .as_ref()
        .and_then(|h| h.dati_trasmissione.as_ref());

    let (id_paese, id_codice) = dt
        .and_then(|d| d.id_trasmittente.as_ref())
        .map(|id| (id.id_paese.as_str(), id.id_codice.as_str()))
        .unwrap_or(("IT", "00000000000"));

    let progressivo = dt
        .map(|d| d.progressivo_invio.as_str())
        .filter(|p| !p.is_empty())
        .unwrap_or("00001");

    format!("{id_paese}{id_codice}_{progressivo}.xml")
}

/// Derive a stable, unique `ProgressivoInvio` (max 10 alphanumeric chars) from
/// an invoice id, so distinct invoices never collide on the SDI file name.
pub fn progressivo_from_id(id: &str) -> String {
    let alnum: String = id.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    let tail: String = alnum
        .chars()
        .rev()
        .take(5)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{:0>5}", tail.to_ascii_uppercase())
}

/// The category of an SDI notification, identified by the XML root element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SdiMessageType {
    /// RC — RicevutaConsegna: the invoice was delivered to the recipient.
    RicevutaConsegna,
    /// NS — NotificaScarto: the invoice was rejected by SDI.
    NotificaScarto,
    /// MC — NotificaMancataConsegna: SDI accepted but could not deliver yet.
    NotificaMancataConsegna,
    /// NE — NotificaEsito: the recipient (PA) returned an acceptance/refusal.
    NotificaEsito,
    /// DT — NotificaDecorrenzaTermini: delivery window elapsed (deemed accepted).
    NotificaDecorrenzaTermini,
    /// AT — AttestazioneTrasmissioneFattura: transmission attestation.
    AttestazioneTrasmissione,
    /// MT — MetadatiInvioFile: file metadata.
    MetadatiInvio,
    /// Any other / unrecognized root element.
    Unknown(String),
}

impl SdiMessageType {
    fn from_root(name: &str) -> Self {
        match name {
            "RicevutaConsegna" => SdiMessageType::RicevutaConsegna,
            "NotificaScarto" => SdiMessageType::NotificaScarto,
            "NotificaMancataConsegna" => SdiMessageType::NotificaMancataConsegna,
            "NotificaEsito" => SdiMessageType::NotificaEsito,
            "NotificaDecorrenzaTermini" => SdiMessageType::NotificaDecorrenzaTermini,
            "AttestazioneTrasmissioneFattura" => SdiMessageType::AttestazioneTrasmissione,
            "MetadatiInvioFile" => SdiMessageType::MetadatiInvio,
            other => SdiMessageType::Unknown(other.to_string()),
        }
    }

    /// The two-letter SDI message code (RC, NS, MC, NE, DT, AT, MT).
    pub fn code(&self) -> &str {
        match self {
            SdiMessageType::RicevutaConsegna => "RC",
            SdiMessageType::NotificaScarto => "NS",
            SdiMessageType::NotificaMancataConsegna => "MC",
            SdiMessageType::NotificaEsito => "NE",
            SdiMessageType::NotificaDecorrenzaTermini => "DT",
            SdiMessageType::AttestazioneTrasmissione => "AT",
            SdiMessageType::MetadatiInvio => "MT",
            SdiMessageType::Unknown(_) => "??",
        }
    }
}

/// A single error entry inside a NotificaScarto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdiError {
    pub codice: String,
    pub descrizione: String,
}

/// A parsed SDI notification with the fields relevant to invoice lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedNotification {
    pub message_type: SdiMessageType,
    /// SDI's internal identifier for the transmitted file.
    pub identificativo_sdi: Option<String>,
    /// The original invoice file name this notification refers to.
    pub nome_file: Option<String>,
    /// For NotificaEsito: the recipient outcome code (EC01 = accept, EC02 = refuse).
    pub esito: Option<String>,
    /// For NotificaScarto: the list of validation errors.
    pub errors: Vec<SdiError>,
}

impl ParsedNotification {
    /// The invoice status this notification implies, if it warrants a change.
    ///
    /// Returns `None` for purely informational messages (AT/MT/MC) that do not
    /// move the invoice out of `sent`.
    pub fn resulting_status(&self) -> Option<InvoiceStatus> {
        match &self.message_type {
            SdiMessageType::RicevutaConsegna
            | SdiMessageType::NotificaDecorrenzaTermini => Some(InvoiceStatus::Accepted),
            SdiMessageType::NotificaScarto => Some(InvoiceStatus::Rejected),
            SdiMessageType::NotificaEsito => match self.esito.as_deref() {
                Some("EC01") => Some(InvoiceStatus::Accepted),
                Some("EC02") => Some(InvoiceStatus::Rejected),
                _ => None,
            },
            _ => None,
        }
    }
}

/// Parse an SDI notification XML into a [`ParsedNotification`].
///
/// Identifies the message by its root element, then scans for the common
/// fields (`IdentificativoSdI`, `NomeFile`, `Esito`) and, for a scarto, the
/// `ListaErrori/Errore` entries. Namespaces and unknown elements are ignored.
pub fn parse_notification(xml: &str) -> Result<ParsedNotification, XmlError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut message_type: Option<SdiMessageType> = None;
    let mut identificativo_sdi = None;
    let mut nome_file = None;
    let mut esito = None;
    let mut errors: Vec<SdiError> = Vec::new();

    // Scratch state for the current Errore being read.
    let mut cur_codice: Option<String> = None;
    let mut cur_descrizione: Option<String> = None;
    let mut in_errore = false;

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => {
                let name = local_name(e.name().as_ref());
                if message_type.is_none() {
                    message_type = Some(SdiMessageType::from_root(&name));
                }
                match name.as_str() {
                    "Errore" => {
                        in_errore = true;
                        cur_codice = None;
                        cur_descrizione = None;
                    }
                    "IdentificativoSdI" => {
                        identificativo_sdi = Some(read_text(&mut reader)?);
                    }
                    "NomeFile" if nome_file.is_none() => {
                        nome_file = Some(read_text(&mut reader)?);
                    }
                    "Esito" => {
                        esito = Some(read_text(&mut reader)?);
                    }
                    "Codice" if in_errore => {
                        cur_codice = Some(read_text(&mut reader)?);
                    }
                    "Descrizione" if in_errore => {
                        cur_descrizione = Some(read_text(&mut reader)?);
                    }
                    _ => {}
                }
            }
            Event::End(e) => {
                if local_name(e.name().as_ref()) == "Errore" && in_errore {
                    errors.push(SdiError {
                        codice: cur_codice.take().unwrap_or_default(),
                        descrizione: cur_descrizione.take().unwrap_or_default(),
                    });
                    in_errore = false;
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    let message_type = message_type
        .ok_or_else(|| XmlError::MissingElement("notification root element".into()))?;

    Ok(ParsedNotification {
        message_type,
        identificativo_sdi,
        nome_file,
        esito,
        errors,
    })
}

/// Strip an optional namespace prefix from an element name.
fn local_name(raw: &[u8]) -> String {
    let full = String::from_utf8_lossy(raw);
    full.rsplit(':').next().unwrap_or(&full).to_string()
}

/// Read the text content of the current element up to its matching end tag.
fn read_text(reader: &mut Reader<&[u8]>) -> Result<String, XmlError> {
    let mut buf = Vec::new();
    let mut text = String::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Text(e) => text.push_str(&e.unescape()?),
            Event::CData(e) => text.push_str(&String::from_utf8_lossy(&e)),
            Event::End(_) | Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ricevuta_consegna() {
        let xml = r#"<?xml version="1.0"?>
            <RicevutaConsegna versione="1.0">
                <IdentificativoSdI>123456</IdentificativoSdI>
                <NomeFile>IT01234567890_00001.xml</NomeFile>
                <DataOraRicezione>2026-06-16T10:00:00.000+02:00</DataOraRicezione>
            </RicevutaConsegna>"#;
        let n = parse_notification(xml).unwrap();
        assert_eq!(n.message_type, SdiMessageType::RicevutaConsegna);
        assert_eq!(n.identificativo_sdi.as_deref(), Some("123456"));
        assert_eq!(n.nome_file.as_deref(), Some("IT01234567890_00001.xml"));
        assert_eq!(n.resulting_status(), Some(InvoiceStatus::Accepted));
    }

    #[test]
    fn parse_notifica_scarto_with_errors() {
        let xml = r#"<NotificaScarto>
                <IdentificativoSdI>999</IdentificativoSdI>
                <NomeFile>IT01234567890_00002.xml</NomeFile>
                <ListaErrori>
                    <Errore>
                        <Codice>00305</Codice>
                        <Descrizione>CodiceDestinatario non valido</Descrizione>
                    </Errore>
                    <Errore>
                        <Codice>00400</Codice>
                        <Descrizione>Natura mancante</Descrizione>
                    </Errore>
                </ListaErrori>
            </NotificaScarto>"#;
        let n = parse_notification(xml).unwrap();
        assert_eq!(n.message_type, SdiMessageType::NotificaScarto);
        assert_eq!(n.errors.len(), 2);
        assert_eq!(n.errors[0].codice, "00305");
        assert_eq!(n.errors[1].descrizione, "Natura mancante");
        assert_eq!(n.resulting_status(), Some(InvoiceStatus::Rejected));
    }

    #[test]
    fn parse_notifica_esito_accept_and_refuse() {
        let accept = r#"<NotificaEsito><IdentificativoSdI>1</IdentificativoSdI><Esito>EC01</Esito></NotificaEsito>"#;
        assert_eq!(
            parse_notification(accept).unwrap().resulting_status(),
            Some(InvoiceStatus::Accepted)
        );
        let refuse = r#"<NotificaEsito><IdentificativoSdI>1</IdentificativoSdI><Esito>EC02</Esito></NotificaEsito>"#;
        assert_eq!(
            parse_notification(refuse).unwrap().resulting_status(),
            Some(InvoiceStatus::Rejected)
        );
    }

    #[test]
    fn parse_mancata_consegna_no_status_change() {
        let xml = r#"<NotificaMancataConsegna><IdentificativoSdI>7</IdentificativoSdI></NotificaMancataConsegna>"#;
        let n = parse_notification(xml).unwrap();
        assert_eq!(n.message_type, SdiMessageType::NotificaMancataConsegna);
        assert_eq!(n.resulting_status(), None);
    }

    #[test]
    fn progressivo_is_unique_and_alphanumeric() {
        let a = progressivo_from_id("550e8400-e29b-41d4-a716-446655440000");
        let b = progressivo_from_id("550e8400-e29b-41d4-a716-446655440001");
        assert_eq!(a.len(), 5);
        assert_ne!(a, b);
        assert!(a.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn handles_namespaced_root() {
        let xml = r#"<ns:RicevutaConsegna xmlns:ns="urn:test"><ns:IdentificativoSdI>5</ns:IdentificativoSdI></ns:RicevutaConsegna>"#;
        let n = parse_notification(xml).unwrap();
        assert_eq!(n.message_type, SdiMessageType::RicevutaConsegna);
        assert_eq!(n.identificativo_sdi.as_deref(), Some("5"));
    }
}
