use std::path::Path;

use chrono::Utc;

use crate::error::{AxiomError, Result};
use crate::models::{
    AddResourceRequest, IngestProfile, RepoMountReport, RepoMountRequest, ResourceRecord,
    UpsertResource,
};

use super::AxiomSync;

pub(super) struct RepoService<'a> {
    app: &'a AxiomSync,
}

impl<'a> RepoService<'a> {
    pub(super) fn new(app: &'a AxiomSync) -> Self {
        Self { app }
    }

    pub(super) fn mount_repo(&self, req: RepoMountRequest) -> Result<RepoMountReport> {
        let source_path = Path::new(&req.source_path);
        if !source_path.exists() || !source_path.is_dir() {
            return Err(AxiomError::Validation(format!(
                "repo mount source must be an existing directory: {}",
                req.source_path
            )));
        }

        let add_result = self
            .app
            .add_resource_with_ingest_options(AddResourceRequest {
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
        self.app.state.persist_resource(UpsertResource {
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

    fn persist_resource_and_sync_index(&self, resource: &ResourceRecord) -> Result<()> {
        let profile = IngestProfile::for_kind(&resource.kind);
        self.app
            .state
            .persist_resource_search_document(resource, &profile)?;
        self.sync_runtime_index(&resource.uri.to_string())
    }

    fn sync_runtime_index(&self, uri: &str) -> Result<()> {
        let record = self.app.state.get_search_document(uri)?;
        let mut index = self
            .app
            .index
            .write()
            .map_err(|_| AxiomError::lock_poisoned("index"))?;
        index.remove(uri);
        if let Some(record) = record {
            index.upsert(record);
        }
        Ok(())
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
        self.app.fs.write_bytes(&object_uri, &payload, true)?;
        Ok(object_uri)
    }

    fn resource_id_for_uri(target_uri: &crate::AxiomUri) -> String {
        blake3::hash(target_uri.to_string().as_bytes())
            .to_hex()
            .to_string()
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
}

impl AxiomSync {
    pub fn mount_repo(&self, req: RepoMountRequest) -> Result<RepoMountReport> {
        self.repo_service().mount_repo(req)
    }
}
