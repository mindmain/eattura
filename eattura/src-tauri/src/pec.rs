//! PEC transport for the desktop app (SMTP send + IMAP receive).
//!
//! Uses a [`MailboxSession`] adapter trait so IMAP folder discovery, UID cursor
//! progression, `UIDVALIDITY` resets, and nested PEC MIME parsing can be tested
//! deterministically in memory without connecting to a live PEC server.

use std::collections::BTreeMap;

use common::pec::{extract_sdi_pec_addresses, select_preferred_sdi_address};
use lettre::message::{Attachment, MultiPart, SinglePart, header::ContentType};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use mail_parser::{MessagePart, MimeHeaders, PartType};

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

/// Transport-level PEC receipt parsed from `daticert.xml` (`accettazione` or `avvenuta-consegna`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PecTransportReceipt {
    /// `"accettazione"` or `"avvenuta-consegna"`.
    pub kind: String,
    /// `<oggetto>` of the certified envelope (e.g. `"INVIO FATTURA IT..._74660.xml"`).
    pub subject: Option<String>,
    /// Certified delivery target (`<consegna>` or `<destinatari>`), e.g. `"sdi48@pec.fatturapa.it"`.
    pub recipient: Option<String>,
    /// Envelope timestamp `<giorno> <ora>` from `daticert.xml`, e.g. `"27/11/2026 11:21:25"`.
    pub timestamp: Option<String>,
}

/// A message fetched from the PEC inbox.
#[derive(Debug, Clone)]
#[allow(dead_code)] // subject/from are part of the message model, used for logging/diagnostics
pub struct PecMessage {
    pub uid: u32,
    pub subject: String,
    pub from: String,
    /// SDI PEC address (`*@pec.fatturapa.it`) detected from message sender headers or `daticert.xml`.
    pub sdi_sender: Option<String>,
    /// Fallback SDI PEC address found anywhere in headers/body if the message carries an SDI notification.
    pub sdi_fallback: Option<String>,
    /// Transport receipt metadata when this message is a PEC `accettazione` or `avvenuta-consegna`.
    pub transport_receipt: Option<PecTransportReceipt>,
    pub attachments: Vec<PecAttachment>,
}

/// A consistent IMAP scan with per-folder `(uid_validity, last_uid)` cursors for incremental sync.
#[derive(Debug, Clone)]
pub struct PecInbox {
    pub uid_validity: Option<u32>,
    pub last_uid: u32,
    pub folder_cursors: BTreeMap<String, (u32, u32)>,
    pub messages: Vec<PecMessage>,
}

/// An attachment of a PEC message (typically an XML notification from SDI).
#[derive(Debug, Clone)]
pub struct PecAttachment {
    pub filename: String,
    pub data: Vec<u8>,
}

/// Folder descriptor returned by a [`MailboxSession`] implementation.
#[derive(Debug, Clone)]
pub struct MailboxFolderEntry {
    pub name: String,
    pub has_excluded_attr: bool,
}

/// Port/Adapter abstraction over an IMAP session so PEC sync logic is 100% testable offline.
pub trait MailboxSession {
    fn list_folders(&mut self) -> Result<Vec<MailboxFolderEntry>, String>;
    /// Select `folder` and return its `UIDVALIDITY`.
    fn select_folder(&mut self, folder: &str) -> Result<Option<u32>, String>;
    fn uid_search_all(&mut self) -> Result<Vec<u32>, String>;
    fn uid_fetch_peek(&mut self, uid: u32) -> Result<Vec<Vec<u8>>, String>;
    fn logout(&mut self);
}

struct LiveImapSession<T: std::io::Read + std::io::Write> {
    session: imap::Session<T>,
}

impl<T: std::io::Read + std::io::Write> MailboxSession for LiveImapSession<T> {
    fn list_folders(&mut self) -> Result<Vec<MailboxFolderEntry>, String> {
        let listed = self
            .session
            .list(None, Some("*"))
            .map_err(|e| format!("IMAP list failed: {e}"))?;
        Ok(listed
            .iter()
            .map(|entry| {
                let has_excluded_attr = entry.attributes().iter().any(|attr| match attr {
                    imap::types::NameAttribute::NoSelect => true,
                    imap::types::NameAttribute::Custom(flag) => matches!(
                        flag.to_ascii_lowercase().as_str(),
                        "\\trash" | "\\sent" | "\\drafts" | "\\junk" | "\\archive"
                    ),
                    _ => false,
                });
                MailboxFolderEntry {
                    name: entry.name().to_string(),
                    has_excluded_attr,
                }
            })
            .collect())
    }

