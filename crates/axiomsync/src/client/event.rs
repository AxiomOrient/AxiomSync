use chrono::Utc;

use crate::error::{AxiomError, Result};
use crate::models::{AddEventRequest, EventRecord};

use super::AxiomSync;

const EXTERNALIZE_INLINE_JSON_BYTES: usize = 4 * 1024;
const MAX_EVENT_ID_BYTES: usize = 256;

// ── Pure data functions ───────────────────────────────────────────────────────

/// Returns `true` when `attrs` should be written to an external object store rather than
/// stored inline in the database.  Pure predicate: no I/O, no side effects.
fn needs_externalization(attrs: &serde_json::Value, serialized_len: usize) -> bool {
    attrs.get("raw_payload").is_some() || serialized_len > EXTERNALIZE_INLINE_JSON_BYTES
}

/// Builds the trimmed inline attrs that replace the full payload after externalization.
/// Removes `raw_payload` and inserts an `externalized` metadata object. Pure transform.
fn build_externalized_attrs(
    attrs: &serde_json::Value,
    object_uri: &crate::AxiomUri,
    bytes: usize,
) -> serde_json::Value {
    let mut trimmed = attrs.clone();
    if let Some(obj) = trimmed.as_object_mut() {
        obj.remove("raw_payload");
        obj.insert(
            "externalized".to_string(),
            serde_json::json!({
                "bytes": bytes,
                "object_uri": object_uri.to_string(),
            }),
        );
    }
    trimmed
}

pub(super) struct EventService<'a> {
    app: &'a AxiomSync,
}

impl<'a> EventService<'a> {
    pub(super) fn new(app: &'a AxiomSync) -> Self {
        Self { app }
    }

    pub(super) fn add_event(&self, req: AddEventRequest) -> Result<EventRecord> {
        self.add_events(vec![req])?
            .into_iter()
            .next()
            .ok_or_else(|| AxiomError::Internal("add_event returned empty batch".to_string()))
    }

    pub(super) fn add_events(&self, batch: Vec<AddEventRequest>) -> Result<Vec<EventRecord>> {
        if batch.is_empty() {
            return Ok(Vec::new());
        }

        let records = batch
            .into_iter()
            .map(|req| -> Result<EventRecord> {
                if req.event_id.is_empty() {
                    return Err(AxiomError::Validation("event_id must not be empty".to_string()));
                }
                if req.event_id.len() > MAX_EVENT_ID_BYTES {
                    return Err(AxiomError::Validation(format!(
                        "event_id exceeds {MAX_EVENT_ID_BYTES} bytes"
                    )));
                }
                let created_at = req.created_at.unwrap_or_else(|| Utc::now().timestamp());
                let (attrs, object_uri, content_hash) = self.write_event_object_if_needed(
                    &req.namespace,
                    &req.kind,
                    &req.event_id,
                    &req.attrs,
                    req.object_uri,
                )?;
                Ok(EventRecord {
                    event_id: req.event_id,
                    uri: req.uri,
                    namespace: req.namespace,
                    kind: req.kind,
                    event_time: req.event_time,
                    title: req.title,
                    summary_text: req.summary_text,
                    severity: req.severity,
                    actor_uri: req.actor_uri,
                    subject_uri: req.subject_uri,
                    run_id: req.run_id,
                    session_id: req.session_id,
                    tags: req.tags,
                    attrs,
                    object_uri,
                    content_hash: content_hash.or(req.content_hash),
                    tombstoned_at: None,
                    created_at,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        self.app.state.append_events_and_search_docs(&records)?;
        self.sync_runtime_index(&records)?;
        Ok(records)
    }

    fn sync_runtime_index(&self, records: &[EventRecord]) -> Result<()> {
        let uris: Vec<String> = records.iter().map(|r| r.uri.to_string()).collect();
        let docs = self.app.state.get_search_documents_batch(&uris)?;
        let mut index = self
            .app
            .index
            .write()
            .map_err(|_| AxiomError::lock_poisoned("index"))?;
        for uri in &uris {
            index.remove(uri);
            if let Some(record) = docs.get(uri) {
                index.upsert(record.clone());
            }
        }
        Ok(())
    }

    /// Externalizes `attrs` to the object store when the payload is large or contains
    /// `raw_payload`. Returns `(inline_attrs, object_uri, content_hash)`.
    ///
    /// The decision and data transformation are handled by the pure functions
    /// `needs_externalization` and `build_externalized_attrs`; this method is
    /// responsible only for the file-system write.
    ///
    /// When `existing_object_uri` is `Some`, the object was already written by the caller
    /// (e.g. during a replay or re-import).  In that case the attrs are passed through
    /// unchanged and no content_hash is re-derived — the caller is assumed to have already
    /// provided a valid hash if needed.
    fn write_event_object_if_needed(
        &self,
        namespace: &crate::models::NamespaceKey,
        kind: &crate::models::Kind,
        event_id: &str,
        attrs: &serde_json::Value,
        existing_object_uri: Option<crate::AxiomUri>,
    ) -> Result<(serde_json::Value, Option<crate::AxiomUri>, Option<String>)> {
        if existing_object_uri.is_some() {
            return Ok((attrs.clone(), existing_object_uri, None));
        }

        let payload = serde_json::to_vec(attrs)?;
        if !needs_externalization(attrs, payload.len()) {
            return Ok((attrs.clone(), None, None));
        }

        let object_uri = self
            .event_object_uri(namespace, kind, event_id)
            .map_err(|err| {
                AxiomError::Internal(format!("failed to resolve event object uri: {err}"))
            })?;
        self.app.fs.write_bytes(&object_uri, &payload, true)?;

        let content_hash = blake3::hash(&payload).to_hex().to_string();
        let inline_attrs = build_externalized_attrs(attrs, &object_uri, payload.len());

        Ok((inline_attrs, Some(object_uri), Some(content_hash)))
    }

    fn event_object_uri(
        &self,
        namespace: &crate::models::NamespaceKey,
        kind: &crate::models::Kind,
        event_id: &str,
    ) -> std::result::Result<crate::AxiomUri, crate::AxiomError> {
        let mut uri = crate::AxiomUri::root(crate::Scope::Events)
            .join("_objects")?
            .join(&namespace.as_path())?
            .join(kind.as_str())?;
        uri = uri.join(&format!("{event_id}.json"))?;
        Ok(uri)
    }
}

impl AxiomSync {
    pub fn add_event(&self, req: AddEventRequest) -> Result<EventRecord> {
        self.event_service().add_event(req)
    }

    pub fn add_events(&self, batch: Vec<AddEventRequest>) -> Result<Vec<EventRecord>> {
        self.event_service().add_events(batch)
    }
}
