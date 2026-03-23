use axiomsync_domain::domain::{
    AppendRawEventsRequest, IngestPlan, IngressReceiptRow, SourceCursorRow, SourceCursorUpsertPlan,
    UpsertSourceCursorRequest, canonical_json_string, stable_id,
};
use axiomsync_domain::error::Result;

pub fn plan_append_raw_events(
    request: &AppendRawEventsRequest,
    existing_dedupe_keys: &[String],
) -> Result<IngestPlan> {
    request.validate()?;
    let batch_id = request
        .batch_id
        .clone()
        .unwrap_or_else(|| stable_id("batch", &request.request_id));
    let mut receipts = Vec::new();
    let mut skipped = Vec::new();
    for event in &request.events {
        let dedupe_key = event
            .dedupe_key
            .clone()
            .or_else(|| {
                Some(stable_id(
                    "dedupe",
                    &(
                        event.normalized_source_kind(),
                        event.normalized_session_kind(),
                        event.normalized_session_key().ok(),
                        event.external_entry_key.as_deref(),
                        event.normalized_event_kind().ok(),
                        event.normalized_observed_at().ok(),
                        event.normalized_content_hash().ok(),
                    ),
                ))
            });
        if let Some(key) = dedupe_key.as_ref()
            && existing_dedupe_keys.iter().any(|existing| existing == key)
        {
            skipped.push(key.clone());
            continue;
        }
        let artifacts_json = serde_json::to_string(&event.artifacts)?;
        let payload_json = canonical_json_string(&event.payload);
        let raw_payload_json = event
            .raw_payload
            .as_ref()
            .map(canonical_json_string);
        let receipt_id = stable_id(
            "receipt",
            &(
                batch_id.as_str(),
                event.normalized_source_kind(),
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
            source_kind: event.normalized_source_kind().to_string(),
            connector: event.source.clone(),
            session_kind: event.normalized_session_kind().to_string(),
            external_session_key: Some(event.normalized_session_key()?),
            external_entry_key: event.external_entry_key.clone(),
            event_kind: event.normalized_event_kind()?,
            observed_at: event.normalized_observed_at()?,
            captured_at: event.captured_at.clone(),
            workspace_root: event.workspace_root.clone(),
            content_hash: event.normalized_content_hash()?,
            dedupe_key,
            payload_json,
            raw_payload_json,
            artifacts_json,
        });
    }
    Ok(IngestPlan {
        receipts,
        cursor_update: None,
        skipped_dedupe_keys: skipped,
    })
}

pub fn plan_source_cursor_upsert(
    request: &UpsertSourceCursorRequest,
) -> Result<SourceCursorUpsertPlan> {
    request.validate()?;
    Ok(SourceCursorUpsertPlan {
        cursor: SourceCursorRow {
            connector: request.source.clone(),
            cursor_key: request.cursor.cursor_key.clone(),
            cursor_value: request.cursor.cursor_value.clone(),
            updated_at: request.cursor.normalized_updated_at()?,
        },
    })
}
