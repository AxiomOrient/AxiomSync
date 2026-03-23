use super::*;

impl ContextDb {
    pub(crate) fn apply_projection_in_tx(
        tx: &rusqlite::Transaction<'_>,
        plan: &ProjectionPlan,
    ) -> Result<serde_json::Value> {
        plan.validate()?;
        tx.execute("delete from execution_event", []).map_db_err()?;
        tx.execute("delete from execution_check", []).map_db_err()?;
        tx.execute("delete from execution_approval", [])
            .map_db_err()?;
        tx.execute("delete from execution_task", []).map_db_err()?;
        tx.execute("delete from execution_run", []).map_db_err()?;
        tx.execute("delete from document_record", []).map_db_err()?;
        tx.execute("delete from artifact", []).map_db_err()?;
        tx.execute("delete from evidence_anchor", []).map_db_err()?;
        tx.execute("delete from conv_item", []).map_db_err()?;
        tx.execute("delete from conv_turn", []).map_db_err()?;
        tx.execute("delete from conv_session", []).map_db_err()?;
        tx.execute("delete from workspace", []).map_db_err()?;

        for workspace in &plan.workspaces {
            tx.execute(
                "insert into workspace (stable_id, canonical_root, repo_remote, branch, worktree_path)
                 values (?1, ?2, ?3, ?4, ?5)",
                params![
                    workspace.stable_id,
                    workspace.canonical_root,
                    workspace.repo_remote,
                    workspace.branch,
                    workspace.worktree_path
                ],
            )
            .map_db_err()?;
        }
        let workspace_ids = stable_id_map(tx, "workspace")?;

        for session in &plan.conv_sessions {
            tx.execute(
                "insert into conv_session (stable_id, connector, native_session_id, workspace_id, title, transcript_uri, status, started_at_ms, ended_at_ms)
                 values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    session.stable_id,
                    session.connector,
                    session.native_session_id,
                    lookup_fk(&workspace_ids, session.workspace_id.as_deref())?,
                    session.title,
                    session.transcript_uri,
                    session.status,
                    session.started_at_ms,
                    session.ended_at_ms
                ],
            )
            .map_db_err()?;
        }
        let session_ids = stable_id_map(tx, "conv_session")?;

        for turn in &plan.conv_turns {
            tx.execute(
                "insert into conv_turn (stable_id, session_id, native_turn_id, turn_index, actor)
                 values (?1, ?2, ?3, ?4, ?5)",
                params![
                    turn.stable_id,
                    lookup_fk(&session_ids, Some(turn.session_id.as_str()))?,
                    turn.native_turn_id,
                    turn.turn_index as i64,
                    turn.actor
                ],
            )
            .map_db_err()?;
        }
        let turn_ids = stable_id_map(tx, "conv_turn")?;

        for item in &plan.conv_items {
            tx.execute(
                "insert into conv_item (stable_id, turn_id, item_type, tool_name, body_text, payload_json)
                 values (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    item.stable_id,
                    lookup_fk(&turn_ids, Some(item.turn_id.as_str()))?,
                    item.item_type.as_str(),
                    item.tool_name,
                    item.body_text,
                    item.payload_json
                ],
            )
            .map_db_err()?;
        }
        let item_ids = stable_id_map(tx, "conv_item")?;

        for artifact in &plan.artifacts {
            tx.execute(
                "insert into artifact (stable_id, item_id, uri, mime, sha256, bytes)
                 values (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    artifact.stable_id,
                    lookup_fk(&item_ids, Some(artifact.item_id.as_str()))?,
                    artifact.uri,
                    artifact.mime,
                    artifact
                        .sha256_hex
                        .as_deref()
                        .map(hex_to_bytes)
                        .transpose()?,
                    artifact.bytes.map(|value| value as i64)
                ],
            )
            .map_db_err()?;
        }

        for anchor in &plan.evidence_anchors {
            tx.execute(
                "insert into evidence_anchor (stable_id, item_id, selector_type, selector_json, quoted_text)
                 values (?1, ?2, ?3, ?4, ?5)",
                params![
                    anchor.stable_id,
                    lookup_fk(&item_ids, Some(anchor.item_id.as_str()))?,
                    anchor.selector_type.as_str(),
                    anchor.selector_json,
                    anchor.quoted_text
                ],
            )
            .map_db_err()?;
        }

