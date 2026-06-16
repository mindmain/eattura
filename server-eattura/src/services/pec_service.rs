use lettre::message::{header::ContentType, Attachment, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use common::pec::{parse_notification, ParsedNotification, SDI_PEC_ADDRESS};

use crate::error::AppError;

/// PEC (Posta Elettronica Certificata) integration for SDI communication.
///
/// The SDI system accepts invoices via PEC email. This service handles:
/// - Sending invoice XML files via PEC SMTP to the SDI endpoint
/// - Polling the PEC inbox via IMAP for new SDI notifications
/// - Parsing SDI notification/receipt messages (delegated to `common::pec`)
pub struct PecService {
    pub config: PecConfig,
}

/// Connection parameters and credentials for a PEC mailbox.
///
/// On the server these come from environment variables; on the desktop the
/// non-secret parts come from settings and the password from the OS keyring.
#[derive(Debug, Clone)]
pub struct PecConfig {
    pub email: String,
    pub password: String,
    pub imap_host: String,
    pub imap_port: u16,
    pub smtp_host: String,
    pub smtp_port: u16,
}

impl PecConfig {
    /// Load PEC configuration from environment variables.
    ///
    /// `PEC_EMAIL`, `PEC_PASSWORD`, `PEC_IMAP_HOST`, `PEC_IMAP_PORT`,
    /// `PEC_SMTP_HOST`, `PEC_SMTP_PORT` (ports default to 993 / 465).
    pub fn from_env() -> Result<Self, AppError> {
        let var = |k: &str| {
            std::env::var(k).map_err(|_| AppError::Internal(format!("Missing env var {k}")))
        };
        Ok(Self {
            email: var("PEC_EMAIL")?,
            password: var("PEC_PASSWORD")?,
            imap_host: var("PEC_IMAP_HOST")?,
            imap_port: std::env::var("PEC_IMAP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(993),
            smtp_host: var("PEC_SMTP_HOST")?,
            smtp_port: std::env::var("PEC_SMTP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(465),
        })
    }
}

/// A message received from the PEC inbox.
#[derive(Debug)]
pub struct PecMessage {
    pub subject: String,
    pub from: String,
    pub attachments: Vec<PecAttachment>,
}

/// An attachment from a PEC message (typically an XML notification from SDI).
#[derive(Debug)]
pub struct PecAttachment {
    pub filename: String,
    pub data: Vec<u8>,
}

impl PecService {
    pub fn new(config: PecConfig) -> Self {
        Self { config }
    }

    /// Send an invoice XML via PEC to the SDI endpoint.
    ///
    /// Builds a PEC email with the XML as an attachment named per SDI
    /// conventions and submits it over an authenticated TLS SMTP connection.
    /// Returns the SMTP message identifier on success.
    pub async fn send_invoice(&self, xml: &str, filename: &str) -> Result<String, AppError> {
        let xml_part = Attachment::new(filename.to_string()).body(
            xml.as_bytes().to_vec(),
            ContentType::parse("application/xml").unwrap(),
        );

        let email = Message::builder()
            .from(
                self.config
                    .email
                    .parse()
                    .map_err(|e| AppError::Internal(format!("Invalid sender address: {e}")))?,
            )
            .to(SDI_PEC_ADDRESS
                .parse()
                .map_err(|e| AppError::Internal(format!("Invalid SDI address: {e}")))?)
            .subject(format!("INVIO FATTURA {filename}"))
            .multipart(
                MultiPart::mixed()
                    .singlepart(
                        SinglePart::plain("Invio fattura elettronica al Sistema di Interscambio.".to_string()),
                    )
                    .singlepart(xml_part),
            )
            .map_err(|e| AppError::Internal(format!("Failed to build PEC email: {e}")))?;

        let creds = Credentials::new(self.config.email.clone(), self.config.password.clone());
        let mailer: AsyncSmtpTransport<Tokio1Executor> =
            AsyncSmtpTransport::<Tokio1Executor>::relay(&self.config.smtp_host)
                .map_err(|e| AppError::Internal(format!("SMTP relay setup failed: {e}")))?
                .port(self.config.smtp_port)
                .credentials(creds)
                .build();

        let response = mailer
            .send(email)
            .await
            .map_err(|e| AppError::Internal(format!("PEC send failed: {e}")))?;

        Ok(response
            .message()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string())
    }

    /// Check the PEC inbox for new (unseen) SDI notifications.
    ///
    /// Connects via IMAPS, fetches unseen messages, and extracts their XML
    /// attachments. The synchronous `imap` client runs on a blocking thread so
    /// it does not stall the async runtime.
    pub async fn check_inbox(&self) -> Result<Vec<PecMessage>, AppError> {
        let config = self.config.clone();
        tokio::task::spawn_blocking(move || Self::fetch_unseen(&config))
            .await
            .map_err(|e| AppError::Internal(format!("IMAP task join error: {e}")))?
    }

    /// Blocking IMAP fetch of unseen messages and their attachments.
    fn fetch_unseen(config: &PecConfig) -> Result<Vec<PecMessage>, AppError> {
        let tls = native_tls::TlsConnector::builder()
            .build()
            .map_err(|e| AppError::Internal(format!("TLS setup failed: {e}")))?;

        let client = imap::connect(
            (config.imap_host.as_str(), config.imap_port),
            config.imap_host.as_str(),
            &tls,
        )
        .map_err(|e| AppError::Internal(format!("IMAP connect failed: {e}")))?;

        let mut session = client
            .login(&config.email, &config.password)
            .map_err(|(e, _)| AppError::Internal(format!("IMAP login failed: {e}")))?;

        session
            .select("INBOX")
            .map_err(|e| AppError::Internal(format!("IMAP select failed: {e}")))?;

        let unseen = session
            .search("UNSEEN")
            .map_err(|e| AppError::Internal(format!("IMAP search failed: {e}")))?;

        let mut messages = Vec::new();
        for uid in unseen {
            let fetches = session
                .fetch(uid.to_string(), "RFC822")
                .map_err(|e| AppError::Internal(format!("IMAP fetch failed: {e}")))?;
            for fetch in fetches.iter() {
                if let Some(body) = fetch.body() {
                    if let Some(msg) = Self::parse_mime(body) {
                        messages.push(msg);
                    }
                }
            }
        }

        let _ = session.logout();
        Ok(messages)
    }

    /// Parse a raw RFC822 message into a [`PecMessage`] with its attachments.
    fn parse_mime(raw: &[u8]) -> Option<PecMessage> {
        use mail_parser::MimeHeaders;
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

    /// Parse an SDI notification XML into a structured notification.
    ///
    /// Delegates to the shared `common::pec` parser so the server and desktop
    /// interpret notifications identically.
    pub fn parse_notification(xml: &str) -> Result<ParsedNotification, AppError> {
        parse_notification(xml).map_err(|e| AppError::Internal(format!("Notification parse failed: {e}")))
    }
}
