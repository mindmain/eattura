use super::{ValidationError, ValidationResult};

const MAX_INVOICE_BYTES: usize = 5_000_000;

/// Check the final XML attachment's filename and size against Annex A §1.2.2–1.3.1.
pub fn validate_xml_attachment(filename: &str, xml_size: usize) -> ValidationResult {
    let mut errors = Vec::new();
    let valid_name = filename
        .strip_suffix(".xml")
        .and_then(|stem| stem.split_once('_'))
        .is_some_and(|(identifier, sequence)| {
            let (country, tax_id) = identifier.split_at_checked(2).unwrap_or_default();
            let length = if country == "IT" { 11..=16 } else { 2..=28 };
            country.len() == 2
                && country.bytes().all(|byte| byte.is_ascii_uppercase())
                && length.contains(&tax_id.len())
                && tax_id.bytes().all(|byte| byte.is_ascii_alphanumeric())
                && (1..=5).contains(&sequence.len())
                && sequence.bytes().all(|byte| byte.is_ascii_alphanumeric())
        });
    if !valid_name {
        errors.push(ValidationError {
            code: "00001".into(),
            message: "Invoice filename must use the country code, tax identifier and a 1–5 character alphanumeric sequence".into(),
            element_path: None,
        });
    }
    if xml_size > MAX_INVOICE_BYTES {
        errors.push(ValidationError {
            code: "00003".into(),
            message: "Invoice XML exceeds the 5 MB size limit".into(),
            element_path: None,
        });
    }
    ValidationResult { errors }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdi_00001_invoice_filename_syntax() {
        for name in [
            "IT01234567897_00001.xml",
            "FRAB_Ab5.xml",
            "ITRSSMRA80A01H501U_1.xml",
        ] {
            assert!(validate_xml_attachment(name, 100).is_valid(), "{name}");
        }
        for name in [
            "IT01234567897_123456.xml",
            "IT01234567897_.xml",
            "IT01234567897.xml",
            "FRZ_12345.xml",
            "IT01234567897_12-45.xml",
            "IT01234567897_12_45.xml",
            "IÇ01234567897_00001.xml",
        ] {
            let result = validate_xml_attachment(name, 100);
            assert_eq!(result.errors.len(), 1, "{name}");
            assert_eq!(result.errors[0].code, "00001", "{name}");
        }
    }

    #[test]
    fn sdi_00003_applies_to_the_encoded_file_size() {
        let filename = "IT01234567897_00001.xml";
        assert!(validate_xml_attachment(filename, MAX_INVOICE_BYTES).is_valid());
        let result = validate_xml_attachment(filename, MAX_INVOICE_BYTES + 1);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].code, "00003");
    }
}
