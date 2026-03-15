use std::path::Path;

use chrono::Utc;

use crate::error::{AxiomError, Result};
use crate::models::{
    AddEventRequest, AddResourceRequest, EventArchiveReport, EventQuery, EventRecord,
    IngestProfile, LinkRecord, LinkRequest, RepoMountReport, RepoMountRequest, ResourceRecord,
    RetentionClass, UpsertResource,
};

use super::AxiomSync;

const EXTERNALIZE_INLINE_JSON_BYTES: usize = 4 * 1024;

impl AxiomSync {
    pub fn add_event(&self, req: AddEventRequest) -> Result<EventRecord> {
        self.add_events(vec![req])?
            .into_iter()
            .next()
            .ok_or_else(|| AxiomError::Internal("add_event returned empty batch".to_string()))
    }

    pub fn add_events(&self, batch: Vec<AddEventRequest>) -> Result<Vec<EventRecord>> {
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

        self.state.append_events(&records)?;
        for record in &records {
            let profile = IngestProfile::for_kind(&record.kind);
            self.state.persist_event_search_document(record, &profile)?;
        }
        self.sync_events_runtime_index(&records)?;
        Ok(records)
    }

    pub fn link_records(&self, req: LinkRequest) -> Result<LinkRecord> {
        let created_at = req.created_at.unwrap_or_else(|| Utc::now().timestamp());
        let record = LinkRecord {
            link_id: req.link_id,
            namespace: req.namespace,
            from_uri: req.from_uri,
            relation: req.relation.trim().to_ascii_lowercase(),
            to_uri: req.to_uri,
            weight: req.weight,
            attrs: req.attrs,
            created_at,
        };
        self.state.persist_link(&record)?;
        Ok(record)
    }

    pub fn export_event_archive(
        &self,
        archive_id: &str,
        query: EventQuery,
    ) -> Result<EventArchiveReport> {
        let archive_id = archive_id.trim();
        if archive_id.is_empty() {
            return Err(AxiomError::Validation(
                "archive_id must not be empty".to_string(),
            ));
        }

        let records = self.state.query_events(query.clone())?;
        if records.is_empty() {
            return Err(AxiomError::Validation(
                "event archive export matched no events".to_string(),
            ));
        }

        let retention = resolve_archive_retention(&records)?;
        let namespace_prefix = query.namespace_prefix.clone();
        let kind = query.kind.clone();
        let object_uri = self
            .event_archive_object_uri(namespace_prefix.as_ref(), kind.as_ref(), archive_id)
            .map_err(|err| {
                AxiomError::Internal(format!("failed to resolve event archive object uri: {err}"))
            })?;
        let payload = records
            .iter()
            .map(serde_json::to_string)
            .collect::<std::result::Result<Vec<_>, _>>()?
            .join("\n");
        self.fs.write(&object_uri, &payload, true)?;
        if matches!(retention, RetentionClass::Ephemeral) {
            self.compact_archived_events(&records, archive_id, &object_uri)?;
        }

        Ok(EventArchiveReport {
            archive_id: archive_id.to_string(),
            event_count: records.len(),
            namespace_prefix,
            kind,
            retention,
            object_uri,
            exported_at: Utc::now().timestamp(),
        })
    }

    pub fn mount_repo(&self, req: RepoMountRequest) -> Result<RepoMountReport> {
        let source_path = Path::new(&req.source_path);
        if !source_path.exists() || !source_path.is_dir() {
            return Err(AxiomError::Validation(format!(
                "repo mount source must be an existing directory: {}",
                req.source_path
            )));
        }

        let add_result = self.add_resource_with_ingest_options(AddResourceRequest {
            source: req.source_path.clone(),
            target: Some(req.target_uri.to_string()),
            wait: req.wait,
            timeout_secs: None,
            wait_mode: crate::models::AddResourceWaitMode::Relaxed,
            ingest_options: crate::models::AddResourceIngestOptions::default(),
        })?;

        let now = Utc::now().timestamp();
        let resource_id = Self::resource_id_for_uri(&req.target_uri);
        let repo_object_uri =
            self.write_resource_mount_object(&req.target_uri, &req.namespace, &req.kind, &req)?;
        let resource = ResourceRecord {
            resource_id,
            uri: req.target_uri.clone(),
            namespace: req.namespace,
            kind: req.kind,
            title: req.title.clone().or_else(|| {
                source_path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .map(ToString::to_string)
            }),
            mime: None,
            tags: req.tags,
            attrs: req.attrs,
            object_uri: Some(repo_object_uri),
            excerpt_text: req.title,
            content_hash: blake3::hash(req.source_path.as_bytes())
                .to_hex()
                .to_string(),
            tombstoned_at: None,
            created_at: now,
            updated_at: now,
        };
        self.state.persist_resource(UpsertResource {
            resource_id: resource.resource_id.clone(),
            uri: resource.uri.clone(),
            namespace: resource.namespace.clone(),
            kind: resource.kind.clone(),
            title: resource.title.clone(),
            mime: resource.mime.clone(),
            tags: resource.tags.clone(),
            attrs: resource.attrs.clone(),
            object_uri: resource.object_uri.clone(),
            excerpt_text: resource.excerpt_text.clone(),
            content_hash: resource.content_hash.clone(),
            tombstoned_at: None,
            created_at: resource.created_at,
            updated_at: resource.updated_at,
        })?;
        self.persist_resource_and_sync_index(&resource)?;

        Ok(RepoMountReport {
            root_uri: req.target_uri,
            resource,
            queued: add_result.queued,
        })
    }

