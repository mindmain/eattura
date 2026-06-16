use thiserror::Error;

/// Errors that can occur during XML encoding/decoding of SDI invoices.
#[derive(Debug, Error)]
pub enum XmlError {
    /// Failed to serialize a FatturaElettronica into XML.
    #[error("XML serialization failed: {0}")]
    Serialize(String),

    /// Failed to deserialize XML into a FatturaElettronica.
    #[error("XML deserialization failed: {0}")]
    Deserialize(String),

    /// An enum value string from XML does not match any known SDI value.
    #[error("Unknown enum value '{value}' for type {enum_type}")]
    UnknownEnumValue {
        enum_type: String,
        value: String,
    },

    /// A required XML element is missing.
    #[error("Missing required element: {0}")]
    MissingElement(String),

    /// I/O error during read/write.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Underlying quick-xml error.
    #[error("XML parser error: {0}")]
    QuickXml(String),
}

impl From<quick_xml::Error> for XmlError {
    fn from(e: quick_xml::Error) -> Self {
        XmlError::QuickXml(e.to_string())
    }
}

impl From<quick_xml::DeError> for XmlError {
    fn from(e: quick_xml::DeError) -> Self {
        XmlError::Deserialize(e.to_string())
    }
}
