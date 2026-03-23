use super::*;

impl ContextDb {
    pub fn load_workspaces(&self) -> Result<Vec<WorkspaceRow>> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "select stable_id, canonical_root, repo_remote, branch, worktree_path
             from workspace order by stable_id",
            )
            .map_db_err()?;
        let rows = stmt
            .query_map([], |row| {
                Ok(WorkspaceRow {
                    stable_id: row.get(0)?,
                    canonical_root: row.get(1)?,
                    repo_remote: row.get(2)?,
                    branch: row.get(3)?,
                    worktree_path: row.get(4)?,
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }

    pub fn load_sessions(&self) -> Result<Vec<ConvSessionRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select conv_session.stable_id, conv_session.connector, conv_session.native_session_id,
                    workspace.stable_id, conv_session.title, conv_session.transcript_uri,
                    conv_session.status, conv_session.started_at_ms, conv_session.ended_at_ms
             from conv_session
             left join workspace on workspace.id = conv_session.workspace_id
             order by conv_session.stable_id",
        ).map_db_err()?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ConvSessionRow {
                    stable_id: row.get(0)?,
                    connector: row.get(1)?,
                    native_session_id: row.get(2)?,
                    workspace_id: row.get(3)?,
                    title: row.get(4)?,
                    transcript_uri: row.get(5)?,
                    status: row.get(6)?,
                    started_at_ms: row.get(7)?,
                    ended_at_ms: row.get(8)?,
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }

    pub fn load_turns(&self) -> Result<Vec<ConvTurnRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select conv_turn.stable_id, conv_session.stable_id, conv_turn.native_turn_id, conv_turn.turn_index, conv_turn.actor
             from conv_turn
             join conv_session on conv_session.id = conv_turn.session_id
             order by conv_session.stable_id, conv_turn.turn_index, conv_turn.stable_id",
        ).map_db_err()?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ConvTurnRow {
                    stable_id: row.get(0)?,
                    session_id: row.get(1)?,
                    native_turn_id: row.get(2)?,
                    turn_index: row.get::<_, i64>(3)? as usize,
                    actor: row.get(4)?,
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }

    pub fn load_items(&self) -> Result<Vec<ConvItemRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select conv_item.stable_id, conv_turn.stable_id, conv_item.item_type, conv_item.tool_name, conv_item.body_text, conv_item.payload_json
             from conv_item
             join conv_turn on conv_turn.id = conv_item.turn_id
             order by conv_turn.stable_id, conv_item.stable_id",
        ).map_db_err()?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ConvItemRow {
                    stable_id: row.get(0)?,
                    turn_id: row.get(1)?,
                    item_type: crate::context_db::codec::parse_item_type(row.get(2)?)?,
                    tool_name: row.get(3)?,
                    body_text: row.get(4)?,
                    payload_json: row.get(5)?,
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }

    pub fn load_artifacts(&self) -> Result<Vec<ArtifactRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select artifact.stable_id, conv_item.stable_id, artifact.uri, artifact.mime, hex(artifact.sha256), artifact.bytes
             from artifact
             join conv_item on conv_item.id = artifact.item_id
             order by artifact.stable_id",
        ).map_db_err()?;
        let rows = stmt
            .query_map([], |row| {
                let sha256: Option<String> = row.get(4)?;
                Ok(ArtifactRow {
                    stable_id: row.get(0)?,
                    item_id: row.get(1)?,
                    uri: row.get(2)?,
                    mime: row.get(3)?,
                    sha256_hex: sha256.map(|value| value.to_ascii_lowercase()),
                    bytes: row.get::<_, Option<i64>>(5)?.map(|value| value as u64),
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }

    pub fn load_evidence_anchors(&self) -> Result<Vec<EvidenceAnchorRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select evidence_anchor.stable_id, conv_item.stable_id, evidence_anchor.selector_type, evidence_anchor.selector_json, evidence_anchor.quoted_text
             from evidence_anchor
             join conv_item on conv_item.id = evidence_anchor.item_id
             order by evidence_anchor.stable_id",
        ).map_db_err()?;
        let rows = stmt
            .query_map([], |row| {
                Ok(EvidenceAnchorRow {
                    stable_id: row.get(0)?,
                    item_id: row.get(1)?,
                    selector_type: crate::context_db::codec::parse_selector_type(row.get(2)?)?,
                    selector_json: row.get(3)?,
                    quoted_text: row.get(4)?,
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }

    pub fn load_execution_runs(&self) -> Result<Vec<ExecutionRunRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select stable_id, run_id, workspace_id, producer, mission_id, flow_id, mode, status, started_at_ms, updated_at_ms, last_event_type
             from execution_run
             order by updated_at_ms asc, stable_id asc",
        ).map_db_err()?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ExecutionRunRow {
                    stable_id: row.get(0)?,
                    run_id: row.get(1)?,
                    workspace_id: row.get(2)?,
                    producer: row.get(3)?,
                    mission_id: row.get(4)?,
                    flow_id: row.get(5)?,
                    mode: row.get(6)?,
                    status: row.get(7)?,
                    started_at_ms: row.get(8)?,
                    updated_at_ms: row.get(9)?,
                    last_event_type: row.get(10)?,
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }

    pub fn load_execution_tasks(&self) -> Result<Vec<ExecutionTaskRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select stable_id, run_id, task_id, workspace_id, producer, title, status, owner_role, updated_at_ms
             from execution_task
             order by updated_at_ms asc, stable_id asc",
        ).map_db_err()?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ExecutionTaskRow {
                    stable_id: row.get(0)?,
                    run_id: row.get(1)?,
                    task_id: row.get(2)?,
                    workspace_id: row.get(3)?,
                    producer: row.get(4)?,
                    title: row.get(5)?,
                    status: row.get(6)?,
                    owner_role: row.get(7)?,
                    updated_at_ms: row.get(8)?,
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }

    pub fn load_execution_checks(&self) -> Result<Vec<ExecutionCheckRow>> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "select stable_id, run_id, task_id, name, status, details, updated_at_ms
             from execution_check
             order by updated_at_ms asc, stable_id asc",
            )
            .map_db_err()?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ExecutionCheckRow {
                    stable_id: row.get(0)?,
                    run_id: row.get(1)?,
                    task_id: row.get(2)?,
                    name: row.get(3)?,
                    status: row.get(4)?,
                    details: row.get(5)?,
                    updated_at_ms: row.get(6)?,
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }

    pub fn load_execution_approvals(&self) -> Result<Vec<ExecutionApprovalRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select stable_id, run_id, task_id, approval_id, kind, status, resume_token, updated_at_ms
             from execution_approval
             order by updated_at_ms asc, stable_id asc",
        ).map_db_err()?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ExecutionApprovalRow {
                    stable_id: row.get(0)?,
                    run_id: row.get(1)?,
                    task_id: row.get(2)?,
                    approval_id: row.get(3)?,
                    kind: row.get(4)?,
                    status: row.get(5)?,
                    resume_token: row.get(6)?,
                    updated_at_ms: row.get(7)?,
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }

    pub fn load_execution_events(&self) -> Result<Vec<ExecutionEventRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select stable_id, raw_event_id, run_id, task_id, producer, role, event_type, status, body_text, occurred_at_ms
             from execution_event
             order by occurred_at_ms asc, stable_id asc",
        ).map_db_err()?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ExecutionEventRow {
                    stable_id: row.get(0)?,
                    raw_event_id: row.get(1)?,
                    run_id: row.get(2)?,
                    task_id: row.get(3)?,
                    producer: row.get(4)?,
                    role: row.get(5)?,
                    event_type: row.get(6)?,
                    status: row.get(7)?,
                    body_text: row.get(8)?,
                    occurred_at_ms: row.get(9)?,
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }

    pub fn load_document_records(&self) -> Result<Vec<DocumentRecordRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select stable_id, document_id, workspace_id, producer, kind, path, title, body_text, artifact_uri, artifact_mime, hex(artifact_sha256), artifact_bytes, updated_at_ms, raw_event_id
             from document_record
             order by updated_at_ms asc, stable_id asc",
        ).map_db_err()?;
        let rows = stmt
            .query_map([], |row| {
                let sha256: Option<String> = row.get(10)?;
                Ok(DocumentRecordRow {
                    stable_id: row.get(0)?,
                    document_id: row.get(1)?,
                    workspace_id: row.get(2)?,
                    producer: row.get(3)?,
                    kind: row.get(4)?,
                    path: row.get(5)?,
                    title: row.get(6)?,
                    body_text: row.get(7)?,
                    artifact_uri: row.get(8)?,
                    artifact_mime: row.get(9)?,
                    artifact_sha256_hex: sha256.map(|value| value.to_ascii_lowercase()),
                    artifact_bytes: row.get::<_, Option<i64>>(11)?.map(|value| value as u64),
                    updated_at_ms: row.get(12)?,
                    raw_event_id: row.get(13)?,
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }
}
