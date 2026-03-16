use std::fs;
use std::path::Path;

use chrono::Utc;

use crate::error::Result;
use crate::models::{
    MigrationApplyReport, MigrationInspectReport, MigrationRunRecord, RUN_STATUS_SUCCESS,
    ReleaseVerifyReport, RetrievalDoctorReport, StorageDoctorReport,
};
use crate::state::schema::{
    CONTEXT_SCHEMA_VERSION_KEY, INDEX_PROFILE_STAMP_KEY, RELEASE_CONTRACT_VERSION_KEY,
    RUNTIME_RESTORE_SOURCE_KEY, SEARCH_DOCS_FTS_SCHEMA_VERSION_KEY,
};

use super::AxiomSync;

impl AxiomSync {
    pub fn doctor_storage(&self) -> Result<StorageDoctorReport> {
        Ok(StorageDoctorReport {
            context_schema_version: self.state.get_system_value(CONTEXT_SCHEMA_VERSION_KEY)?,
            search_docs_fts_schema_version: self
                .state
                .get_system_value(SEARCH_DOCS_FTS_SCHEMA_VERSION_KEY)?,
            index_profile_stamp: self.state.get_system_value(INDEX_PROFILE_STAMP_KEY)?,
            release_contract_version: self.state.get_system_value(RELEASE_CONTRACT_VERSION_KEY)?,
            search_document_count: self.state.search_document_count()?,
            event_count: self.state.event_count()?,
            link_count: self.state.link_count()?,
            latest_migration_runs: self.state.list_migration_runs(5)?,
            latest_repair_runs: self.state.list_repair_runs(5)?,
        })
    }

    pub fn doctor_retrieval(&self) -> Result<RetrievalDoctorReport> {
        let backend = self.backend_status()?;
        Ok(RetrievalDoctorReport {
            retrieval_backend: backend.retrieval_backend,
            retrieval_backend_policy: backend.retrieval_backend_policy,
            local_records: backend.local_records,
            indexed_documents: self.state.search_document_count()?,
            trace_count: self.state.trace_count()?,
            restore_source: self.state.get_system_value(RUNTIME_RESTORE_SOURCE_KEY)?,
            fts_ready: self.state.fts_ready()?,
        })
    }

    pub fn migrate_inspect(&self) -> Result<MigrationInspectReport> {
        let mut pending_actions = Vec::new();
        if self
            .state
            .get_system_value(CONTEXT_SCHEMA_VERSION_KEY)?
            .is_none()
        {
            pending_actions.push("context_schema_version_missing".to_string());
        }
        if self
            .state
            .get_system_value(SEARCH_DOCS_FTS_SCHEMA_VERSION_KEY)?
            .is_none()
        {
            pending_actions.push("search_docs_fts_schema_version_missing".to_string());
        }
        if self
            .state
            .get_system_value(RELEASE_CONTRACT_VERSION_KEY)?
            .is_none()
        {
            pending_actions.push("release_contract_version_missing".to_string());
        }

        Ok(MigrationInspectReport {
            context_schema_version: self.state.get_system_value(CONTEXT_SCHEMA_VERSION_KEY)?,
            search_docs_fts_schema_version: self
                .state
                .get_system_value(SEARCH_DOCS_FTS_SCHEMA_VERSION_KEY)?,
            release_contract_version: self.state.get_system_value(RELEASE_CONTRACT_VERSION_KEY)?,
            latest_migration_runs: self.state.list_migration_runs(5)?,
            latest_repair_runs: self.state.list_repair_runs(5)?,
            pending_actions,
        })
    }

    pub fn migrate_apply(&self, backup_dir: Option<&Path>) -> Result<MigrationApplyReport> {
        let inspect_before = self.migrate_inspect()?;
        let backup_path = backup_dir
            .map(|dir| -> Result<String> {
                fs::create_dir_all(dir)?;
                let target = dir.join(format!("context-{}.db", uuid::Uuid::new_v4().simple()));
                fs::copy(self.fs.root().join("context.db"), &target)?;
                Ok(target.to_string_lossy().to_string())
            })
            .transpose()?;

        let started_at = Utc::now().to_rfc3339();
        self.state.ensure_schema()?;
        let finished_at = Utc::now().to_rfc3339();
        let run = MigrationRunRecord {
            run_id: format!("migration-{}", uuid::Uuid::new_v4().simple()),
            operation: "ensure_schema".to_string(),
            started_at,
            finished_at: Some(finished_at),
            status: RUN_STATUS_SUCCESS.to_string(),
            details: Some(serde_json::json!({
                "backup_path": backup_path,
            })),
        };
        self.state.record_migration_run(&run)?;
        let inspect_after = self.migrate_inspect()?;

        Ok(MigrationApplyReport {
            backup_path,
            inspect_before,
            inspect_after,
            applied_run: run,
        })
    }

    pub fn release_verify(&self) -> Result<ReleaseVerifyReport> {
        Ok(ReleaseVerifyReport {
            verified_at: Utc::now().to_rfc3339(),
            storage: self.doctor_storage()?,
            retrieval: self.doctor_retrieval()?,
        })
    }
}
