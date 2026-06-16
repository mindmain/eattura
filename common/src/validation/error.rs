/// Result of validating an invoice against SDI rules.
/// Contains zero or more errors found during validation.
#[derive(Debug)]
pub struct ValidationResult {
    pub errors: Vec<ValidationError>,
}

impl ValidationResult {
    /// Returns true if the invoice passed all validation checks.
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Returns the number of validation errors found.
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }
}

impl std::fmt::Display for ValidationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_valid() {
            write!(f, "Validation passed")
        } else {
            write!(f, "Validation failed with {} errors", self.errors.len())
        }
    }
}

/// A single validation error, mapped to an SDI error code.
///
/// Each error corresponds to a specific check defined by the Agenzia delle Entrate.
/// See `proto/eattura/sdi/v1/errors.proto` for the full error code catalog.
#[derive(Debug)]
pub struct ValidationError {
    /// SDI error code (e.g., "00400", "00401").
    pub code: String,

    /// Human-readable description of what went wrong.
    pub message: String,

    /// XPath-like path to the offending XML element (e.g., "Body/DatiGenerali/DatiGeneraliDocumento/Natura").
    pub element_path: Option<String>,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.element_path {
            Some(path) => write!(f, "[{}] {} (at {})", self.code, self.message, path),
            None => write!(f, "[{}] {}", self.code, self.message),
        }
    }
}
