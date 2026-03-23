use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::domain::{
    ArtifactRow, ConvItemRow, ConvSessionRow, ConvTurnRow, DocumentRecordRow, EvidenceAnchorRow,
    ExecutionApprovalRow, ExecutionCheckRow, ExecutionEventRow, ExecutionRunRow, ExecutionTaskRow,
    ItemType, ProjectionPlan, RawEventRow, SelectorType, WorkspaceRow,
};
use crate::error::{AxiomError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectionKind {
    Thread,
    Execution,
    Document,
    Skip,
}

#[derive(Debug, Clone)]
struct ExecutionMaterialized {
    run: ExecutionRunRow,
    task: Option<ExecutionTaskRow>,
    check: Option<ExecutionCheckRow>,
    approval: Option<ExecutionApprovalRow>,
    event: ExecutionEventRow,
}

#[derive(Debug, Clone, Default)]
struct ArtifactReference {
    uri: String,
    mime: Option<String>,
    sha256_hex: Option<String>,
    bytes: Option<u64>,
}

fn payload_value(payload_json: &str) -> Result<Value> {
    serde_json::from_str(payload_json).map_err(Into::into)
}

pub(crate) fn payload_string(payload: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .filter_map(|key| payload.get(key))
        .find_map(Value::as_str)
        .map(ToOwned::to_owned)
}

fn pointer_value<'a>(payload: &'a Value, pointers: &[&str]) -> Option<&'a Value> {
    pointers.iter().find_map(|pointer| payload.pointer(pointer))
}

