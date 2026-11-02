//! PEC transport for the desktop app (SMTP send + IMAP receive).
//!
//! Mirrors the server's transport but follows the desktop convention of
//! `Result<_, String>` errors. The non-secret connection parameters come from
//! the app settings; the password is read from the OS keyring.

use common::pec::{extract_sdi_pec_addresses, select_preferred_sdi_address};
use lettre::message::{Attachment, MultiPart, SinglePart, header::ContentType};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use mail_parser::MimeHeaders;

/// Keyring service name under which PEC passwords are stored.
const KEYRING_SERVICE: &str = "eattura-pec";

/// Connection parameters and credentials for a PEC mailbox.
#[derive(Debug, Clone)]
pub struct PecConfig {
    pub email: String,
    pub sdi_address: String,
    pub password: String,
    pub imap_host: String,
    pub imap_port: u16,
    pub smtp_host: String,
    pub smtp_port: u16,
}

/// A message fetched from the PEC inbox.
#[derive(Debug)]
#[allow(dead_code)] // subject/from are part of the message model, used for logging/diagnostics
pub struct PecMessage {
    pub uid: u32,
    pub subject: String,
    pub from: String,
    /// SDI PEC address (`*@pec.fatturapa.it`) detected from message sender headers or `daticert.xml`.
    pub sdi_sender: Option<String>,
    /// Fallback SDI PEC address found anywhere in headers/body if the message carries an SDI notification.
    pub sdi_fallback: Option<String>,
    pub attachments: Vec<PecAttachment>,
}

/// A consistent IMAP scan with the mailbox identity needed for incremental sync.
pub struct PecInbox {
    pub uid_validity: Option<u32>,
    pub last_uid: u32,
    pub messages: Vec<PecMessage>,
}

/// An attachment of a PEC message (typically an XML notification from SDI).
#[derive(Debug)]
pub struct PecAttachment {
    pub filename: String,
    pub data: Vec<u8>,
}

/// Store the PEC password for `email` in the OS keyring.
pub fn store_password(email: &str, password: &str) -> Result<(), String> {
    keyring::Entry::new(KEYRING_SERVICE, email)
        .and_then(|e| e.set_password(password))
        .map_err(|e| format!("Keyring write failed: {e}"))
}

/// Read the PEC password for `email` from the OS keyring.
pub fn load_password(email: &str) -> Result<String, String> {
    keyring::Entry::new(KEYRING_SERVICE, email)
        .and_then(|e| e.get_password())
        .map_err(|e| format!("Keyring read failed (no PEC password stored?): {e}"))
}

/// Send an invoice XML via PEC to the SDI endpoint, returning the SMTP response.
pub async fn send_invoice(config: &PecConfig, xml: &str, filename: &str) -> Result<String, String> {
    let email = build_invoice_message(config, xml, filename)?;

    let creds = Credentials::new(config.email.clone(), config.password.clone());
    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)
            .map_err(|e| format!("SMTP setup failed: {e}"))?
            .port(config.smtp_port)
            .credentials(creds)
            .build();

    let response = mailer
        .send(email)
        .await
        .map_err(|e| format!("PEC send failed: {e}"))?;
    Ok(response
        .message()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string())
}

fn build_invoice_message(config: &PecConfig, xml: &str, filename: &str) -> Result<Message, String> {
    let content_type = ContentType::parse("application/xml")
        .map_err(|e| format!("Invalid XML content type: {e}"))?;
    let xml_part =
        Attachment::new(filename.to_string()).body(xml.as_bytes().to_vec(), content_type);

    let sdi_recipient = config.sdi_address.trim();
    let sdi_recipient = if sdi_recipient.is_empty() {
        common::pec::SDI_PEC_ADDRESS
    } else {
        sdi_recipient
    };

    Message::builder()
        .from(
            config
                .email
                .parse()
                .map_err(|e| format!("Invalid sender: {e}"))?,
        )
        .to(sdi_recipient
            .parse()
            .map_err(|e| format!("Invalid SDI address: {e}"))?)
        .subject(format!("INVIO FATTURA {filename}"))
        .multipart(
            MultiPart::mixed()
                .singlepart(SinglePart::plain(
                    "Invio fattura elettronica al Sistema di Interscambio.".to_string(),
                ))
                .singlepart(xml_part),
        )
        .map_err(|e| format!("Failed to build PEC email: {e}"))
}

