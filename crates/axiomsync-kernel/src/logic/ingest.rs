use serde_json::json;

use crate::domain::{
    ConnectorBatchInput, CursorInput, ExistingRawEventKey, ImportJournalRow, IngestPlan,
    NormalizedRawEvent, RawEventInput, SourceCursorRow, canonical_json_string, stable_hash,
    stable_id,
};
use crate::error::Result;

pub fn normalize_raw_event(input: &RawEventInput) -> Result<NormalizedRawEvent> {
    input.validate()?;
    let payload_json = canonical_json_string(&input.payload);
    let payload_sha256_hex = stable_hash(&[payload_json.as_str()]);
    let row = crate::domain::RawEventRow {
        stable_id: stable_id(
            "raw_event",
            &json!({
                "source": input.source,
                "native_session_id": input.native_session_id,
                "native_event_id": input.native_event_id,
                "event_type": input.event_type,
                "ts_ms": input.ts_ms,
                "payload_sha256": payload_sha256_hex,
            }),
        ),
        connector: input.source.clone(),
        native_schema_version: input.native_schema_version.clone(),
        native_session_id: input.native_session_id.clone(),
        native_event_id: input.native_event_id.clone(),
        event_type: input.event_type.clone(),
        ts_ms: input.ts_ms,
        payload_json,
        payload_sha256_hex: payload_sha256_hex.clone(),
    };
    let dedupe_key = stable_id(
        "dedupe",
        &json!({
            "connector": row.connector,
            "native_session_id": row.native_session_id,
            "native_event_id": row.native_event_id,
            "event_type": row.event_type,
            "ts_ms": row.ts_ms,
            "payload_sha256": row.payload_sha256_hex,
        }),
    );
    Ok(NormalizedRawEvent { row, dedupe_key })
}

#[must_use]
pub fn deterministic_directory_cursor(
    events: &[RawEventInput],
    latest_path: Option<&str>,
) -> Option<CursorInput> {
    let latest_path = latest_path.map(str::trim).filter(|path| !path.is_empty())?;
    let source = events.first()?.source.as_str();
    Some(CursorInput {
        cursor_key: format!("{source}_directory"),
        cursor_value: latest_path.to_string(),
        updated_at_ms: events
            .iter()
            .map(|event| event.ts_ms)
            .max()
            .unwrap_or_default(),
    })
}

pub fn plan_ingest(
    existing: &[ExistingRawEventKey],
    input: &ConnectorBatchInput,
) -> Result<IngestPlan> {
    let mut adds = Vec::new();
    let mut skipped = Vec::new();
    let mut seen_keys = existing
        .iter()
        .map(|row| row.dedupe_key.clone())
        .collect::<std::collections::HashSet<_>>();
    for event in &input.events {
        let normalized = normalize_raw_event(event)?;
        if seen_keys.contains(&normalized.dedupe_key) {
            skipped.push(normalized.dedupe_key);
            continue;
        }
        seen_keys.insert(normalized.dedupe_key.clone());
        adds.push(normalized);
    }

    let cursor_update = input.cursor.as_ref().and_then(|cursor| {
        input.events.first().map(|event| SourceCursorRow {
            connector: event.source.clone(),
            cursor_key: cursor.cursor_key.clone(),
            cursor_value: cursor.cursor_value.clone(),
            updated_at_ms: cursor.updated_at_ms,
        })
    });
    let journal = input.events.first().map(|event| ImportJournalRow {
        stable_id: stable_id(
            "journal",
            &json!({
                "source": event.source,
                "imported_events": adds.len(),
                "skipped_events": skipped.len(),
                "cursor": cursor_update,
            }),
        ),
        connector: event.source.clone(),
        imported_events: adds.len(),
        skipped_events: skipped.len(),
        cursor_key: cursor_update.as_ref().map(|row| row.cursor_key.clone()),
        cursor_value: cursor_update.as_ref().map(|row| row.cursor_value.clone()),
        applied_at_ms: cursor_update
            .as_ref()
            .map(|row| row.updated_at_ms)
            .unwrap_or_else(|| {
                input
                    .events
                    .iter()
                    .map(|event| event.ts_ms)
                    .max()
                    .unwrap_or_default()
            }),
    });

    let plan = IngestPlan {
        adds,
        cursor_update,
        skipped_dedupe_keys: skipped,
        journal,
    };
    plan.validate()?;
    Ok(plan)
}