fn pointer_string(payload: &Value, pointers: &[&str]) -> Option<String> {
    pointer_value(payload, pointers)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

pub(crate) fn workspace_root_for_payload(payload: &Value) -> String {
    payload_string(payload, &["workspace_root", "root", "cwd"])
        .or_else(|| {
            pointer_string(
                payload,
                &[
                    "/metadata/workspace_root",
                    "/context/workspace_root",
                    "/document/workspace_root",
                ],
            )
        })
        .unwrap_or_else(|| ".".to_string())
}

fn workspace_for_event(event: &RawEventRow) -> Result<WorkspaceRow> {
    let payload = payload_value(&event.payload_json)?;
    let canonical_root = workspace_root_for_payload(&payload);
    Ok(WorkspaceRow {
        stable_id: crate::domain::workspace_stable_id(&canonical_root),
        canonical_root,
        repo_remote: payload_string(&payload, &["repo_remote"])
            .or_else(|| pointer_string(&payload, &["/metadata/repo_remote"])),
        branch: payload_string(&payload, &["branch"])
            .or_else(|| pointer_string(&payload, &["/metadata/branch"])),
        worktree_path: payload_string(&payload, &["worktree_path"])
            .or_else(|| pointer_string(&payload, &["/metadata/worktree_path"])),
    })
}

fn normalized_record_type(payload: &Value, event: &RawEventRow) -> String {
    pointer_string(payload, &["/record_type"])
        .or_else(|| payload_string(payload, &["record_type"]))
        .unwrap_or_else(|| event.event_type.clone())
}

fn subject_kind(payload: &Value) -> Option<String> {
    pointer_string(payload, &["/subject/kind"])
        .or_else(|| payload_string(payload, &["subject_kind"]))
}

fn subject_id(payload: &Value) -> Option<String> {
    pointer_string(payload, &["/subject/id"]).or_else(|| payload_string(payload, &["subject_id"]))
}

fn subject_parent_id(payload: &Value) -> Option<String> {
    pointer_string(payload, &["/subject/parent_id", "/subject/parentId"])
        .or_else(|| payload_string(payload, &["subject_parent_id"]))
}

fn runtime_run_key(payload: &Value) -> Option<String> {
    pointer_string(payload, &["/runtime/run_id", "/runtime/runId"])
        .or_else(|| payload_string(payload, &["run_id"]))
}

fn runtime_task_key(payload: &Value) -> Option<String> {
    pointer_string(payload, &["/runtime/task_id", "/runtime/taskId"])
        .or_else(|| payload_string(payload, &["task_id"]))
}

fn body_text_from_payload(payload: &Value) -> Option<String> {
    payload_string(payload, &["text", "content", "body", "summary"]).or_else(|| {
        pointer_string(
            payload,
            &[
                "/body/text",
                "/body/summary",
                "/body/content",
                "/document/body",
                "/result/text",
                "/check/details",
            ],
        )
    })
}

fn projection_kind(event: &RawEventRow, payload: &Value) -> ProjectionKind {
    let record_type = normalized_record_type(payload, event);
    match subject_kind(payload).as_deref() {
        Some("document") => ProjectionKind::Document,
        Some("run" | "task" | "check" | "approval") => ProjectionKind::Execution,
        Some("thread") => ProjectionKind::Thread,
        Some("artifact" | "case") => ProjectionKind::Skip,
        _ => {
            if record_type == "document_snapshot" {
                ProjectionKind::Document
            } else if runtime_run_key(payload).is_some()
                || matches!(
                    record_type.as_str(),
                    "task_state" | "check_result" | "approval_state"
                )
            {
                ProjectionKind::Execution
            } else {
                ProjectionKind::Thread
            }
        }
    }
}

fn derive_actor(event: &RawEventRow) -> Result<String> {
    let payload = payload_value(&event.payload_json)?;
    Ok(payload_string(&payload, &["actor", "role"])
        .or_else(|| pointer_string(&payload, &["/runtime/role", "/role"]))
        .unwrap_or_else(|| match normalized_record_type(&payload, event).as_str() {
            "assistant_message" | "assistant_msg" => "assistant".to_string(),
            "tool_result" | "command" => "tool".to_string(),
            _ => "user".to_string(),
        }))
}

fn derive_item_type(event: &RawEventRow) -> Result<ItemType> {
    let payload = payload_value(&event.payload_json)?;
    let actor = derive_actor(event)?;
    Ok(match normalized_record_type(&payload, event).as_str() {
        "message" => match actor.as_str() {
            "assistant" => ItemType::AssistantMsg,
            "tool" => ItemType::ToolResult,
            _ => ItemType::UserMsg,
        },
        "assistant_message" | "assistant_msg" => ItemType::AssistantMsg,
        "tool_call" => ItemType::ToolCall,
        "tool_result" | "command" => ItemType::ToolResult,
        "file_change" => ItemType::FileChange,
        "diff" => ItemType::Diff,
        "plan" => ItemType::Plan,
        _ => ItemType::UserMsg,
    })
}

fn derive_body_text(event: &RawEventRow) -> Result<Option<String>> {
    let payload = payload_value(&event.payload_json)?;
    Ok(body_text_from_payload(&payload))
}

fn turn_sort_key(items: &[RawEventRow]) -> i64 {
    items
        .iter()
        .map(|event| event.ts_ms)
        .min()
        .unwrap_or_default()
}

fn thread_session_key(payload: &Value, event: &RawEventRow) -> String {
    if subject_kind(payload).as_deref() == Some("thread") {
        subject_id(payload).unwrap_or_else(|| event.native_session_id.clone())
    } else {
        event.native_session_id.clone()
    }
}

fn thread_turn_key(payload: &Value, event: &RawEventRow) -> Result<String> {
    if let Some(turn_id) = payload_string(payload, &["turn_id", "native_turn_id"]) {
        return Ok(turn_id);
    }
    if let Some(turn_id) = pointer_string(payload, &["/entry/id", "/entry/turn_id", "/step/id"]) {
        return Ok(turn_id);
    }
    Ok(crate::domain::stable_id(
        "turnkey",
        &(event.ts_ms / 1_000, derive_actor(event)?),
    ))
}

fn artifact_values(artifact: &Value) -> ArtifactReference {
    ArtifactReference {
        uri: payload_string(artifact, &["uri", "path"]).unwrap_or_default(),
        mime: payload_string(artifact, &["mime"]),
        sha256_hex: payload_string(artifact, &["sha256"]),
        bytes: artifact
            .get("bytes")
            .and_then(Value::as_u64)
            .or_else(|| artifact.get("size").and_then(Value::as_u64)),
    }
}

fn extract_artifact_rows(item_id: &str, payload: &Value) -> Vec<ArtifactRow> {
    payload
        .get("artifacts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|artifact| {
            let artifact = artifact_values(artifact);
            ArtifactRow {
                stable_id: crate::domain::stable_id("artifact", &(item_id, artifact.uri.as_str())),
                item_id: item_id.to_string(),
                uri: artifact.uri,
                mime: artifact.mime,
                sha256_hex: artifact.sha256_hex,
                bytes: artifact.bytes,
            }
        })
        .collect()
}