/// Fetch every message on first sync, then only UIDs after the saved cursor.
pub async fn check_inbox(
    config: &PecConfig,
    cursor: Option<(u32, u32)>,
) -> Result<PecInbox, String> {
    let config = config.clone();
    tokio::task::spawn_blocking(move || fetch_inbox(&config, cursor))
        .await
        .map_err(|e| format!("IMAP task join error: {e}"))?
}

/// Blocking IMAP fetch that does not alter the read/unread flag.
fn fetch_inbox(config: &PecConfig, cursor: Option<(u32, u32)>) -> Result<PecInbox, String> {
    let tls = native_tls::TlsConnector::builder()
        .build()
        .map_err(|e| format!("TLS setup failed: {e}"))?;

    let client = imap::connect(
        (config.imap_host.as_str(), config.imap_port),
        config.imap_host.as_str(),
        &tls,
    )
    .map_err(|e| format!("IMAP connect failed: {e}"))?;

    let mut session = client
        .login(&config.email, &config.password)
        .map_err(|(e, _)| format!("IMAP login failed: {e}"))?;

    let mailbox = session
        .select("INBOX")
        .map_err(|e| format!("IMAP select failed: {e}"))?;
    let start_uid = cursor
        .filter(|(validity, _)| Some(*validity) == mailbox.uid_validity)
        .map(|(_, uid)| uid)
        .unwrap_or(0);
    let mut uids: Vec<u32> = session
        .uid_search("ALL")
        .map_err(|e| format!("IMAP search failed: {e}"))?
        .into_iter()
        .collect();
    uids.sort_unstable();

    let mut messages = Vec::new();
    for uid in uids.iter().copied().filter(|uid| *uid > start_uid) {
        let fetches = session
            .uid_fetch(uid.to_string(), "BODY.PEEK[]")
            .map_err(|e| format!("IMAP fetch failed: {e}"))?;
        for fetch in fetches.iter() {
            if let Some(body) = fetch.body()
                && let Some(mut msg) = parse_mime(body)
            {
                msg.uid = uid;
                messages.push(msg);
            }
        }
    }

    let _ = session.logout();
    Ok(PecInbox {
        uid_validity: mailbox.uid_validity,
        last_uid: uids.last().copied().unwrap_or(start_uid),
        messages,
    })
}

/// Parse a raw RFC822 message into a [`PecMessage`] with its attachments and detected SDI sender.
fn parse_mime(raw: &[u8]) -> Option<PecMessage> {
    let parsed = mail_parser::MessageParser::default().parse(raw)?;
    let subject = parsed.subject().unwrap_or_default().to_string();
    let from = parsed
        .from()
        .and_then(|addr| addr.first())
        .and_then(|a| a.address())
        .unwrap_or_default()
        .to_string();

    let mut attachments = Vec::new();
    let mut sender_candidates = Vec::new();
    let mut fallback_candidates = Vec::new();
    collect_attachments(
        &parsed,
        &mut attachments,
        &mut sender_candidates,
        &mut fallback_candidates,
        0,
    );

    Some(PecMessage {
        uid: 0,
        subject,
        from,
        sdi_sender: select_preferred_sdi_address(sender_candidates),
        sdi_fallback: select_preferred_sdi_address(fallback_candidates),
        attachments,
    })
}

fn collect_sender_addresses(message: &mail_parser::Message<'_>, out: &mut Vec<String>) {
    for addr_list in [message.from(), message.sender(), message.reply_to()]
        .into_iter()
        .flatten()
    {
        for addr in addr_list.iter() {
            if let Some(email) = addr.address() {
                out.extend(extract_sdi_pec_addresses(email));
            }
            if let Some(name) = addr.name() {
                out.extend(extract_sdi_pec_addresses(name));
            }
        }
    }
    if let Some(return_path) = message.return_path().as_text() {
        out.extend(extract_sdi_pec_addresses(return_path));
    }
}

fn collect_fallback_addresses(message: &mail_parser::Message<'_>, out: &mut Vec<String>) {
    if let Some(subject) = message.subject() {
        out.extend(extract_sdi_pec_addresses(subject));
    }
    if let Some(to) = message.to() {
        for addr in to.iter() {
            if let Some(email) = addr.address() {
                out.extend(extract_sdi_pec_addresses(email));
            }
        }
    }
    for body in message.text_bodies() {
        if let Ok(text) = std::str::from_utf8(body.contents()) {
            out.extend(extract_sdi_pec_addresses(text));
        }
    }
}

