use std::collections::HashSet;

use axiomsync_domain::error::Result;
use axiomsync_domain::{
    AppendRawEventsRequest, IngestPlan, IngressReceiptRow, RawArtifactInput, SourceCursorRow,
    SourceCursorUpsertPlan, UpsertSourceCursorRequest, canonical_json_string, stable_id,
};
use serde_json::{Value, json};

pub fn plan_append_raw_events(
    request: &AppendRawEventsRequest,
    existing_dedupe_keys: &[String],
) -> Result<IngestPlan> {
    request.validate()?;
    let batch_id = request.batch_id.clone();
    let mut receipts = Vec::new();
    let mut skipped = Vec::new();
    let mut seen_dedupe_keys = existing_dedupe_keys.iter().cloned().collect::<HashSet<_>>();
    for event in &request.events {
        let connector = event.connector.clone();
        let source_kind = request.producer.clone();
        let workspace_root = event.normalized_workspace_root();
        let artifacts = normalized_artifacts(event)?;
        let dedupe_key = computed_dedupe_key(source_kind.as_str(), event)?;
        if let Some(key) = dedupe_key.as_ref() {
            if seen_dedupe_keys.contains(key) {
                skipped.push(key.clone());
                continue;
            }
            seen_dedupe_keys.insert(key.clone());
        }
        let artifacts_json = serde_json::to_string(&artifacts)?;
        let payload_json = canonical_json_string(&event.payload);
        let normalized_json = canonical_json_string(&json!({
            "producer": source_kind.clone(),
            "connector": connector.clone(),
            "session_kind": event.normalized_session_kind(),
            "workspace_root": workspace_root.clone(),
            "event_kind": event.normalized_event_kind()?,
            "external_session_key": event.normalized_session_key()?,
            "external_entry_key": event.external_entry_key.clone(),
            "observed_at": event.normalized_observed_at()?,
            "captured_at": event.normalized_captured_at()?,
            "payload": event.payload.clone(),
            "hints": event.hints.clone(),
            "artifacts": artifacts.clone(),
        }));
        let raw_payload_json = event.raw_payload.as_ref().map(canonical_json_string);
        let receipt_id = stable_id(
            "receipt",
            &(
                batch_id.as_str(),
                source_kind.as_str(),
                event.normalized_session_key()?,
                event.external_entry_key.as_deref(),
                event.normalized_event_kind()?,
                event.normalized_observed_at()?,
                event.normalized_content_hash()?,
            ),
        );
        receipts.push(IngressReceiptRow {
            receipt_id,
            batch_id: batch_id.clone(),
            source_kind,
            connector,
            session_kind: event.normalized_session_kind().to_string(),
            external_session_key: Some(event.normalized_session_key()?),
            external_entry_key: event.external_entry_key.clone(),
            event_kind: event.normalized_event_kind()?,
            observed_at: event.normalized_observed_at()?,
            captured_at: event.normalized_captured_at()?,
            workspace_root,
            content_hash: event.normalized_content_hash()?,
            dedupe_key,
            payload_json,
            raw_payload_json,
            artifacts_json,
            normalized_json,
            projection_state: "pending".to_string(),
            derived_state: "pending".to_string(),
            index_state: "pending".to_string(),
        });
    }
    Ok(IngestPlan {
        receipts,
        cursor_update: None,
        skipped_dedupe_keys: skipped,
    })
}

pub fn dedupe_candidates(request: &AppendRawEventsRequest) -> Result<Vec<String>> {
    request.validate()?;
    let mut candidates = Vec::with_capacity(request.events.len());
    for event in &request.events {
        if let Some(key) = computed_dedupe_key(request.producer.as_str(), event)? {
            candidates.push(key);
        }
    }
    candidates.sort();
    candidates.dedup();
    Ok(candidates)
}

pub fn plan_source_cursor_upsert(
    request: &UpsertSourceCursorRequest,
) -> Result<SourceCursorUpsertPlan> {
    request.validate()?;
    Ok(SourceCursorUpsertPlan {
        cursor: SourceCursorRow {
            connector: request.connector.clone(),
            cursor_key: request.cursor_key.clone(),
            cursor_value: request.cursor_value.clone(),
            updated_at: axiomsync_domain::ts_ms_to_rfc3339(request.updated_at_ms)?,
            metadata_json: Value::Object(serde_json::Map::new()),
        },
    })
}

fn normalized_artifacts(event: &axiomsync_domain::RawEventInput) -> Result<Vec<RawArtifactInput>> {
    let mut artifacts = event.artifacts.clone();
    if let Some(values) = event.payload.get("artifacts").and_then(Value::as_array) {
        for value in values {
            let artifact_kind = value
                .get("artifact_kind")
                .and_then(Value::as_str)
                .filter(|raw| !raw.trim().is_empty())
                .unwrap_or("file")
                .to_string();
            let uri = value
                .get("uri")
                .or_else(|| value.get("path"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if uri.trim().is_empty() {
                continue;
            }
            let artifact = RawArtifactInput {
                artifact_kind,
                uri,
                mime_type: value
                    .get("mime_type")
                    .or_else(|| value.get("mime"))
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                sha256: value
                    .get("sha256")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                size_bytes: value
                    .get("size_bytes")
                    .or_else(|| value.get("bytes"))
                    .and_then(Value::as_i64),
                metadata_json: value.clone(),
            };
            artifact.validate()?;
            artifacts.push(artifact);
        }
    }
    Ok(artifacts)
}

fn computed_dedupe_key(
    source_kind: &str,
    event: &axiomsync_domain::RawEventInput,
) -> Result<Option<String>> {
    Ok(event.dedupe_key.clone().or_else(|| {
        Some(stable_id(
            "dedupe",
            &(
                source_kind,
                event.normalized_session_kind(),
                event.normalized_session_key().ok(),
                event.external_entry_key.as_deref(),
                event.normalized_event_kind().ok(),
                event.normalized_observed_at().ok(),
                event.normalized_content_hash().ok(),
            ),
        ))
    }))
}