fn first_artifact(payload: &Value) -> Option<ArtifactReference> {
    payload
        .get("artifacts")
        .and_then(Value::as_array)
        .and_then(|artifacts| artifacts.first())
        .map(artifact_values)
}

fn execution_run_external_id(payload: &Value, event: &RawEventRow) -> Option<String> {
    match subject_kind(payload).as_deref() {
        Some("run") => subject_id(payload),
        Some("task" | "check" | "approval") => {
            runtime_run_key(payload).or_else(|| subject_parent_id(payload))
        }
        _ => runtime_run_key(payload),
    }
    .or_else(|| {
        if event.native_schema_version.as_deref() == Some("agent-record-v1") {
            Some(event.native_session_id.clone())
        } else {
            None
        }
    })
}

fn execution_task_external_id(payload: &Value) -> Option<String> {
    match subject_kind(payload).as_deref() {
        Some("task") => subject_id(payload),
        Some("check" | "approval") => {
            runtime_task_key(payload).or_else(|| subject_parent_id(payload))
        }
        _ => runtime_task_key(payload),
    }
}

fn execution_role(payload: &Value, event: &RawEventRow) -> Option<String> {
    pointer_string(payload, &["/runtime/role", "/role"])
        .or_else(|| payload_string(payload, &["role"]))
        .or_else(|| derive_actor(event).ok())
}

fn execution_status(payload: &Value) -> Option<String> {
    pointer_string(
        payload,
        &[
            "/runtime/status",
            "/approval/status",
            "/check/status",
            "/status",
        ],
    )
    .or_else(|| payload_string(payload, &["status"]))
}

fn document_external_id(payload: &Value, producer: &str) -> String {
    if let Some(id) = subject_id(payload) {
        return id;
    }
    if let Some(path) =
        pointer_string(payload, &["/document/path"]).or_else(|| payload_string(payload, &["path"]))
    {
        return crate::domain::stable_id("doc_ref", &(producer, path.as_str()));
    }
    crate::domain::stable_id("doc_ref", &payload)
}

fn document_kind(payload: &Value) -> String {
    pointer_string(payload, &["/document/kind", "/document/type"])
        .or_else(|| payload_string(payload, &["kind", "document_kind"]))
        .unwrap_or_else(|| "document".to_string())
}

fn choose_later(
    existing_ms: i64,
    existing_id: &str,
    candidate_ms: i64,
    candidate_id: &str,
) -> bool {
    candidate_ms > existing_ms || (candidate_ms == existing_ms && candidate_id > existing_id)
}