    fn sync_events_runtime_index(&self, records: &[EventRecord]) -> Result<()> {
        let uris: Vec<String> = records.iter().map(|r| r.uri.to_string()).collect();
        let docs = self.state.get_search_documents_batch(&uris)?;
        let mut index = self
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

    fn persist_resource_and_sync_index(&self, resource: &ResourceRecord) -> Result<()> {
        let profile = IngestProfile::for_kind(&resource.kind);
        self.state
            .persist_resource_search_document(resource, &profile)?;
        self.sync_runtime_index(&resource.uri.to_string())
    }

    fn sync_runtime_index(&self, uri: &str) -> Result<()> {
        let record = self.state.get_search_document(uri)?;
        let mut index = self
            .index
            .write()
            .map_err(|_| AxiomError::lock_poisoned("index"))?;
        index.remove(uri);
        if let Some(record) = record {
            index.upsert(record);
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
        self.fs.write_bytes(&object_uri, &serialized, true)?;

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

    fn write_resource_mount_object(
        &self,
        target_uri: &crate::AxiomUri,
        namespace: &crate::models::NamespaceKey,
        kind: &crate::models::Kind,
        req: &RepoMountRequest,
    ) -> Result<crate::AxiomUri> {
        let resource_id = Self::resource_id_for_uri(target_uri);
        let object_uri = self
            .resource_object_uri(namespace, kind, &resource_id)
            .map_err(|err| {
                AxiomError::Internal(format!("failed to resolve resource object uri: {err}"))
            })?;
        let payload = serde_json::to_vec_pretty(&serde_json::json!({
            "source_path": req.source_path,
            "target_uri": target_uri.to_string(),
            "namespace": namespace.as_path(),
            "kind": kind.as_str(),
            "title": req.title,
            "tags": req.tags,
            "attrs": req.attrs,
        }))?;
        self.fs.write_bytes(&object_uri, &payload, true)?;
        Ok(object_uri)
    }

    fn resource_id_for_uri(target_uri: &crate::AxiomUri) -> String {
        blake3::hash(target_uri.to_string().as_bytes())
            .to_hex()
            .to_string()
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

    fn event_archive_object_uri(
        &self,
        namespace: Option<&crate::models::NamespaceKey>,
        kind: Option<&crate::models::Kind>,
        archive_id: &str,
    ) -> std::result::Result<crate::AxiomUri, crate::AxiomError> {
        let mut uri = crate::AxiomUri::root(crate::Scope::Events).join("_archive")?;
        if let Some(namespace) = namespace {
            uri = uri.join(&namespace.as_path())?;
        }
        if let Some(kind) = kind {
            uri = uri.join(kind.as_str())?;
        }
        uri.join(&format!("{archive_id}.jsonl"))
    }

    fn resource_object_uri(
        &self,
        namespace: &crate::models::NamespaceKey,
        kind: &crate::models::Kind,
        resource_id: &str,
    ) -> std::result::Result<crate::AxiomUri, crate::AxiomError> {
        let mut uri = crate::AxiomUri::root(crate::Scope::Resources)
            .join("_objects")?
            .join(&namespace.as_path())?
            .join(kind.as_str())?;
        uri = uri.join(&format!("{resource_id}.json"))?;
        Ok(uri)
    }

    fn compact_archived_events(
        &self,
        records: &[EventRecord],
        archive_id: &str,
        object_uri: &crate::AxiomUri,
    ) -> Result<()> {
        let event_ids = records
            .iter()
            .map(|record| record.event_id.clone())
            .collect::<Vec<_>>();
        self.state.compact_events_into_archive(
            &event_ids,
            archive_id,
            object_uri,
            Utc::now().timestamp(),
        )?;
        for record in records {
            self.state.remove_search_document(&record.uri.to_string())?;
            self.sync_runtime_index(&record.uri.to_string())?;
        }
        Ok(())
    }
}

fn resolve_archive_retention(records: &[EventRecord]) -> Result<RetentionClass> {
    let mut retention = records
        .first()
        .map(|record| IngestProfile::for_kind(&record.kind).retention)
        .ok_or_else(|| {
            AxiomError::Validation("event archive export matched no events".to_string())
        })?;
    for record in records.iter().skip(1) {
        let next = IngestProfile::for_kind(&record.kind).retention;
        if next != retention {
            return Err(AxiomError::Validation(
                "event archive export requires events with the same retention class".to_string(),
            ));
        }
        retention = next;
    }
    Ok(retention)
}
