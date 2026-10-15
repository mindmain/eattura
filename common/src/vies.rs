use serde::{Deserialize, Serialize};

/// Standardized VAT Lookup response from VIES / official registries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VatLookupResponse {
    pub valid: bool,
    pub country_code: String,
    pub vat_number: String,
    pub name: Option<String>,
    pub address: Option<String>,
    pub street: Option<String>,
    pub postal_code: Option<String>,
    pub city: Option<String>,
    pub province: Option<String>,
    pub user_error: Option<String>,
}

/// Raw JSON response returned by the European Commission VIES REST API:
/// `https://ec.europa.eu/taxation_customs/vies/rest-api/ms/{countryCode}/vat/{vatNumber}`
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawViesResponse {
    #[serde(default)]
    pub is_valid: bool,
    pub request_date: Option<String>,
    pub user_error: Option<String>,
    pub name: Option<String>,
    pub address: Option<String>,
    pub request_identifier: Option<String>,
    pub vat_number: Option<String>,
}

impl RawViesResponse {
    pub fn into_vat_lookup_response(self, country_code: &str, vat_number: &str) -> VatLookupResponse {
        let name = self.name.and_then(|n| {
            let t = n.trim();
            if t.is_empty() || t == "---" {
                None
            } else {
                Some(t.to_string())
            }
        });

        let address = self.address.and_then(|a| {
            let t = a.trim();
            if t.is_empty() || t == "---" {
                None
            } else {
                Some(t.to_string())
            }
        });

        let (street, postal_code, city, province) = if let Some(ref addr) = address {
            parse_vies_address(country_code, addr)
        } else {
            (None, None, None, None)
        };

        VatLookupResponse {
            valid: self.is_valid,
            country_code: country_code.to_uppercase(),
            vat_number: vat_number.to_string(),
            name,
            address,
            street,
            postal_code,
            city,
            province,
            user_error: self.user_error,
        }
    }
}

/// Normalize country code and VAT number.
/// Removes spaces, dots, dashes, and duplicate country prefix.
/// Greece uses `EL` in VIES, so `GR` is normalized to `EL`.
pub fn normalize_vat(country_code: &str, vat_number: &str) -> (String, String) {
    let raw_country = country_code.trim().to_uppercase();
    let mut vat = vat_number.trim().to_uppercase().replace([' ', '.', '-'], "");
    if vat.starts_with(&raw_country) {
        vat = vat[raw_country.len()..].to_string();
    } else if raw_country == "GR" && vat.starts_with("EL") {
        vat = vat[2..].to_string();
    } else if raw_country == "EL" && vat.starts_with("GR") {
        vat = vat[2..].to_string();
    }
    let country = if raw_country == "GR" {
        "EL".to_string()
    } else {
        raw_country
    };
    (country, vat)
}

/// Parse structured address components from VIES address string.
pub fn parse_vies_address(
    country_code: &str,
    raw_address: &str,
) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    let raw = raw_address.trim();
    if raw.is_empty() || raw == "---" {
        return (None, None, None, None);
    }

    if country_code == "IT" {
        parse_italian_vies_address(raw)
    } else {
        (Some(raw.to_string()), None, None, None)
    }
}

fn parse_italian_vies_address(
    raw: &str,
) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    let lines: Vec<&str> = raw
        .split('\n')
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    if lines.len() >= 2 {
        let street = lines[0].to_string();
        let locality_line = lines[1..].join(" ");
        let (cap, city, prov) = parse_cap_city_prov(&locality_line);
        (Some(street), cap, city, prov)
    } else {
        let parts: Vec<&str> = raw
            .split(',')
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .collect();
        if parts.len() >= 2 {
            let street = parts[0].to_string();
            let locality = parts[1..].join(" ");
            let (cap, city, prov) = parse_cap_city_prov(&locality);
            (Some(street), cap, city, prov)
        } else {
            let (cap, city, prov) = parse_cap_city_prov(raw);
            (Some(raw.to_string()), cap, city, prov)
        }
    }
}

fn parse_cap_city_prov(s: &str) -> (Option<String>, Option<String>, Option<String>) {
    let tokens: Vec<&str> = s.split_whitespace().collect();
    let mut cap: Option<String> = None;
    let mut prov: Option<String> = None;
    let mut city_tokens: Vec<&str> = Vec::new();

    for token in tokens {
        let clean = token.trim_matches(|c: char| !c.is_alphanumeric());
        if clean.len() == 5 && clean.chars().all(|c| c.is_ascii_digit()) && cap.is_none() {
            cap = Some(clean.to_string());
        } else if clean.len() == 2
            && clean.chars().all(|c| c.is_ascii_uppercase())
            && !clean.chars().any(|c| c.is_ascii_digit())
        {
            prov = Some(clean.to_string());
        } else if !clean.is_empty() {
            city_tokens.push(clean);
        }
    }

    let city = if !city_tokens.is_empty() {
        Some(city_tokens.join(" "))
    } else {
        None
    };

    (cap, city, prov)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_vat() {
        let (c, v) = normalize_vat("IT", "IT 01234567890");
        assert_eq!(c, "IT");
        assert_eq!(v, "01234567890");

        let (c2, v2) = normalize_vat("gr", "GR 123456789");
        assert_eq!(c2, "EL");
        assert_eq!(v2, "123456789");
    }

    #[test]
    fn test_parse_italian_vies_address_multiline() {
        let raw = "VIA SAN MARCO 12\n35129 PADOVA PD\n";
        let (street, cap, city, prov) = parse_vies_address("IT", raw);
        assert_eq!(street.as_deref(), Some("VIA SAN MARCO 12"));
        assert_eq!(cap.as_deref(), Some("35129"));
        assert_eq!(city.as_deref(), Some("PADOVA"));
        assert_eq!(prov.as_deref(), Some("PD"));
    }

    #[test]
    fn test_parse_italian_vies_address_comma_separated() {
        let raw = "PIAZZA DEL DUOMO 1, 20121 MILANO MI";
        let (street, cap, city, prov) = parse_vies_address("IT", raw);
        assert_eq!(street.as_deref(), Some("PIAZZA DEL DUOMO 1"));
        assert_eq!(cap.as_deref(), Some("20121"));
        assert_eq!(city.as_deref(), Some("MILANO"));
        assert_eq!(prov.as_deref(), Some("MI"));
    }

    #[test]
    fn test_raw_vies_into_response() {
        let raw = RawViesResponse {
            is_valid: true,
            request_date: Some("2026-03-15T10:00:00Z".to_string()),
            user_error: Some("VALID".to_string()),
            name: Some("ACME TEST SRL".to_string()),
            address: Some("VIA NAZIONALE 100\n00184 ROMA RM".to_string()),
            request_identifier: Some("WAPIAAA123".to_string()),
            vat_number: Some("01234567890".to_string()),
        };

        let res = raw.into_vat_lookup_response("IT", "01234567890");
        assert!(res.valid);
        assert_eq!(res.name.as_deref(), Some("ACME TEST SRL"));
        assert_eq!(res.street.as_deref(), Some("VIA NAZIONALE 100"));
        assert_eq!(res.postal_code.as_deref(), Some("00184"));
        assert_eq!(res.city.as_deref(), Some("ROMA"));
        assert_eq!(res.province.as_deref(), Some("RM"));
    }
}