fn materialize_execution_event(
    event: &RawEventRow,
    payload: &Value,
) -> Option<ExecutionMaterialized> {
    let run_external_id = execution_run_external_id(payload, event)?;
    let run_stable_id =
        crate::domain::stable_id("run", &(event.connector.as_str(), run_external_id.as_str()));
    let workspace_id = workspace_for_event(event)
        .ok()
        .map(|workspace| workspace.stable_id);
    let task_external_id = execution_task_external_id(payload);
    let task_stable_id = task_external_id.as_ref().map(|task_id| {
        crate::domain::stable_id("task", &(run_stable_id.as_str(), task_id.as_str()))
    });
    let role = execution_role(payload, event);
    let record_type = normalized_record_type(payload, event);
    let status = execution_status(payload);
    let body_text = body_text_from_payload(payload);

    let run = ExecutionRunRow {
        stable_id: run_stable_id.clone(),
        run_id: run_external_id,
        workspace_id: workspace_id.clone(),
        producer: event.connector.clone(),
        mission_id: pointer_string(payload, &["/program/mission_id", "/program/name"]),
        flow_id: pointer_string(payload, &["/program/flow_id", "/program/flow"]),
        mode: pointer_string(payload, &["/program/mode", "/runtime/mode"]),
        status: status.clone().unwrap_or_else(|| "working".to_string()),
        started_at_ms: event.ts_ms,
        updated_at_ms: event.ts_ms,
        last_event_type: record_type.clone(),
    };

    let task = task_external_id.as_ref().map(|task_id| ExecutionTaskRow {
        stable_id: task_stable_id.clone().expect("task stable id"),
        run_id: run_stable_id.clone(),
        task_id: task_id.clone(),
        workspace_id: workspace_id.clone(),
        producer: event.connector.clone(),
        title: pointer_string(payload, &["/task/title", "/title"]).or_else(|| body_text.clone()),
        status: status.clone().unwrap_or_else(|| "working".to_string()),
        owner_role: role.clone(),
        updated_at_ms: event.ts_ms,
    });

    let check = if record_type == "check_result"
        || subject_kind(payload).as_deref() == Some("check")
        || pointer_value(payload, &["/check"]).is_some()
    {
        let name = pointer_string(payload, &["/check/name"]).or_else(|| {
            body_text
                .clone()
                .filter(|value| !value.trim().is_empty())
                .map(|value| value.lines().next().unwrap_or_default().to_string())
        })?;
        Some(ExecutionCheckRow {
            stable_id: crate::domain::stable_id(
                "check",
                &(
                    run_stable_id.as_str(),
                    task_stable_id.as_deref().unwrap_or(""),
                    name.as_str(),
                ),
            ),
            run_id: run_stable_id.clone(),
            task_id: task_stable_id.clone(),
            name,
            status: pointer_string(payload, &["/check/status"])
                .or_else(|| status.clone())
                .unwrap_or_else(|| "unknown".to_string()),
            details: pointer_string(payload, &["/check/details"]).or_else(|| body_text.clone()),
            updated_at_ms: event.ts_ms,
        })
    } else {
        None
    };

    let approval = if record_type == "approval_state"
        || subject_kind(payload).as_deref() == Some("approval")
        || pointer_value(payload, &["/approval"]).is_some()
    {
        let approval_id = pointer_string(payload, &["/approval/approval_id", "/approval/id"])
            .or_else(|| payload_string(payload, &["approval_id"]))?;
        Some(ExecutionApprovalRow {
            stable_id: crate::domain::stable_id(
                "approval",
                &(run_stable_id.as_str(), approval_id.as_str()),
            ),
            run_id: run_stable_id.clone(),
            task_id: task_stable_id.clone(),
            approval_id,
            kind: pointer_string(payload, &["/approval/kind"]),
            status: pointer_string(payload, &["/approval/status"])
                .or_else(|| status.clone())
                .unwrap_or_else(|| "required".to_string()),
            resume_token: pointer_string(
                payload,
                &["/approval/resume_token", "/approval/resumeToken"],
            ),
            updated_at_ms: event.ts_ms,
        })
    } else {
        None
    };

    let event_row = ExecutionEventRow {
        stable_id: crate::domain::stable_id("exec_event", &event.stable_id),
        raw_event_id: event.stable_id.clone(),
        run_id: run_stable_id,
        task_id: task_stable_id,
        producer: event.connector.clone(),
        role,
        event_type: record_type,
        status,
        body_text,
        occurred_at_ms: event.ts_ms,
    };

    Some(ExecutionMaterialized {
        run,
        task,
        check,
        approval,
        event: event_row,
    })
}