    fn select_folder(&mut self, folder: &str) -> Result<Option<u32>, String> {
        let mailbox = self
            .session
            .select(folder)
            .map_err(|e| format!("IMAP select failed: {e}"))?;
        Ok(mailbox.uid_validity)
    }

    fn uid_search_all(&mut self) -> Result<Vec<u32>, String> {
        let uids = self
            .session
            .uid_search("ALL")
            .map_err(|e| format!("IMAP search failed: {e}"))?;
        Ok(uids.into_iter().collect())
    }

    fn uid_fetch_peek(&mut self, uid: u32) -> Result<Vec<Vec<u8>>, String> {
        let fetches = self
            .session
            .uid_fetch(uid.to_string(), "BODY.PEEK[]")
            .map_err(|e| format!("IMAP fetch failed: {e}"))?;
        Ok(fetches
            .iter()
            .filter_map(|fetch| fetch.body().map(|b| b.to_vec()))
            .collect())
    }

    fn logout(&mut self) {
        let _ = self.session.logout();
    }
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

pub struct PreparedInvoice {
    email: Message,
    mailer: AsyncSmtpTransport<Tokio1Executor>,
}

/// Prepare the local message and transport before recording an SMTP attempt.
pub fn prepare_invoice(
    config: &PecConfig,
    xml: &str,
    filename: &str,
) -> Result<PreparedInvoice, String> {
    let email = build_invoice_message(config, xml, filename)?;

    let creds = Credentials::new(config.email.clone(), config.password.clone());
    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)
            .map_err(|e| format!("SMTP setup failed: {e}"))?
            .port(config.smtp_port)
            .credentials(creds)
            .build();

    Ok(PreparedInvoice { email, mailer })
}

impl PreparedInvoice {
    pub async fn send(self) -> Result<String, String> {
        let response = self
            .mailer
            .send(self.email)
            .await
            .map_err(|e| e.to_string())?;
        Ok(response
            .message()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string())
    }
}

