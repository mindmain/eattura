//! PEC transport for the desktop app (SMTP send + IMAP receive).
//!
//! Mirrors the server's transport but follows the desktop convention of
//! `Result<_, String>` errors. The non-secret connection parameters come from
//! the app settings; the password is read from the OS keyring.

use lettre::message::{header::ContentType, Attachment, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use mail_parser::MimeHeaders;

use common::pec::SDI_PEC_ADDRESS;

/// Keyring service name under which PEC passwords are stored.
const KEYRING_SERVICE: &str = "eattura-pec";

/// Connection parameters and credentials for a PEC mailbox.
#[derive(Debug, Clone)]
pub struct PecConfig {
    pub email: String,
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
    pub subject: String,
    pub from: String,
    pub attachments: Vec<PecAttachment>,
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
    let xml_part = Attachment::new(filename.to_string()).body(
        xml.as_bytes().to_vec(),
        ContentType::parse("application/xml").unwrap(),
    );

    let email = Message::builder()
        .from(config.email.parse().map_err(|e| format!("Invalid sender: {e}"))?)
        .to(SDI_PEC_ADDRESS.parse().map_err(|e| format!("Invalid SDI address: {e}"))?)
        .subject(format!("INVIO FATTURA {filename}"))
        .multipart(
            MultiPart::mixed()
                .singlepart(SinglePart::plain(
                    "Invio fattura elettronica al Sistema di Interscambio.".to_string(),
                ))
                .singlepart(xml_part),
        )
        .map_err(|e| format!("Failed to build PEC email: {e}"))?;

    let creds = Credentials::new(config.email.clone(), config.password.clone());
    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)
            .map_err(|e| format!("SMTP setup failed: {e}"))?
            .port(config.smtp_port)
            .credentials(creds)
            .build();

    let response = mailer.send(email).await.map_err(|e| format!("PEC send failed: {e}"))?;
    Ok(response.message().collect::<Vec<_>>().join(" ").trim().to_string())
}

/// Fetch unseen messages from the PEC inbox via IMAPS.
pub async fn check_inbox(config: &PecConfig) -> Result<Vec<PecMessage>, String> {
    let config = config.clone();
    tokio::task::spawn_blocking(move || fetch_unseen(&config))
        .await
        .map_err(|e| format!("IMAP task join error: {e}"))?
}

/// Blocking IMAP fetch of unseen messages and their attachments.
fn fetch_unseen(config: &PecConfig) -> Result<Vec<PecMessage>, String> {
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

    session.select("INBOX").map_err(|e| format!("IMAP select failed: {e}"))?;
    let unseen = session.search("UNSEEN").map_err(|e| format!("IMAP search failed: {e}"))?;

    let mut messages = Vec::new();
    for uid in unseen {
        let fetches = session
            .fetch(uid.to_string(), "RFC822")
            .map_err(|e| format!("IMAP fetch failed: {e}"))?;
        for fetch in fetches.iter() {
            if let Some(body) = fetch.body()
                && let Some(msg) = parse_mime(body) {
                    messages.push(msg);
                }
        }
    }

    let _ = session.logout();
    Ok(messages)
}

/// Parse a raw RFC822 message into a [`PecMessage`] with its attachments.
fn parse_mime(raw: &[u8]) -> Option<PecMessage> {
    let parsed = mail_parser::MessageParser::default().parse(raw)?;
    let subject = parsed.subject().unwrap_or_default().to_string();
    let from = parsed
        .from()
        .and_then(|addr| addr.first())
        .and_then(|a| a.address())
        .unwrap_or_default()
        .to_string();

    let attachments = parsed
        .attachments()
        .filter_map(|part| {
            let filename = part.attachment_name()?.to_string();
            Some(PecAttachment {
                filename,
                data: part.contents().to_vec(),
            })
        })
        .collect();

    Some(PecMessage {
        subject,
        from,
        attachments,
    })
}