        for run in &plan.execution_runs {
            tx.execute(
                "insert into execution_run (stable_id, run_id, workspace_id, producer, mission_id, flow_id, mode, status, started_at_ms, updated_at_ms, last_event_type)
                 values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    run.stable_id,
                    run.run_id,
                    run.workspace_id,
                    run.producer,
                    run.mission_id,
                    run.flow_id,
                    run.mode,
                    run.status,
                    run.started_at_ms,
                    run.updated_at_ms,
                    run.last_event_type,
                ],
            )
            .map_db_err()?;
        }

        for task in &plan.execution_tasks {
            tx.execute(
                "insert into execution_task (stable_id, run_id, task_id, workspace_id, producer, title, status, owner_role, updated_at_ms)
                 values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    task.stable_id,
                    task.run_id,
                    task.task_id,
                    task.workspace_id,
                    task.producer,
                    task.title,
                    task.status,
                    task.owner_role,
                    task.updated_at_ms,
                ],
            )
            .map_db_err()?;
        }

        for check in &plan.execution_checks {
            tx.execute(
                "insert into execution_check (stable_id, run_id, task_id, name, status, details, updated_at_ms)
                 values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    check.stable_id,
                    check.run_id,
                    check.task_id,
                    check.name,
                    check.status,
                    check.details,
                    check.updated_at_ms,
                ],
            )
            .map_db_err()?;
        }

        for approval in &plan.execution_approvals {
            tx.execute(
                "insert into execution_approval (stable_id, run_id, task_id, approval_id, kind, status, resume_token, updated_at_ms)
                 values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    approval.stable_id,
                    approval.run_id,
                    approval.task_id,
                    approval.approval_id,
                    approval.kind,
                    approval.status,
                    approval.resume_token,
                    approval.updated_at_ms,
                ],
            )
            .map_db_err()?;
        }

        for event in &plan.execution_events {
            tx.execute(
                "insert into execution_event (stable_id, raw_event_id, run_id, task_id, producer, role, event_type, status, body_text, occurred_at_ms)
                 values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    event.stable_id,
                    event.raw_event_id,
                    event.run_id,
                    event.task_id,
                    event.producer,
                    event.role,
                    event.event_type,
                    event.status,
                    event.body_text,
                    event.occurred_at_ms,
                ],
            )
            .map_db_err()?;
        }

        for document in &plan.document_records {
            tx.execute(
                "insert into document_record (stable_id, document_id, workspace_id, producer, kind, path, title, body_text, artifact_uri, artifact_mime, artifact_sha256, artifact_bytes, updated_at_ms, raw_event_id)
                 values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    document.stable_id,
                    document.document_id,
                    document.workspace_id,
                    document.producer,
                    document.kind,
                    document.path,
                    document.title,
                    document.body_text,
                    document.artifact_uri,
                    document.artifact_mime,
                    document
                        .artifact_sha256_hex
                        .as_deref()
                        .map(hex_to_bytes)
                        .transpose()?,
                    document.artifact_bytes.map(|value| value as i64),
                    document.updated_at_ms,
                    document.raw_event_id,
                ],
            )
            .map_db_err()?;
        }

        Ok(serde_json::json!({
            "workspaces": plan.workspaces.len(),
            "conv_sessions": plan.conv_sessions.len(),
            "conv_turns": plan.conv_turns.len(),
            "conv_items": plan.conv_items.len(),
            "artifacts": plan.artifacts.len(),
            "evidence_anchors": plan.evidence_anchors.len(),
            "execution_runs": plan.execution_runs.len(),
            "execution_tasks": plan.execution_tasks.len(),
            "execution_checks": plan.execution_checks.len(),
            "execution_approvals": plan.execution_approvals.len(),
            "execution_events": plan.execution_events.len(),
            "document_records": plan.document_records.len(),
        }))
    }
}
