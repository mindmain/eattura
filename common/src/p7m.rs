use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SignerInfo {
    pub is_signed: bool,
    pub signer_name: Option<String>,
    pub signer_fiscal_code: Option<String>,
    pub issuer: Option<String>,
    pub signing_time: Option<String>,
    pub algorithm: Option<String>,
    pub digest_hex: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum P7mError {
    #[error("I/O error: {0}")]
    Io(String),
    #[error("Invalid ASN.1 encoding: {0}")]
    InvalidAsn1(String),
    #[error("No payload found in P7M / PKCS#7 envelope")]
    MissingPayload,
    #[error("XML extraction error: {0}")]
    XmlExtraction(String),
}

/// Check whether the given byte buffer represents a P7M file or raw XML.
pub fn is_p7m(data: &[u8]) -> bool {
    let trimmed = data.trim_ascii_start();
    if trimmed.starts_with(b"<?xml")
        || trimmed.starts_with(b"<p:FatturaElettronica")
        || trimmed.starts_with(b"<FatturaElettronica")
    {
        return false;
    }
    trimmed.first() == Some(&0x30)
}

/// Extract the raw XML invoice payload from a `.xml.p7m` byte slice or raw `.xml`.
pub fn extract_payload(data: &[u8]) -> Result<Vec<u8>, P7mError> {
    let (payload, _) = inspect_p7m(data)?;
    Ok(payload)
}

/// Inspect a `.p7m` file: extracts the XML payload and metadata from the PKCS#7 envelope.
pub fn inspect_p7m(data: &[u8]) -> Result<(Vec<u8>, SignerInfo), P7mError> {
    let trimmed = data.trim_ascii();

    // 1. If it's already plain XML
    if trimmed.starts_with(b"<?xml")
        || trimmed.starts_with(b"<p:FatturaElettronica")
        || trimmed.starts_with(b"<FatturaElettronica")
    {
        let digest = format!("{:x}", Sha256::digest(trimmed));
        return Ok((
            trimmed.to_vec(),
            SignerInfo {
                is_signed: false,
                signer_name: None,
                signer_fiscal_code: None,
                issuer: None,
                signing_time: None,
                algorithm: None,
                digest_hex: Some(digest),
            },
        ));
    }

    // 2. ASN.1 DER/BER extraction
    let mut extracted_xml: Option<Vec<u8>> = None;
    let mut signer_name: Option<String> = None;
    let mut signer_cf: Option<String> = None;
    let mut issuer: Option<String> = None;
    let mut signing_time: Option<String> = None;

    if let Ok((_tag, val, _)) = decode_tlv(trimmed) {
        find_asn1_payload_and_metadata(
            val,
            &mut extracted_xml,
            &mut signer_name,
            &mut signer_cf,
            &mut issuer,
            &mut signing_time,
        );
    }

    // Fallback: search for XML envelope markers directly in case of non-standard BER encapsulation
    let final_payload = match extracted_xml {
        Some(xml) => xml,
        None => extract_xml_boundary_fallback(trimmed).ok_or(P7mError::MissingPayload)?,
    };

    let digest = format!("{:x}", Sha256::digest(&final_payload));

    Ok((
        final_payload,
        SignerInfo {
            is_signed: true,
            signer_name,
            signer_fiscal_code: signer_cf,
            issuer,
            signing_time,
            algorithm: Some("PKCS#7 / CAdES-BES".to_string()),
            digest_hex: Some(digest),
        },
    ))
}

/// Fallback extractor that locates XML boundary in binary streams
fn extract_xml_boundary_fallback(bytes: &[u8]) -> Option<Vec<u8>> {
    let start_candidates = [
        b"<?xml".as_slice(),
        b"<p:FatturaElettronica".as_slice(),
        b"<FatturaElettronica".as_slice(),
    ];
    let end_candidates = [
        b"</p:FatturaElettronica>".as_slice(),
        b"</FatturaElettronica>".as_slice(),
    ];

    let mut start_pos = None;
    for cand in &start_candidates {
        if let Some(pos) = find_subsequence(bytes, cand) {
            start_pos = match start_pos {
                Some(prev) => Some(std::cmp::min(prev, pos)),
                None => Some(pos),
            };
        }
    }

    let start = start_pos?;

    let mut end_pos = None;
    for cand in &end_candidates {
        if let Some(pos) = rfind_subsequence(&bytes[start..], cand) {
            let abs_pos = start + pos + cand.len();
            end_pos = match end_pos {
                Some(prev) => Some(std::cmp::max(prev, abs_pos)),
                None => Some(abs_pos),
            };
        }
    }

    let end = end_pos?;
    if end > start {
        Some(bytes[start..end].to_vec())
    } else {
        None
    }
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn rfind_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .rposition(|window| window == needle)
}

/// Recursive helper to walk ASN.1 nodes looking for eContent OCTET STRING and signer metadata
fn find_asn1_payload_and_metadata(
    mut slice: &[u8],
    extracted_xml: &mut Option<Vec<u8>>,
    signer_name: &mut Option<String>,
    signer_cf: &mut Option<String>,
    issuer: &mut Option<String>,
    signing_time: &mut Option<String>,
) {
    while !slice.is_empty() {
        if let Ok((tag, val, rem)) = decode_tlv(slice) {
            slice = rem;

            // OCTET STRING (0x04)
            if tag == 0x04 {
                if extracted_xml.is_none() {
                    let trimmed = val.trim_ascii();
                    if trimmed.starts_with(b"<?xml")
                        || trimmed.starts_with(b"<p:FatturaElettronica")
                        || trimmed.starts_with(b"<FatturaElettronica")
                    {
                        *extracted_xml = Some(trimmed.to_vec());
                    }
                }
            }
            // UTCTime (0x17) or GeneralizedTime (0x18)
            else if tag == 0x17 || tag == 0x18 {
                if signing_time.is_none()
                    && let Ok(s) = std::str::from_utf8(val)
                {
                    *signing_time = Some(s.to_string());
                }
            }
            // PrintableString (0x13), UTF8String (0x0C)
            else if tag == 0x13 || tag == 0x0C {
                if let Ok(s) = std::str::from_utf8(val) {
                    if s.len() == 16 && s.chars().all(|c| c.is_ascii_alphanumeric()) {
                        if signer_cf.is_none() {
                            *signer_cf = Some(s.to_string());
                        }
                    } else if s.to_lowercase().contains("ca")
                        || s.to_lowercase().contains("poste")
                        || s.to_lowercase().contains("aruba")
                        || s.to_lowercase().contains("infocert")
                    {
                        if issuer.is_none() {
                            *issuer = Some(s.to_string());
                        }
                    } else if signer_name.is_none() && s.len() > 3 {
                        *signer_name = Some(s.to_string());
                    }
                }
            }
            // Constructed structures: SEQUENCE (0x30), SET (0x31), Context-specific [0..3] (0xA0..0xA3)
            else if (tag & 0x20) != 0 {
                find_asn1_payload_and_metadata(
                    val,
                    extracted_xml,
                    signer_name,
                    signer_cf,
                    issuer,
                    signing_time,
                );
            }
        } else {
            break;
        }
    }
}

/// ASN.1 DER TLV Decoder: returns (tag, value_slice, remainder_slice)
pub fn decode_tlv(input: &[u8]) -> Result<(u8, &[u8], &[u8]), P7mError> {
    if input.is_empty() {
        return Err(P7mError::InvalidAsn1("Unexpected EOF reading tag".into()));
    }

    let tag = input[0];
    let mut cursor = 1;

    if cursor >= input.len() {
        return Err(P7mError::InvalidAsn1(
            "Unexpected EOF reading length".into(),
        ));
    }

    let first_len = input[cursor];
    cursor += 1;

    let length = if (first_len & 0x80) == 0 {
        first_len as usize
    } else {
        let num_bytes = (first_len & 0x7F) as usize;
        if num_bytes == 0 {
            // Indefinite length (BER) - not strict DER, handle gracefully
            return Ok((tag, &input[cursor..], &[]));
        }
        if cursor + num_bytes > input.len() {
            return Err(P7mError::InvalidAsn1("Length overflow".into()));
        }
        let mut len: usize = 0;
        for &b in &input[cursor..cursor + num_bytes] {
            len = (len << 8) | (b as usize);
        }
        cursor += num_bytes;
        len
    };

    if cursor + length > input.len() {
        // Tolerant: slice up to the end if payload was truncated
        return Ok((tag, &input[cursor..], &[]));
    }

    let val = &input[cursor..cursor + length];
    let rem = &input[cursor + length..];
    Ok((tag, val, rem))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_XML: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<p:FatturaElettronica versione="FPR12" xmlns:p="http://ivaservizi.agenziaentrate.gov.it/docs/xsd/fatture/v1.2">
  <FatturaElettronicaHeader>
    <DatiTrasmissione><ProgressivoInvio>1</ProgressivoInvio></DatiTrasmissione>
  </FatturaElettronicaHeader>
  <FatturaElettronicaBody>
    <DatiGenerali><DatiGeneraliDocumento><Numero>123</Numero></DatiGeneraliDocumento></DatiGenerali>
  </FatturaElettronicaBody>
</p:FatturaElettronica>"#;

    fn encode_test_tlv(tag: u8, value: &[u8]) -> Vec<u8> {
        let mut out = vec![tag];
        let len = value.len();
        if len < 128 {
            out.push(len as u8);
        } else {
            out.push(0x82);
            out.push((len >> 8) as u8);
            out.push((len & 0xFF) as u8);
        }
        out.extend_from_slice(value);
        out
    }

    fn build_sample_p7m_fixture(xml: &[u8]) -> Vec<u8> {
        let mut inner = Vec::new();
        inner.extend_from_slice(&encode_test_tlv(0x13, b"Mario Rossi"));
        inner.extend_from_slice(&encode_test_tlv(0x13, b"RSSMRA80A01H501U"));
        inner.extend_from_slice(&encode_test_tlv(0x04, xml));
        encode_test_tlv(0x30, &inner)
    }

    #[test]
    fn test_is_p7m_detection() {
        assert!(!is_p7m(SAMPLE_XML));

        let p7m = build_sample_p7m_fixture(SAMPLE_XML);
        assert!(is_p7m(&p7m));
    }

    #[test]
    fn test_p7m_extraction_and_metadata() {
        let p7m = build_sample_p7m_fixture(SAMPLE_XML);
        let (extracted, info) = inspect_p7m(&p7m).unwrap();
        assert_eq!(extracted, SAMPLE_XML);
        assert!(info.is_signed);
        assert_eq!(info.signer_name.as_deref(), Some("Mario Rossi"));
        assert_eq!(info.signer_fiscal_code.as_deref(), Some("RSSMRA80A01H501U"));
        assert!(info.digest_hex.is_some());
    }

    #[test]
    fn test_raw_xml_extraction() {
        let (extracted, info) = inspect_p7m(SAMPLE_XML).unwrap();
        assert_eq!(extracted, SAMPLE_XML);
        assert!(!info.is_signed);
    }

    #[test]
    fn test_fallback_boundary_extraction() {
        let mut corrupted_p7m = vec![0x30, 0x82, 0x05, 0x00, 0x99, 0x88];
        corrupted_p7m.extend_from_slice(SAMPLE_XML);
        corrupted_p7m.extend_from_slice(&[0x11, 0x22, 0x33]);

        let (extracted, info) = inspect_p7m(&corrupted_p7m).unwrap();
        assert_eq!(extracted, SAMPLE_XML);
        assert!(info.is_signed);
    }
}
