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
}
