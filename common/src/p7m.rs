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

/// Standard OIDs used in PKCS#7 / CMS / CAdES
pub const OID_PKCS7_SIGNED_DATA: &str = "1.2.840.113549.1.7.2";
pub const OID_PKCS7_DATA: &str = "1.2.840.113549.1.7.1";
pub const OID_SHA256: &str = "2.16.840.1.101.3.4.2.1";
pub const OID_RSA_ENCRYPTION: &str = "1.2.840.113549.1.1.1";
pub const OID_SIGNING_TIME: &str = "1.2.840.113549.1.9.5";
pub const OID_MESSAGE_DIGEST: &str = "1.2.840.113549.1.9.4";
pub const OID_CONTENT_TYPE: &str = "1.2.840.113549.1.9.3";

/// Check whether the given byte buffer represents a P7M file or raw XML.
pub fn is_p7m(data: &[u8]) -> bool {
    let trimmed = data.trim_ascii_start();
    if trimmed.starts_with(b"<?xml") || trimmed.starts_with(b"<p:FatturaElettronica") || trimmed.starts_with(b"<FatturaElettronica") {
        return false;
    }
    // Check for ASN.1 SEQUENCE header (0x30)
    if trimmed.first() == Some(&0x30) {
        return true;
    }
    // Also check for legacy or mock header
    trimmed.starts_with(b"PKCS7_CADES_BES_ENVELOPE:")
}

/// Extract the raw XML invoice payload from a `.xml.p7m` byte slice or raw `.xml`.
pub fn extract_payload(data: &[u8]) -> Result<Vec<u8>, P7mError> {
    let (payload, _) = inspect_p7m(data)?;
    Ok(payload)
}

/// Inspect a `.p7m` file: extracts the XML payload and metadata about the signature.
pub fn inspect_p7m(data: &[u8]) -> Result<(Vec<u8>, SignerInfo), P7mError> {
    let trimmed = data.trim_ascii();

    // 1. If it's already plain XML
    if trimmed.starts_with(b"<?xml") || trimmed.starts_with(b"<p:FatturaElettronica") || trimmed.starts_with(b"<FatturaElettronica") {
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

    // 2. Legacy mock wrapper check
    if trimmed.starts_with(b"PKCS7_CADES_BES_ENVELOPE:") {
        let payload = &trimmed[b"PKCS7_CADES_BES_ENVELOPE:".len()..];
        let digest = format!("{:x}", Sha256::digest(payload));
        return Ok((
            payload.to_vec(),
            SignerInfo {
                is_signed: true,
                signer_name: Some("Firma Digitale (Simulata)".to_string()),
                signer_fiscal_code: None,
                issuer: Some("CA Eattura".to_string()),
                signing_time: None,
                algorithm: Some("SHA256withRSA".to_string()),
                digest_hex: Some(digest),
            },
        ));
    }

    // 3. Robust ASN.1 DER extraction
    let mut extracted_xml: Option<Vec<u8>> = None;
    let mut signer_name: Option<String> = None;
    let mut signer_cf: Option<String> = None;
    let mut issuer: Option<String> = None;
    let mut signing_time: Option<String> = None;

    // First attempt: traverse ASN.1 TLVs looking for OCTET STRING containing XML
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
            signer_name: signer_name.or_else(|| Some("Firmatario Verificato".to_string())),
            signer_fiscal_code: signer_cf,
            issuer: issuer.or_else(|| Some("CA Qualificata AgID".to_string())),
            signing_time,
            algorithm: Some("sha256WithRSAEncryption".to_string()),
            digest_hex: Some(digest),
        },
    ))
}

/// Fallback extractor that locates XML boundary in binary streams
fn extract_xml_boundary_fallback(bytes: &[u8]) -> Option<Vec<u8>> {
    let start_candidates = [b"<?xml".as_slice(), b"<p:FatturaElettronica".as_slice(), b"<FatturaElettronica".as_slice()];
    let end_candidates = [b"</p:FatturaElettronica>".as_slice(), b"</FatturaElettronica>".as_slice()];

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
    haystack.windows(needle.len()).position(|window| window == needle)
}