fn materialize_document_record(event: &RawEventRow, payload: &Value) -> Option<DocumentRecordRow> {
    let workspace_id = workspace_for_event(event)
        .ok()
        .map(|workspace| workspace.stable_id);
    let external_id = document_external_id(payload, &event.connector);
    let stable_id = crate::domain::stable_id(
        "document",
        &(event.connector.as_str(), external_id.as_str()),
    );
    let artifact = first_artifact(payload).unwrap_or_default();

    Some(DocumentRecordRow {
        stable_id,
        document_id: external_id,
        workspace_id,
        producer: event.connector.clone(),
        kind: document_kind(payload),
        path: pointer_string(payload, &["/document/path"])
            .or_else(|| payload_string(payload, &["path"])),
        title: pointer_string(payload, &["/document/title"])
            .or_else(|| payload_string(payload, &["title"])),
        body_text: pointer_string(payload, &["/document/body"])
            .or_else(|| body_text_from_payload(payload)),
        artifact_uri: if artifact.uri.is_empty() {
            None
        } else {
            Some(artifact.uri)
        },
        artifact_mime: artifact.mime,
        artifact_sha256_hex: artifact.sha256_hex,
        artifact_bytes: artifact.bytes,
        updated_at_ms: event.ts_ms,
        raw_event_id: event.stable_id.clone(),
    })
}

