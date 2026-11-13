pub mod error;
mod file;
pub mod rules;

pub use error::{ValidationError, ValidationResult};
pub use file::validate_xml_attachment;
pub use rules::{validate, validate_on};
