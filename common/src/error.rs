use thiserror::Error;

/// Top-level error type for the common crate.
/// Wraps XML and validation errors into a single type for consumers.
#[derive(Debug, Error)]
pub enum CommonError {
    #[error("XML error: {0}")]
    Xml(#[from] crate::xml::XmlError),

    #[error("Validation failed with {0} errors")]
    Validation(crate::validation::ValidationResult),
}