fn build_invoice_message(
    config: &PecConfig,
    xml: &str,
    filename: &str,
) -> Result<Message, String> {
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

/// Decide whether an IMAP folder should be scanned for PEC receipts / SdI messages.
pub(crate) fn is_scannable_pec_folder_name(name: &str, has_excluded_attr: bool) -> bool {
    if has_excluded_attr {
        return false;
    }
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.eq_ignore_ascii_case("INBOX") {
        return true;
    }
    let lower = trimmed.to_ascii_lowercase();
    let segments: Vec<&str> = lower.split(['.', '/']).collect();
    let excluded_keywords = [
        "trash",
        "cestino",
        "deleted",
        "sent",
        "inviat",
        "draft",
        "bozze",
        "spam",
        "junk",
        "archivio",
    ];
    if segments
        .iter()
        .any(|seg| excluded_keywords.iter().any(|kw| seg.contains(kw)))
    {
        return false;
    }
    let relevant_keywords = ["ricevut", "fattur", "sdi", "certificat"];
    segments
        .iter()
        .any(|seg| relevant_keywords.iter().any(|kw| seg.contains(kw)))
}

/// Fetch every message on first sync, then only UIDs after each folder's saved cursor.
pub async fn check_inbox(
    config: &PecConfig,
    cursors: BTreeMap<String, (u32, u32)>,
) -> Result<PecInbox, String> {
    let config = config.clone();
    tokio::task::spawn_blocking(move || fetch_live_inbox(&config, &cursors))
        .await
        .map_err(|e| format!("IMAP task join error: {e}"))?
}

fn fetch_live_inbox(
    config: &PecConfig,
    cursors: &BTreeMap<String, (u32, u32)>,
) -> Result<PecInbox, String> {
    let tls = native_tls::TlsConnector::builder()
        .build()
        .map_err(|e| format!("TLS setup failed: {e}"))?;

    let client = imap::connect(
        (config.imap_host.as_str(), config.imap_port),
        config.imap_host.as_str(),
        &tls,
    )
    .map_err(|e| format!("IMAP connect failed: {e}"))?;

    let session = client
        .login(&config.email, &config.password)
        .map_err(|(e, _)| format!("IMAP login failed: {e}"))?;

    let mut adapter = LiveImapSession { session };
    fetch_inbox_with_session(&mut adapter, cursors)
}

/// Adapter-driven IMAP scan across `INBOX` and any PEC receipt/SdI subfolders without altering `\Seen`.
pub fn fetch_inbox_with_session(
    session: &mut dyn MailboxSession,
    cursors: &BTreeMap<String, (u32, u32)>,
) -> Result<PecInbox, String> {
    let mut folders = vec!["INBOX".to_string()];
    if let Ok(listed) = session.list_folders() {
        for entry in listed {
            if !entry.name.eq_ignore_ascii_case("INBOX")
                && is_scannable_pec_folder_name(&entry.name, entry.has_excluded_attr)
                && !folders.iter().any(|existing| existing == &entry.name)
            {
                folders.push(entry.name);
            }
        }
    }

    let mut messages = Vec::new();
    let mut folder_cursors = cursors.clone();
    let mut inbox_uid_validity = None;
    let mut inbox_last_uid = 0;

    for folder in folders {
        let is_primary_inbox = folder.eq_ignore_ascii_case("INBOX");
        let uid_validity = match session.select_folder(&folder) {
            Ok(v) => v,
            Err(_) if !is_primary_inbox => continue,
            Err(e) => return Err(e),
        };
        let saved_cursor = cursors.get(&folder).copied().or_else(|| {
            is_primary_inbox
                .then(|| cursors.get("INBOX").copied())
                .flatten()
        });
        let start_uid = saved_cursor
            .filter(|(validity, _)| Some(*validity) == uid_validity)
            .map(|(_, uid)| uid)
            .unwrap_or(0);

        let mut uids = match session.uid_search_all() {
            Ok(list) => list,
            Err(_) if !is_primary_inbox => continue,
            Err(e) => return Err(e),
        };
        uids.sort_unstable();

        for uid in uids.iter().copied().filter(|uid| *uid > start_uid) {
            let bodies = match session.uid_fetch_peek(uid) {
                Ok(b) => b,
                Err(_) if !is_primary_inbox => continue,
                Err(e) => return Err(e),
            };
            for body in bodies {
                if let Some(mut msg) = parse_mime(&body) {
                    msg.uid = uid;
                    messages.push(msg);
                }
            }
        }

        let last_uid = uids.last().copied().unwrap_or(start_uid);
        if let Some(validity) = uid_validity {
            let key = if is_primary_inbox {
                "INBOX".to_string()
            } else {
                folder.clone()
            };
            folder_cursors.insert(key, (validity, last_uid));
        }
        if is_primary_inbox {
            inbox_uid_validity = uid_validity;
            inbox_last_uid = last_uid;
        }
    }

    session.logout();
    Ok(PecInbox {
        uid_validity: inbox_uid_validity,
        last_uid: inbox_last_uid,
        folder_cursors,
        messages,
    })
}

/// Parse a raw RFC822 message into a [`PecMessage`] with its attachments and detected SDI sender.
pub(crate) fn parse_mime(raw: &[u8]) -> Option<PecMessage> {
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
    let mut transport_receipt = None;
    collect_attachments(
        &parsed,
        &mut attachments,
        &mut sender_candidates,
        &mut fallback_candidates,
        &mut transport_receipt,
        0,
    );

    Some(PecMessage {
        uid: 0,
        subject,
        from,
        sdi_sender: select_preferred_sdi_address(sender_candidates),
        sdi_fallback: select_preferred_sdi_address(fallback_candidates),
        transport_receipt,
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

fn extract_simple_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let start = xml.find(&open)?;
    let after = &xml[start + open.len()..];
    let gt = after.find('>')?;
    let inner = &after[gt + 1..];
    let end = inner.find(&close)?;
    let text = inner[..end].trim();
    (!text.is_empty()).then(|| text.to_string())
}

pub(crate) fn parse_daticert_transport_receipt(xml: &str) -> Option<PecTransportReceipt> {
    let postacert_start = xml.find("<postacert")?;
    let header_end = xml[postacert_start..].find('>')?;
    let root_tag = &xml[postacert_start..postacert_start + header_end];
    let lower_tag = root_tag.to_ascii_lowercase();
    let kind = if lower_tag.contains("tipo=\"avvenuta-consegna\"")
        || lower_tag.contains("tipo='avvenuta-consegna'")
    {
        "avvenuta-consegna"
    } else if lower_tag.contains("tipo=\"accettazione\"")
        || lower_tag.contains("tipo='accettazione'")
    {
        "accettazione"
    } else {
        return None;
    };

    let subject = extract_simple_xml_tag(xml, "oggetto");
    let recipient = extract_simple_xml_tag(xml, "consegna")
        .or_else(|| extract_simple_xml_tag(xml, "destinatari"));
    let giorno = extract_simple_xml_tag(xml, "giorno");
    let ora = extract_simple_xml_tag(xml, "ora");
    let timestamp = match (giorno, ora) {
        (Some(g), Some(o)) => Some(format!("{g} {o}")),
        (Some(g), None) => Some(g),
        _ => None,
    };

    Some(PecTransportReceipt {
        kind: kind.to_string(),
        subject,
        recipient,
        timestamp,
    })
}

fn collect_daticert_addresses(
    xml: &str,
    sender_out: &mut Vec<String>,
    fallback_out: &mut Vec<String>,
    transport_receipt: &mut Option<PecTransportReceipt>,
) {
    if transport_receipt.is_none() {
        *transport_receipt = parse_daticert_transport_receipt(xml);
    }
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

fn is_xml_mime_part(part: &MessagePart<'_>) -> bool {
    let Some(ct) = part.content_type() else {
        return false;
    };
    let ctype = ct.ctype().to_ascii_lowercase();
    let subtype = ct.subtype().unwrap_or_default().to_ascii_lowercase();
    (ctype == "application" || ctype == "text") && (subtype == "xml" || subtype.ends_with("+xml"))
}

fn collect_attachments(
    message: &mail_parser::Message<'_>,
    output: &mut Vec<PecAttachment>,
    sender_candidates: &mut Vec<String>,
    fallback_candidates: &mut Vec<String>,
    transport_receipt: &mut Option<PecTransportReceipt>,
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
                transport_receipt,
                depth + 1,
            );
        } else if let Some(raw_filename) = part.attachment_name() {
            let filename = raw_filename.trim();
            if filename.is_empty() {
                continue;
            }
            let lower = filename.to_ascii_lowercase();
            if lower.ends_with(".eml") {
                if let Some(nested) = mail_parser::MessageParser::default().parse(part.contents()) {
                    collect_attachments(
                        &nested,
                        output,
                        sender_candidates,
                        fallback_candidates,
                        transport_receipt,
                        depth + 1,
                    );
                }
                continue;
            }
            if depth == 0
                && lower == "daticert.xml"
                && let Ok(xml) = std::str::from_utf8(part.contents())
            {
                collect_daticert_addresses(
                    xml,
                    sender_candidates,
                    fallback_candidates,
                    transport_receipt,
                );
            } else if lower == "daticert.xml"
                && let Ok(xml) = std::str::from_utf8(part.contents())
            {
                let mut nested_tr = None;
                collect_daticert_addresses(
                    xml,
                    sender_candidates,
                    fallback_candidates,
                    &mut nested_tr,
                );
            }
            output.push(PecAttachment {
                filename: filename.to_string(),
                data: part.contents().to_vec(),
            });
        } else if is_xml_mime_part(part) && !part.contents().is_empty() {
            output.push(PecAttachment {
                filename: format!("inline_{}.xml", output.len() + 1),
                data: part.contents().to_vec(),
            });
        }
    }

    // Also inspect non-attachment MIME parts for inline XML receipts lacking Content-Disposition.
    for (idx, part) in message.parts.iter().enumerate() {
        if part.attachment_name().is_some() {
            continue;
        }
        if matches!(&part.body, PartType::Text(_) | PartType::Binary(_))
            && is_xml_mime_part(part)
            && !part.contents().is_empty()
            && !output.iter().any(|existing| existing.data == part.contents())
        {
            output.push(PecAttachment {
                filename: format!("inline_part_{idx}.xml"),
                data: part.contents().to_vec(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type MockFolder = (MailboxFolderEntry, Option<u32>, BTreeMap<u32, Vec<u8>>);

    /// In-memory `MailboxSession` test adapter simulating a multi-folder PEC IMAP server.
    #[derive(Default)]
    struct MockImapSession {
        folders: Vec<MockFolder>,
        selected: Option<usize>,
        logged_out: bool,
    }

    impl MockImapSession {
        fn add_folder(
            &mut self,
            name: &str,
            has_excluded_attr: bool,
            uid_validity: Option<u32>,
            messages: Vec<(u32, Vec<u8>)>,
        ) {
            self.folders.push((
                MailboxFolderEntry {
                    name: name.to_string(),
                    has_excluded_attr,
                },
                uid_validity,
                messages.into_iter().collect(),
            ));
        }

        fn push_message(&mut self, folder: &str, uid: u32, raw: Vec<u8>) {
            if let Some((_, _, map)) = self.folders.iter_mut().find(|(f, _, _)| f.name == folder) {
                map.insert(uid, raw);
            }
        }
    }

    impl MailboxSession for MockImapSession {
        fn list_folders(&mut self) -> Result<Vec<MailboxFolderEntry>, String> {
            Ok(self.folders.iter().map(|(f, _, _)| f.clone()).collect())
        }

        fn select_folder(&mut self, folder: &str) -> Result<Option<u32>, String> {
            let idx = self
                .folders
                .iter()
                .position(|(f, _, _)| f.name.eq_ignore_ascii_case(folder))
                .ok_or_else(|| format!("Folder not found: {folder}"))?;
            self.selected = Some(idx);
            Ok(self.folders[idx].1)
        }

        fn uid_search_all(&mut self) -> Result<Vec<u32>, String> {
            let idx = self.selected.ok_or("No folder selected")?;
            Ok(self.folders[idx].2.keys().copied().collect())
        }

        fn uid_fetch_peek(&mut self, uid: u32) -> Result<Vec<Vec<u8>>, String> {
            let idx = self.selected.ok_or("No folder selected")?;
            Ok(self.folders[idx]
                .2
                .get(&uid)
                .cloned()
                .into_iter()
                .collect())
        }

        fn logout(&mut self) {
            self.logged_out = true;
        }
    }

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
            "<postacert tipo=\"posta-certificata\"><intestazione><mittente>sdi14@pec.fatturapa.it</mittente></intestazione></postacert>\r\n",
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
        assert!(message.transport_receipt.is_none());
        assert_eq!(message.attachments.len(), 2);
        assert_eq!(message.attachments[1].filename, "fattura.xml");
    }

    #[test]
    fn parses_pec_accettazione_and_avvenuta_consegna_daticert() {
        let consegna_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<postacert tipo="avvenuta-consegna" errore="nessuno">
  <intestazione>
    <mittente>spinanico@pec.it</mittente>
    <destinatari tipo="certificato">sdi48@pec.fatturapa.it</destinatari>
    <oggetto>INVIO FATTURA ITSPNNCI93R07C351G_74660.xml</oggetto>
  </intestazione>
  <dati>
    <consegna>sdi48@pec.fatturapa.it</consegna>
    <data zona="+0100">
      <giorno>27/11/2026</giorno>
      <ora>11:21:25</ora>
    </data>
  </dati>
</postacert>"#;
        let parsed = parse_daticert_transport_receipt(consegna_xml).unwrap();
        assert_eq!(parsed.kind, "avvenuta-consegna");
        assert_eq!(
            parsed.subject.as_deref(),
            Some("INVIO FATTURA ITSPNNCI93R07C351G_74660.xml")
        );
        assert_eq!(parsed.recipient.as_deref(), Some("sdi48@pec.fatturapa.it"));
        assert_eq!(parsed.timestamp.as_deref(), Some("27/11/2026 11:21:25"));

        let incoming_xml = r#"<postacert tipo="posta-certificata" errore="nessuno"></postacert>"#;
        assert!(parse_daticert_transport_receipt(incoming_xml).is_none());
    }

    #[test]
    fn extracts_inline_xml_without_filename_and_octet_stream_eml() {
        let raw = concat!(
            "From: posta-certificata@pec.aruba.it\r\n",
            "Subject: POSTA CERTIFICATA: Invio File 18658643935\r\n",
            "MIME-Version: 1.0\r\n",
            "Content-Type: multipart/mixed; boundary=outer\r\n\r\n",
            "--outer\r\nContent-Type: application/octet-stream; name=\"postacert.eml\"\r\n",
            "Content-Disposition: attachment; filename=\"postacert.eml\"\r\n\r\n",
            "From: sdi48@pec.fatturapa.it\r\n",
            "MIME-Version: 1.0\r\n",
            "Content-Type: application/xml\r\n\r\n",
            "<RicevutaConsegna><IdentificativoSdI>18658643935</IdentificativoSdI><NomeFile>ITSPNNCI93R07C351G_74660.xml</NomeFile></RicevutaConsegna>\r\n",
            "--outer--\r\n"
        );
        let message = parse_mime(raw.as_bytes()).unwrap();
        assert_eq!(
            message.sdi_sender.as_deref(),
            Some("sdi48@pec.fatturapa.it")
        );
        assert_eq!(message.attachments.len(), 1);
        assert!(
            std::str::from_utf8(&message.attachments[0].data)
                .unwrap()
                .contains("<RicevutaConsegna>")
        );
    }

    #[test]
    fn mock_imap_adapter_scans_inbox_and_ricevute_incrementally_and_handles_uidvalidity_reset() {
        let msg_inbox = b"From: sdi48@pec.fatturapa.it\r\nSubject: Invio File 1\r\nContent-Type: text/plain\r\n\r\nBody\r\n".to_vec();
        let msg_ricevute = b"From: posta-certificata@pec.aruba.it\r\nSubject: CONSEGNA: INVIO FATTURA IT01234567890_00001.xml\r\nContent-Type: text/plain\r\n\r\nConsegna\r\n".to_vec();
        let msg_trash = b"From: spam@example.com\r\nSubject: Trash\r\nContent-Type: text/plain\r\n\r\nIgnored\r\n".to_vec();

        let mut mock = MockImapSession::default();
        mock.add_folder("INBOX", false, Some(100), vec![(10, msg_inbox)]);
        mock.add_folder("INBOX.Ricevute", false, Some(200), vec![(5, msg_ricevute)]);
        mock.add_folder("INBOX.Cestino", false, Some(300), vec![(99, msg_trash)]);

        // 1. Initial scan reads INBOX and INBOX.Ricevute, skipping INBOX.Cestino
        let first = fetch_inbox_with_session(&mut mock, &BTreeMap::new()).unwrap();
        assert!(mock.logged_out);
        assert_eq!(first.messages.len(), 2);
        assert_eq!(first.folder_cursors.get("INBOX"), Some(&(100, 10)));
        assert_eq!(first.folder_cursors.get("INBOX.Ricevute"), Some(&(200, 5)));
        assert!(!first.folder_cursors.contains_key("INBOX.Cestino"));

        // 2. Second scan with no new messages returns 0 messages
        let second = fetch_inbox_with_session(&mut mock, &first.folder_cursors).unwrap();
        assert_eq!(second.messages.len(), 0);

        // 3. New message arrives in INBOX.Ricevute (UID 6) -> only UID 6 is fetched
        mock.push_message(
            "INBOX.Ricevute",
            6,
            b"From: sdi48@pec.fatturapa.it\r\nSubject: Invio File 2\r\nContent-Type: text/plain\r\n\r\nRC\r\n".to_vec(),
        );
        let third = fetch_inbox_with_session(&mut mock, &second.folder_cursors).unwrap();
        assert_eq!(third.messages.len(), 1);
        assert_eq!(third.messages[0].uid, 6);
        assert_eq!(third.folder_cursors.get("INBOX.Ricevute"), Some(&(200, 6)));

        // 4. UIDVALIDITY rotation on INBOX resets only INBOX's cursor and re-fetches UID 10
        mock.folders[0].1 = Some(101);
        let fourth = fetch_inbox_with_session(&mut mock, &third.folder_cursors).unwrap();
        assert_eq!(fourth.messages.len(), 1);
        assert_eq!(fourth.messages[0].uid, 10);
        assert_eq!(fourth.folder_cursors.get("INBOX"), Some(&(101, 10)));
    }
}