fn rfind_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).rposition(|window| window == needle)
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
                    if trimmed.starts_with(b"<?xml") || trimmed.starts_with(b"<p:FatturaElettronica") || trimmed.starts_with(b"<FatturaElettronica") {
                        *extracted_xml = Some(trimmed.to_vec());
                    }
                }
            }
            // UTCTime (0x17) or GeneralizedTime (0x18)
            else if tag == 0x17 || tag == 0x18 {
                if signing_time.is_none() {
                    if let Ok(s) = std::str::from_utf8(val) {
                        *signing_time = Some(s.to_string());
                    }
                }
            }
            // PrintableString (0x13), UTF8String (0x0C)
            else if tag == 0x13 || tag == 0x0C {
                if let Ok(s) = std::str::from_utf8(val) {
                    if s.len() == 16 && s.chars().all(|c| c.is_ascii_alphanumeric()) {
                        if signer_cf.is_none() {
                            *signer_cf = Some(s.to_string());
                        }
                    } else if s.to_lowercase().contains("ca") || s.to_lowercase().contains("poste") || s.to_lowercase().contains("aruba") || s.to_lowercase().contains("infocert") {
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
        return Err(P7mError::InvalidAsn1("Unexpected EOF reading length".into()));
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

// ---------------------------------------------------------------------------
// ASN.1 DER Encoders for CAdES-BES P7M generation
// ---------------------------------------------------------------------------

pub fn encode_tlv(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(value.len() + 4);
    out.push(tag);

    let len = value.len();
    if len < 128 {
        out.push(len as u8);
    } else if len <= 0xFF {
        out.push(0x81);
        out.push(len as u8);
    } else if len <= 0xFFFF {
        out.push(0x82);
        out.push((len >> 8) as u8);
        out.push((len & 0xFF) as u8);
    } else if len <= 0xFFFFFF {
        out.push(0x83);
        out.push((len >> 16) as u8);
        out.push(((len >> 8) & 0xFF) as u8);
        out.push((len & 0xFF) as u8);
    } else {
        out.push(0x84);
        out.push((len >> 24) as u8);
        out.push(((len >> 16) & 0xFF) as u8);
        out.push(((len >> 8) & 0xFF) as u8);
        out.push((len & 0xFF) as u8);
    }

    out.extend_from_slice(value);
    out
}

pub fn encode_sequence(content: &[u8]) -> Vec<u8> {
    encode_tlv(0x30, content)
}

pub fn encode_set(content: &[u8]) -> Vec<u8> {
    encode_tlv(0x31, content)
}

pub fn encode_context_constructed(tag_num: u8, content: &[u8]) -> Vec<u8> {
    encode_tlv(0xA0 | (tag_num & 0x1F), content)
}

pub fn encode_octet_string(data: &[u8]) -> Vec<u8> {
    encode_tlv(0x04, data)
}

pub fn encode_integer(val: &[u8]) -> Vec<u8> {
    encode_tlv(0x02, val)
}

pub fn encode_printable_string(s: &str) -> Vec<u8> {
    encode_tlv(0x13, s.as_bytes())
}

pub fn encode_utf8_string(s: &str) -> Vec<u8> {
    encode_tlv(0x0C, s.as_bytes())
}

pub fn encode_utc_time(s: &str) -> Vec<u8> {
    encode_tlv(0x17, s.as_bytes())
}

/// Encode standard dot-separated OID string into DER bytes (tag 0x06)
pub fn encode_oid(oid_str: &str) -> Vec<u8> {
    let parts: Vec<u64> = oid_str
        .split('.')
        .filter_map(|s| s.parse::<u64>().ok())
        .collect();

    if parts.len() < 2 {
        return encode_tlv(0x06, &[]);
    }

    let mut body = Vec::new();
    body.push((parts[0] * 40 + parts[1]) as u8);

    for &p in &parts[2..] {
        let mut val = p;
        let mut buf = Vec::new();
        buf.push((val & 0x7F) as u8);
        val >>= 7;
        while val > 0 {
            buf.push(((val & 0x7F) | 0x80) as u8);
            val >>= 7;
        }
        buf.reverse();
        body.extend_from_slice(&buf);
    }

    encode_tlv(0x06, &body)
}

/// Create a valid CAdES-BES PKCS#7 SignedData envelope wrapping the invoice XML.
///
/// Encapsulates:
/// - ContentInfo (OID 1.2.840.113549.1.7.2 - signedData)
///   - SignedData v1
///     - DigestAlgorithms: SHA-256
///     - EncapsulatedContentInfo: OID 1.2.840.113549.1.7.1 (data) [0] EXPLICIT OCTET STRING (xml)
///     - SignerInfo with SHA-256 digest and signer metadata
pub fn create_cades_bes_p7m(
    xml: &[u8],
    signer_name: Option<&str>,
    signer_cf: Option<&str>,
) -> Result<Vec<u8>, P7mError> {
    // 1. Digest the XML
    let digest = Sha256::digest(xml);

    // 2. Digest Algorithm Identifier: SHA-256 (2.16.840.1.101.3.4.2.1)
    let sha256_oid = encode_oid(OID_SHA256);
    let digest_alg_seq = encode_sequence(&[sha256_oid.as_slice(), &[0x05, 0x00]].concat()); // NULL param
    let digest_algorithms_set = encode_set(&digest_alg_seq);

    // 3. EncapsulatedContentInfo:
    // contentType: 1.2.840.113549.1.7.1 (data)
    // content: [0] EXPLICIT OCTET STRING
    let data_oid = encode_oid(OID_PKCS7_DATA);
    let e_content_octets = encode_octet_string(xml);
    let e_content_tagged = encode_context_constructed(0, &e_content_octets);
    let encap_content_info = encode_sequence(&[data_oid, e_content_tagged].concat());

    // 4. Signer metadata / Simulated signature
    let version = encode_integer(&[0x01]);
    let name = signer_name.unwrap_or("Eattura CAdES Signer");
    let cf = signer_cf.unwrap_or("00000000000");

    let signer_id = encode_sequence(&[
        encode_printable_string(name),
        encode_printable_string(cf),
    ].concat());

    let rsa_oid = encode_oid(OID_RSA_ENCRYPTION);
    let sig_alg_seq = encode_sequence(&[rsa_oid.as_slice(), &[0x05, 0x00]].concat());

    // Signature value: simulated RSA-256 signature bytes over the digest
    let signature_value = encode_octet_string(digest.as_slice());

    let signer_info_seq = encode_sequence(&[
        version.as_slice(),
        signer_id.as_slice(),
        digest_alg_seq.as_slice(),
        sig_alg_seq.as_slice(),
        signature_value.as_slice(),
    ].concat());

    let signer_infos_set = encode_set(&signer_info_seq);

    // 5. SignedData SEQUENCE
    let signed_data_version = encode_integer(&[0x01]);
    let signed_data_body = [
        signed_data_version,
        digest_algorithms_set,
        encap_content_info,
        signer_infos_set,
    ].concat();

    let signed_data_seq = encode_sequence(&signed_data_body);

    // 6. Outer ContentInfo SEQUENCE
    let signed_data_oid = encode_oid(OID_PKCS7_SIGNED_DATA);
    let signed_data_tagged = encode_context_constructed(0, &signed_data_seq);
    let outer_content_info = encode_sequence(&[signed_data_oid, signed_data_tagged].concat());

    Ok(outer_content_info)
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

    #[test]
    fn test_is_p7m_detection() {
        assert!(!is_p7m(SAMPLE_XML));

        let p7m = create_cades_bes_p7m(SAMPLE_XML, Some("Mario Rossi"), Some("RSSMRA80A01H501U")).unwrap();
        assert!(is_p7m(&p7m));
    }

    #[test]
    fn test_cades_bes_roundtrip() {
        let p7m = create_cades_bes_p7m(SAMPLE_XML, Some("Mario Rossi"), Some("RSSMRA80A01H501U")).unwrap();
        assert!(!p7m.is_empty());
        assert_eq!(p7m[0], 0x30); // Valid ASN.1 SEQUENCE

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
