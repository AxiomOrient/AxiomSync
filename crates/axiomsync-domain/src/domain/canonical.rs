use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            let ordered: BTreeMap<_, _> = map.iter().collect();
            for (key, value) in ordered {
                out.insert(key.clone(), canonical_json(value));
            }
            Value::Object(out)
        }
        Value::Array(values) => Value::Array(values.iter().map(canonical_json).collect()),
        other => other.clone(),
    }
}

pub fn canonical_json_string(value: &Value) -> String {
    serde_json::to_string(&canonical_json(value)).expect("canonical JSON")
}

pub fn stable_hash(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0x1f]);
    }
    hex::encode(hasher.finalize())
}

pub fn stable_id(prefix: &str, value: &impl Serialize) -> String {
    let serialized = serde_json::to_value(value).expect("serializable id value");
    let canonical = canonical_json_string(&serialized);
    format!(
        "{prefix}_{}",
        &stable_hash(&[prefix, canonical.as_str()])[..16]
    )
}

pub fn workspace_stable_id(canonical_root: &str) -> String {
    stable_id("ws", &canonical_root)
}

pub fn normalize_fts_query(query: &str) -> Option<String> {
    let tokens = query
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|token| token.len() >= 2)
        .map(|token| token.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join(" "))
    }
}
