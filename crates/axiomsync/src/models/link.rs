use serde::{Deserialize, Serialize};

use crate::uri::AxiomUri;

use super::NamespaceKey;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkRecord {
    pub link_id: String,
    pub namespace: NamespaceKey,
    pub from_uri: AxiomUri,
    pub relation: String,
    pub to_uri: AxiomUri,
    pub weight: f32,
    pub attrs: serde_json::Value,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkRequest {
    pub link_id: String,
    pub namespace: NamespaceKey,
    pub from_uri: AxiomUri,
    pub relation: String,
    pub to_uri: AxiomUri,
    pub weight: f32,
    #[serde(default)]
    pub attrs: serde_json::Value,
    pub created_at: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LinkQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace_prefix: Option<NamespaceKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_uri: Option<AxiomUri>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_uri: Option<AxiomUri>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}