fn build_thread_projection(raw_events: &[RawEventRow]) -> Result<ProjectionPlan> {
    let mut session_events = BTreeMap::<(String, String), Vec<RawEventRow>>::new();
    for event in raw_events {
        let payload = payload_value(&event.payload_json)?;
        let session_key = thread_session_key(&payload, event);
        session_events
            .entry((event.connector.clone(), session_key))
            .or_default()
            .push(event.clone());
    }

    let mut conv_sessions = Vec::new();
    let mut conv_turns = Vec::new();
    let mut conv_items = Vec::new();
    let mut artifacts = Vec::new();
    let mut evidence_anchors = Vec::new();

    for ((producer, session_key), mut events) in session_events {
        let workspace = workspace_for_event(events.first().ok_or_else(|| {
            AxiomError::Validation("session must contain at least one raw event".to_string())
        })?)?;
        events.sort_by(|left, right| {
            left.ts_ms
                .cmp(&right.ts_ms)
                .then(left.stable_id.cmp(&right.stable_id))
        });
        let session_payload = payload_value(&events[0].payload_json)?;
        let last_payload = payload_value(
            &events
                .last()
                .ok_or_else(|| {
                    AxiomError::Validation("session must contain a last raw event".to_string())
                })?
                .payload_json,
        )?;
        let session_record = ConvSessionRow {
            stable_id: crate::domain::stable_id(
                "session",
                &(producer.as_str(), session_key.as_str()),
            ),
            connector: producer.clone(),
            native_session_id: session_key.clone(),
            workspace_id: Some(workspace.stable_id.clone()),
            title: payload_string(&session_payload, &["title", "session_title"])
                .or_else(|| pointer_string(&session_payload, &["/thread/title"])),
            transcript_uri: payload_string(&session_payload, &["transcript_uri"])
                .or_else(|| pointer_string(&session_payload, &["/thread/transcript_uri"])),
            status: payload_string(&last_payload, &["status"])
                .or_else(|| pointer_string(&last_payload, &["/runtime/status", "/status"]))
                .unwrap_or_else(|| "active".to_string()),
            started_at_ms: events.first().map(|event| event.ts_ms),
            ended_at_ms: events.last().map(|event| event.ts_ms),
        };
        let session_id = session_record.stable_id.clone();
        conv_sessions.push(session_record);

        let mut turns = BTreeMap::<String, Vec<RawEventRow>>::new();
        for event in events {
            let payload = payload_value(&event.payload_json)?;
            turns
                .entry(thread_turn_key(&payload, &event)?)
                .or_default()
                .push(event);
        }

        let mut ordered_turns = turns.into_iter().collect::<Vec<_>>();
        ordered_turns.sort_by(|left, right| {
            turn_sort_key(&left.1)
                .cmp(&turn_sort_key(&right.1))
                .then(left.0.cmp(&right.0))
        });

        for (turn_index, (turn_key, turn_events)) in ordered_turns.into_iter().enumerate() {
            let actor = derive_actor(turn_events.first().ok_or_else(|| {
                AxiomError::Validation("turn must contain at least one raw event".to_string())
            })?)?;
            let turn_record = ConvTurnRow {
                stable_id: crate::domain::stable_id(
                    "turn",
                    &(session_id.as_str(), turn_key.as_str(), turn_index),
                ),
                session_id: session_id.clone(),
                native_turn_id: Some(turn_key),
                turn_index,
                actor,
            };
            let turn_id = turn_record.stable_id.clone();
            conv_turns.push(turn_record);

            for event in turn_events {
                let payload = payload_value(&event.payload_json)?;
                let item_record = ConvItemRow {
                    stable_id: crate::domain::stable_id(
                        "item",
                        &(turn_id.as_str(), event.stable_id.as_str()),
                    ),
                    turn_id: turn_id.clone(),
                    item_type: derive_item_type(&event)?,
                    tool_name: payload_string(&payload, &["tool_name", "tool"])
                        .or_else(|| pointer_string(&payload, &["/tool/name"])),
                    body_text: derive_body_text(&event)?,
                    payload_json: event.payload_json.clone(),
                };
                let item_id = item_record.stable_id.clone();
                if let Some(text) = item_record.body_text.as_ref() {
                    evidence_anchors.push(EvidenceAnchorRow {
                        stable_id: crate::domain::stable_id("anchor", &(item_id.as_str(), "body")),
                        item_id: item_id.clone(),
                        selector_type: SelectorType::TextSpan,
                        selector_json: json!({"start": 0, "end": text.chars().count()}).to_string(),
                        quoted_text: Some(text.chars().take(200).collect()),
                    });
                } else {
                    evidence_anchors.push(EvidenceAnchorRow {
                        stable_id: crate::domain::stable_id(
                            "anchor",
                            &(item_id.as_str(), "payload"),
                        ),
                        item_id: item_id.clone(),
                        selector_type: SelectorType::JsonPointer,
                        selector_json: Value::String("/".to_string()).to_string(),
                        quoted_text: None,
                    });
                }
                artifacts.extend(extract_artifact_rows(&item_id, &payload));
                conv_items.push(item_record);
            }
        }
    }

    conv_sessions.sort_by(|left, right| left.stable_id.cmp(&right.stable_id));
    conv_turns.sort_by(|left, right| {
        left.session_id
            .cmp(&right.session_id)
            .then(left.turn_index.cmp(&right.turn_index))
    });
    conv_items.sort_by(|left, right| {
        left.turn_id
            .cmp(&right.turn_id)
            .then(left.stable_id.cmp(&right.stable_id))
    });
    artifacts.sort_by(|left, right| {
        left.item_id
            .cmp(&right.item_id)
            .then(left.stable_id.cmp(&right.stable_id))
    });
    evidence_anchors.sort_by(|left, right| {
        left.item_id
            .cmp(&right.item_id)
            .then(left.stable_id.cmp(&right.stable_id))
    });

    Ok(ProjectionPlan {
        workspaces: Vec::new(),
        conv_sessions,
        conv_turns,
        conv_items,
        artifacts,
        evidence_anchors,
        execution_runs: Vec::new(),
        execution_tasks: Vec::new(),
        execution_checks: Vec::new(),
        execution_approvals: Vec::new(),
        execution_events: Vec::new(),
        document_records: Vec::new(),
    })
}

