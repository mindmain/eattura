//! Deterministic SHA-256 names for persisted JSON content.
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

fn normalize(value: &mut Value) {
    match value {
        Value::String(text) => {
            *text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        }
        Value::Array(values) => values.iter_mut().for_each(normalize),
        Value::Object(fields) => fields.values_mut().for_each(normalize),
        _ => {}
    }
}

/// Hash canonical JSON after folding whitespace within string values.
pub(crate) fn content_id(value: &impl Serialize) -> Result<String, String> {
    let mut value = serde_json::to_value(value).map_err(|e| e.to_string())?;
    normalize(&mut value);
    let bytes = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

/// Stable client identity based on the fiscal fields, which are immutable once referenced.
pub(crate) fn client_id(country: &str, tax_code: &str) -> Result<String, String> {
    content_id(&serde_json::json!({"kind": "client", "country": country, "taxCode": tax_code}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn whitespace_and_key_order_do_not_change_the_hash() {
        let a = json!({"name":" A  B\n C ","count":1});
        let b = json!({"count":1,"name":"A B C"});
        assert_eq!(content_id(&a).unwrap(), content_id(&b).unwrap());
        assert_ne!(
            content_id(&a).unwrap(),
            content_id(&json!({"name":"A B D","count":1})).unwrap()
        );
    }
}
