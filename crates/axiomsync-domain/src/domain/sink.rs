use serde::{Deserialize, Serialize};

use crate::error::{AxiomError, Result};

use super::{CursorInput, RawEventInput};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppendRawEventsRequest {
    pub request_id: Option<String>,
    pub events: Vec<RawEventInput>,
}

impl AppendRawEventsRequest {
    pub fn validate(&self) -> Result<()> {
        if self.events.is_empty() {
            return Err(AxiomError::Validation(
                "append_raw_events requires at least one event".to_string(),
            ));
        }
        for event in &self.events {
            event.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpsertSourceCursorRequest {
    #[serde(rename = "source", alias = "connector")]
    pub source: String,
    pub cursor: CursorInput,
}

impl UpsertSourceCursorRequest {
    pub fn validate(&self) -> Result<()> {
        if self.source.trim().is_empty() {
            return Err(AxiomError::Validation(
                "upsert_source_cursor requires source".to_string(),
            ));
        }
        self.cursor.validate()
    }
}