fn build_execution_projection(raw_events: &[RawEventRow]) -> Result<ProjectionPlan> {
    let mut ordered = raw_events.to_vec();
    ordered.sort_by(|left, right| {
        left.ts_ms
            .cmp(&right.ts_ms)
            .then(left.stable_id.cmp(&right.stable_id))
    });

    let mut runs = BTreeMap::<String, ExecutionRunRow>::new();
    let mut tasks = BTreeMap::<String, ExecutionTaskRow>::new();
    let mut checks = BTreeMap::<String, ExecutionCheckRow>::new();
    let mut approvals = BTreeMap::<String, ExecutionApprovalRow>::new();
    let mut events = Vec::<ExecutionEventRow>::new();

    for event in ordered {
        let payload = payload_value(&event.payload_json)?;
        let Some(materialized) = materialize_execution_event(&event, &payload) else {
            continue;
        };

        runs.entry(materialized.run.stable_id.clone())
            .and_modify(|existing| {
                existing.started_at_ms = existing.started_at_ms.min(materialized.run.started_at_ms);
                if choose_later(
                    existing.updated_at_ms,
                    existing.stable_id.as_str(),
                    materialized.run.updated_at_ms,
                    materialized.run.stable_id.as_str(),
                ) {
                    *existing = materialized.run.clone();
                } else {
                    if existing.mission_id.is_none() {
                        existing.mission_id = materialized.run.mission_id.clone();
                    }
                    if existing.flow_id.is_none() {
                        existing.flow_id = materialized.run.flow_id.clone();
                    }
                    if existing.mode.is_none() {
                        existing.mode = materialized.run.mode.clone();
                    }
                }
            })
            .or_insert(materialized.run.clone());

        if let Some(task) = materialized.task.clone() {
            tasks
                .entry(task.stable_id.clone())
                .and_modify(|existing| {
                    if choose_later(
                        existing.updated_at_ms,
                        existing.stable_id.as_str(),
                        task.updated_at_ms,
                        task.stable_id.as_str(),
                    ) {
                        *existing = task.clone();
                    } else {
                        if existing.title.is_none() {
                            existing.title = task.title.clone();
                        }
                        if existing.owner_role.is_none() {
                            existing.owner_role = task.owner_role.clone();
                        }
                    }
                })
                .or_insert(task);
        }

        if let Some(check) = materialized.check.clone() {
            checks
                .entry(check.stable_id.clone())
                .and_modify(|existing| {
                    if choose_later(
                        existing.updated_at_ms,
                        existing.stable_id.as_str(),
                        check.updated_at_ms,
                        check.stable_id.as_str(),
                    ) {
                        *existing = check.clone();
                    } else if existing.details.is_none() {
                        existing.details = check.details.clone();
                    }
                })
                .or_insert(check);
        }

        if let Some(approval) = materialized.approval.clone() {
            approvals
                .entry(approval.stable_id.clone())
                .and_modify(|existing| {
                    if choose_later(
                        existing.updated_at_ms,
                        existing.stable_id.as_str(),
                        approval.updated_at_ms,
                        approval.stable_id.as_str(),
                    ) {
                        *existing = approval.clone();
                    } else if existing.resume_token.is_none() {
                        existing.resume_token = approval.resume_token.clone();
                    }
                })
                .or_insert(approval);
        }

        events.push(materialized.event);
    }

    Ok(ProjectionPlan {
        workspaces: Vec::new(),
        conv_sessions: Vec::new(),
        conv_turns: Vec::new(),
        conv_items: Vec::new(),
        artifacts: Vec::new(),
        evidence_anchors: Vec::new(),
        execution_runs: runs.into_values().collect(),
        execution_tasks: tasks.into_values().collect(),
        execution_checks: checks.into_values().collect(),
        execution_approvals: approvals.into_values().collect(),
        execution_events: events,
        document_records: Vec::new(),
    })
}

