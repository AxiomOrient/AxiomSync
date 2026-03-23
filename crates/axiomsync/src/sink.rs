use std::fs;
use std::path::Path;

use crate::domain::{AppendRawEventsRequest, UpsertSourceCursorRequest};
use crate::error::Result;

pub fn load_json_file<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

pub fn load_append_request(path: &Path) -> Result<AppendRawEventsRequest> {
    load_json_file(path)
}

pub fn load_cursor_request(path: &Path) -> Result<UpsertSourceCursorRequest> {
    load_json_file(path)
}