fn collect_daticert_addresses(
    xml: &str,
    sender_out: &mut Vec<String>,
    fallback_out: &mut Vec<String>,
) {
    for tag in ["mittente", "risposte"] {
        let open = format!("<{tag}");
        let close = format!("</{tag}>");
        let mut rest = xml;
        while let Some(start) = rest.find(&open) {
            rest = &rest[start + open.len()..];
            if let Some(gt) = rest.find('>') {
                rest = &rest[gt + 1..];
                if let Some(end) = rest.find(&close) {
                    sender_out.extend(extract_sdi_pec_addresses(&rest[..end]));
                    rest = &rest[end + close.len()..];
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }
    fallback_out.extend(extract_sdi_pec_addresses(xml));
}

fn collect_attachments(
    message: &mail_parser::Message<'_>,
    output: &mut Vec<PecAttachment>,
    sender_candidates: &mut Vec<String>,
    fallback_candidates: &mut Vec<String>,
    depth: usize,
) {
    if depth >= 5 {
        return;
    }
    collect_sender_addresses(message, sender_candidates);
    collect_fallback_addresses(message, fallback_candidates);

    for part in message.attachments() {
        if let Some(nested) = part.message() {
            collect_attachments(
                nested,
                output,
                sender_candidates,
                fallback_candidates,
                depth + 1,
            );
        } else if let Some(filename) = part.attachment_name() {
            let lower = filename.to_ascii_lowercase();
            if lower.ends_with(".eml") {
                if let Some(nested) = mail_parser::MessageParser::default().parse(part.contents()) {
                    collect_attachments(
                        &nested,
                        output,
                        sender_candidates,
                        fallback_candidates,
                        depth + 1,
                    );
                }
                continue;
            }
            if lower == "daticert.xml"
                && let Ok(xml) = std::str::from_utf8(part.contents())
            {
                collect_daticert_addresses(xml, sender_candidates, fallback_candidates);
            }
            output.push(PecAttachment {
                filename: filename.to_string(),
                data: part.contents().to_vec(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invoice_message_uses_configured_sdi_address() {
        let config = PecConfig {
            email: "sender@pec.example.it".into(),
            sdi_address: "sdi12@pec.fatturapa.it".into(),
            password: String::new(),
            imap_host: String::new(),
            imap_port: 993,
            smtp_host: String::new(),
            smtp_port: 465,
        };
        let message =
            build_invoice_message(&config, "<FatturaElettronica/>", "invoice.xml").unwrap();
        assert_eq!(message.envelope().to()[0].to_string(), config.sdi_address);
    }

    #[test]
    fn invoice_message_falls_back_to_sdi01_when_empty() {
        let config = PecConfig {
            email: "sender@pec.example.it".into(),
            sdi_address: String::new(),
            password: String::new(),
            imap_host: String::new(),
            imap_port: 993,
            smtp_host: String::new(),
            smtp_port: 465,
        };
        let message =
            build_invoice_message(&config, "<FatturaElettronica/>", "invoice.xml").unwrap();
        assert_eq!(
            message.envelope().to()[0].to_string(),
            common::pec::SDI_PEC_ADDRESS
        );
    }

    #[test]
    fn extracts_invoice_and_sdi_sender_from_nested_certified_message() {
        let raw = concat!(
            "From: provider@pec.example.it\r\n",
            "Subject: Posta certificata\r\n",
            "MIME-Version: 1.0\r\n",
            "Content-Type: multipart/mixed; boundary=outer\r\n\r\n",
            "--outer\r\nContent-Type: text/plain\r\n\r\nReceipt\r\n",
            "--outer\r\nContent-Type: application/xml\r\n",
            "Content-Disposition: attachment; filename=daticert.xml\r\n\r\n",
            "<postacert><intestazione><mittente>sdi14@pec.fatturapa.it</mittente></intestazione></postacert>\r\n",
            "--outer\r\nContent-Type: message/rfc822\r\n",
            "Content-Disposition: attachment; filename=postacert.eml\r\n\r\n",
            "From: sdi14@pec.fatturapa.it\r\n",
            "MIME-Version: 1.0\r\n",
            "Content-Type: multipart/mixed; boundary=inner\r\n\r\n",
            "--inner\r\nContent-Type: application/xml\r\n",
            "Content-Disposition: attachment; filename=fattura.xml\r\n\r\n",
            "<FatturaElettronica/>\r\n--inner--\r\n",
            "--outer--\r\n"
        );
        let message = parse_mime(raw.as_bytes()).unwrap();
        assert_eq!(
            message.sdi_sender.as_deref(),
            Some("sdi14@pec.fatturapa.it")
        );
        assert_eq!(message.attachments.len(), 2);
        assert_eq!(message.attachments[1].filename, "fattura.xml");
    }
}