fn build_document_projection(raw_events: &[RawEventRow]) -> Result<ProjectionPlan> {
    let mut ordered = raw_events.to_vec();
    ordered.sort_by(|left, right| {
        left.ts_ms
            .cmp(&right.ts_ms)
            .then(left.stable_id.cmp(&right.stable_id))
    });

    let mut documents = BTreeMap::<String, DocumentRecordRow>::new();
    for event in ordered {
        let payload = payload_value(&event.payload_json)?;
        let Some(document) = materialize_document_record(&event, &payload) else {
            continue;
        };
        documents
            .entry(document.stable_id.clone())
            .and_modify(|existing| {
                if choose_later(
                    existing.updated_at_ms,
                    existing.stable_id.as_str(),
                    document.updated_at_ms,
                    document.stable_id.as_str(),
                ) {
                    *existing = document.clone();
                }
            })
            .or_insert(document);
    }

    Ok(ProjectionPlan {
        workspaces: Vec::new(),
        conv_sessions: Vec::new(),
        conv_turns: Vec::new(),
        conv_items: Vec::new(),
        artifacts: Vec::new(),
        evidence_anchors: Vec::new(),
        execution_runs: Vec::new(),
        execution_tasks: Vec::new(),
        execution_checks: Vec::new(),
        execution_approvals: Vec::new(),
        execution_events: Vec::new(),
        document_records: documents.into_values().collect(),
    })
}

pub fn plan_projection(raw_events: &[RawEventRow]) -> Result<ProjectionPlan> {
    let mut workspaces_by_id = BTreeMap::<String, WorkspaceRow>::new();
    let mut thread_events = Vec::new();
    let mut execution_events = Vec::new();
    let mut document_events = Vec::new();

    for event in raw_events {
        event.validate()?;
        let workspace = workspace_for_event(event)?;
        workspaces_by_id.insert(workspace.stable_id.clone(), workspace);
        let payload = payload_value(&event.payload_json)?;
        match projection_kind(event, &payload) {
            ProjectionKind::Thread => thread_events.push(event.clone()),
            ProjectionKind::Execution => execution_events.push(event.clone()),
            ProjectionKind::Document => document_events.push(event.clone()),
            ProjectionKind::Skip => {}
        }
    }

    let mut thread_projection = build_thread_projection(&thread_events)?;
    let execution_projection = build_execution_projection(&execution_events)?;
    let document_projection = build_document_projection(&document_events)?;

    thread_projection.workspaces = workspaces_by_id.into_values().collect();
    thread_projection
        .execution_runs
        .extend(execution_projection.execution_runs);
    thread_projection
        .execution_tasks
        .extend(execution_projection.execution_tasks);
    thread_projection
        .execution_checks
        .extend(execution_projection.execution_checks);
    thread_projection
        .execution_approvals
        .extend(execution_projection.execution_approvals);
    thread_projection
        .execution_events
        .extend(execution_projection.execution_events);
    thread_projection
        .document_records
        .extend(document_projection.document_records);

    thread_projection
        .execution_runs
        .sort_by(|left, right| left.stable_id.cmp(&right.stable_id));
    thread_projection.execution_tasks.sort_by(|left, right| {
        left.run_id
            .cmp(&right.run_id)
            .then(left.stable_id.cmp(&right.stable_id))
    });
    thread_projection.execution_checks.sort_by(|left, right| {
        left.run_id
            .cmp(&right.run_id)
            .then(left.stable_id.cmp(&right.stable_id))
    });
    thread_projection
        .execution_approvals
        .sort_by(|left, right| {
            left.run_id
                .cmp(&right.run_id)
                .then(left.stable_id.cmp(&right.stable_id))
        });
    thread_projection.execution_events.sort_by(|left, right| {
        left.occurred_at_ms
            .cmp(&right.occurred_at_ms)
            .then(left.stable_id.cmp(&right.stable_id))
    });
    thread_projection.document_records.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then(left.stable_id.cmp(&right.stable_id))
    });

    thread_projection.validate()?;
    Ok(thread_projection)
}
