use chrono::Utc;
use std::collections::HashSet;

use crate::error::{AxiomError, Result};
use crate::models::{
    EventArchivePlan, EventArchiveReport, EventQuery, EventRecord, IngestProfile, RetentionClass,
};

use super::AxiomSync;

pub(super) struct ArchiveService<'a> {
    app: &'a AxiomSync,
}

impl<'a> ArchiveService<'a> {
    pub(super) fn new(app: &'a AxiomSync) -> Self {
        Self { app }
    }

    pub(super) fn plan_event_archive(
        &self,
        archive_id: &str,
        query: EventQuery,
        archive_reason: Option<String>,
        archived_by: Option<String>,
    ) -> Result<EventArchivePlan> {
        let archive_id = archive_id.trim();
        if archive_id.is_empty() {
            return Err(AxiomError::Validation(
                "archive_id must not be empty".to_string(),
            ));
        }

        let records = self.app.state.query_events(query.clone())?;
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
        let archive_generated_at = Utc::now().timestamp();

        Ok(EventArchivePlan {
            archive_plan_id: format!("archive-plan-{}", uuid::Uuid::new_v4().simple()),
            archive_id: archive_id.to_string(),
            event_count: records.len(),
            namespace_prefix,
            kind,
            retention,
            object_uri,
            query,
            event_ids: records.into_iter().map(|record| record.event_id).collect(),
            archive_reason,
            archived_by,
            archive_generated_at,
        })
    }

    pub(super) fn execute_event_archive(
        &self,
        plan: EventArchivePlan,
    ) -> Result<EventArchiveReport> {
        let records = self.app.state.query_events(plan.query.clone())?;
        if records.is_empty() {
            return Err(AxiomError::Validation(
                "event archive execute matched no events".to_string(),
            ));
        }
        let planned_ids = plan.event_ids.iter().cloned().collect::<HashSet<_>>();
        let actual_ids = records
            .iter()
            .map(|record| record.event_id.clone())
            .collect::<HashSet<_>>();
        if actual_ids != planned_ids {
            return Err(AxiomError::Validation(
                "event archive execute matched a different event set than the approved plan"
                    .to_string(),
            ));
        }
        let retention = resolve_archive_retention(&records)?;
        if retention != plan.retention {
            return Err(AxiomError::Validation(
                "event archive execute retention no longer matches planned retention".to_string(),
            ));
        }

        let mut buf: Vec<u8> = Vec::with_capacity(records.len() * 256);
        for record in &records {
            serde_json::to_writer(&mut buf, record)?;
            buf.push(b'\n');
        }
        let payload = String::from_utf8(buf)
            .map_err(|e| AxiomError::Internal(format!("archive jsonl utf8 error: {e}")))?;
        self.app.fs.write(&plan.object_uri, &payload, true)?;
        if matches!(retention, RetentionClass::Ephemeral) {
            self.app
                .compact_archived_events(&records, &plan.archive_id, &plan.object_uri)?;
        }

        Ok(EventArchiveReport {
            archive_plan_id: plan.archive_plan_id,
            archive_id: plan.archive_id,
            event_count: records.len(),
            namespace_prefix: plan.namespace_prefix,
            kind: plan.kind,
            retention,
            object_uri: plan.object_uri,
            archive_reason: plan.archive_reason,
            archived_by: plan.archived_by,
            archive_generated_at: plan.archive_generated_at,
            exported_at: Utc::now().timestamp(),
        })
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
}

impl AxiomSync {
    pub fn plan_event_archive(
        &self,
        archive_id: &str,
        query: EventQuery,
        archive_reason: Option<String>,
        archived_by: Option<String>,
    ) -> Result<EventArchivePlan> {
        self.archive_service()
            .plan_event_archive(archive_id, query, archive_reason, archived_by)
    }

    pub fn execute_event_archive(&self, plan: EventArchivePlan) -> Result<EventArchiveReport> {
        self.archive_service().execute_event_archive(plan)
    }

    pub(super) fn compact_archived_events(
        &self,
        records: &[EventRecord],
        archive_id: &str,
        object_uri: &crate::AxiomUri,
    ) -> Result<()> {
        let event_ids: Vec<String> = records
            .iter()
            .map(|record| record.event_id.clone())
            .collect();
        self.state.compact_events_into_archive(
            &event_ids,
            archive_id,
            object_uri,
            Utc::now().timestamp(),
        )?;
        let uris: Vec<String> = records
            .iter()
            .map(|record| record.uri.to_string())
            .collect();
        self.state.remove_search_documents_batch(&uris)?;
        {
            let mut index = self
                .index
                .write()
                .map_err(|_| crate::error::AxiomError::lock_poisoned("index"))?;
            for uri in &uris {
                index.remove(uri);
            }
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
