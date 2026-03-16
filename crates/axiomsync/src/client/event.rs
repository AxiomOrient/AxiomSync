use chrono::Utc;

use crate::error::{AxiomError, Result};
use crate::models::{AddEventRequest, EventRecord, IngestProfile};

use super::AxiomSync;

const EXTERNALIZE_INLINE_JSON_BYTES: usize = 4 * 1024;

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

        self.app.state.append_events(&records)?;
        for record in &records {
            let profile = IngestProfile::for_kind(&record.kind);
            self.app
                .state
                .persist_event_search_document(record, &profile)?;
        }
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

        let raw_payload = attrs.get("raw_payload").cloned();
        let serialized = serde_json::to_vec(attrs)?;
        let should_externalize =
            raw_payload.is_some() || serialized.len() > EXTERNALIZE_INLINE_JSON_BYTES;
        if !should_externalize {
            return Ok((attrs.clone(), None, None));
        }

        let object_uri = self
            .event_object_uri(namespace, kind, event_id)
            .map_err(|err| {
                AxiomError::Internal(format!("failed to resolve event object uri: {err}"))
            })?;
        self.app.fs.write_bytes(&object_uri, &serialized, true)?;

        let mut trimmed = attrs.clone();
        if let Some(object) = trimmed.as_object_mut() {
            object.remove("raw_payload");
            object.insert(
                "externalized".to_string(),
                serde_json::json!({
                    "bytes": serialized.len(),
                    "object_uri": object_uri.to_string(),
                }),
            );
        }

        Ok((
            trimmed,
            Some(object_uri),
            Some(blake3::hash(&serialized).to_hex().to_string()),
        ))
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
