use rusqlite::types::Value;

use crate::models::{Kind, NamespaceKey};
use crate::uri::AxiomUri;

pub(super) fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut normalized = tags
        .iter()
        .map(|tag| tag.trim().to_ascii_lowercase())
        .filter(|tag| !tag.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
}

pub(super) fn parse_json_column<T: serde::de::DeserializeOwned>(
    raw: String,
) -> rusqlite::Result<T> {
    serde_json::from_str(&raw).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
    })
}

pub(super) fn parse_uri(raw: String) -> rusqlite::Result<AxiomUri> {
    AxiomUri::parse(&raw).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
    })
}

pub(super) fn parse_optional_uri(raw: Option<String>) -> rusqlite::Result<Option<AxiomUri>> {
    raw.map(parse_uri).transpose()
}

pub(super) fn parse_namespace(raw: String) -> rusqlite::Result<NamespaceKey> {
    NamespaceKey::parse(&raw).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
    })
}

pub(super) fn parse_kind(raw: String) -> rusqlite::Result<Kind> {
    Kind::new(raw).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
    })
}

/// Appends a namespace prefix filter to a dynamic SQL string and parameter list.
///
/// Matches rows where `field` equals the prefix exactly, or starts with `prefix/`.
/// Uses a range bound (`\x7f`, ASCII 127) instead of LIKE to allow index range scans.
pub(super) fn push_namespace_range_filter(
    sql: &mut String,
    params: &mut Vec<Value>,
    field: &str,
    namespace: &str,
) {
    sql.push_str(&format!(
        " AND ({field} = ? OR ({field} > ? AND {field} < ?))"
    ));
    params.push(Value::Text(namespace.to_owned()));
    params.push(Value::Text(format!("{namespace}/")));
    params.push(Value::Text(format!("{namespace}/\x7f")));
}
