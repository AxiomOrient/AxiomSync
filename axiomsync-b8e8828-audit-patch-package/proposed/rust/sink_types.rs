// compile-oriented skeleton, not build-verified

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceCursorInput {
    pub connector: String,
    pub cursor_key: String,
    pub cursor_value: String,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceCursorRow {
    pub connector: String,
    pub cursor_key: String,
    pub cursor_value: String,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceCursorUpsertPlan {
    pub row: SourceCursorRow,
}

impl SourceCursorInput {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(!self.connector.trim().is_empty(), "connector required");
        anyhow::ensure!(!self.cursor_key.trim().is_empty(), "cursor_key required");
        anyhow::ensure!(!self.cursor_value.trim().is_empty(), "cursor_value required");
        Ok(())
    }
}

impl SourceCursorUpsertPlan {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(!self.row.connector.trim().is_empty(), "connector required");
        anyhow::ensure!(!self.row.cursor_key.trim().is_empty(), "cursor_key required");
        anyhow::ensure!(!self.row.cursor_value.trim().is_empty(), "cursor_value required");
        Ok(())
    }
}
