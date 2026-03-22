use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::domain::{
    ArtifactRow, ConvItemRow, ConvSessionRow, ConvTurnRow, EvidenceAnchorRow, ItemType,
    ProjectionPlan, RawEventRow, SelectorType, WorkspaceRow,
};
use crate::error::{AxiomError, Result};

fn payload_value(payload_json: &str) -> Result<Value> {
    serde_json::from_str(payload_json).map_err(Into::into)
}

pub(crate) fn payload_string(payload: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .filter_map(|key| payload.get(key))
        .find_map(Value::as_str)
        .map(ToOwned::to_owned)
}

fn workspace_for_event(event: &RawEventRow) -> Result<WorkspaceRow> {
    let payload = payload_value(&event.payload_json)?;
    let canonical_root = payload_string(&payload, &["workspace_root", "root", "cwd"])
        .unwrap_or_else(|| ".".to_string());
    Ok(WorkspaceRow {
        stable_id: crate::domain::workspace_stable_id(&canonical_root),
        canonical_root,
        repo_remote: payload_string(&payload, &["repo_remote"]),
        branch: payload_string(&payload, &["branch"]),
        worktree_path: payload_string(&payload, &["worktree_path"]),
    })
}

fn derive_actor(event: &RawEventRow) -> Result<String> {
    let payload = payload_value(&event.payload_json)?;
    Ok(
        payload_string(&payload, &["actor", "role"]).unwrap_or_else(|| {
            match event.event_type.as_str() {
                "assistant_message" => "assistant".to_string(),
                "tool_result" => "tool".to_string(),
                _ => "user".to_string(),
            }
        }),
    )
}

fn derive_item_type(event: &RawEventRow) -> Result<ItemType> {
    let payload = payload_value(&event.payload_json)?;
    Ok(
        match payload_string(&payload, &["item_type"])
            .as_deref()
            .unwrap_or(event.event_type.as_str())
        {
            "assistant_message" | "assistant_msg" => ItemType::AssistantMsg,
            "tool_call" => ItemType::ToolCall,
            "tool_result" => ItemType::ToolResult,
            "file_change" => ItemType::FileChange,
            "diff" => ItemType::Diff,
            "plan" => ItemType::Plan,
            _ => ItemType::UserMsg,
        },
    )
}

fn derive_body_text(event: &RawEventRow) -> Result<Option<String>> {
    let payload = payload_value(&event.payload_json)?;
    Ok(payload_string(
        &payload,
        &["text", "content", "body", "summary"],
    ))
}

fn turn_sort_key(items: &[RawEventRow]) -> i64 {
    items
        .iter()
        .map(|event| event.ts_ms)
        .min()
        .unwrap_or_default()
}

pub fn plan_projection(raw_events: &[RawEventRow]) -> Result<ProjectionPlan> {
    let mut workspaces_by_id = BTreeMap::<String, WorkspaceRow>::new();
    let mut session_events = BTreeMap::<(String, String), Vec<RawEventRow>>::new();
    for event in raw_events {
        event.validate()?;
        let workspace = workspace_for_event(event)?;
        workspaces_by_id.insert(workspace.stable_id.clone(), workspace);
        session_events
            .entry((event.connector.clone(), event.native_session_id.clone()))
            .or_default()
            .push(event.clone());
    }

    let mut conv_sessions = Vec::new();
    let mut conv_turns = Vec::new();
    let mut conv_items = Vec::new();
    let mut artifacts = Vec::new();
    let mut evidence_anchors = Vec::new();

    for ((connector, native_session_id), mut events) in session_events {
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
                &(connector.as_str(), native_session_id.as_str()),
            ),
            connector: connector.clone(),
            native_session_id: native_session_id.clone(),
            workspace_id: Some(workspace.stable_id.clone()),
            title: payload_string(&session_payload, &["title", "session_title"]),
            transcript_uri: payload_string(&session_payload, &["transcript_uri"]),
            status: payload_string(&last_payload, &["status"])
                .unwrap_or_else(|| "active".to_string()),
            started_at_ms: events.first().map(|event| event.ts_ms),
            ended_at_ms: events.last().map(|event| event.ts_ms),
        };
        let session_id = session_record.stable_id.clone();
        conv_sessions.push(session_record);

        let mut turns = BTreeMap::<String, Vec<RawEventRow>>::new();
        for event in events {
            let payload = payload_value(&event.payload_json)?;
            let turn_key = if let Some(turn_id) =
                payload_string(&payload, &["turn_id", "native_turn_id"])
            {
                turn_id
            } else {
                crate::domain::stable_id("turnkey", &(event.ts_ms / 1_000, derive_actor(&event)?))
            };
            turns.entry(turn_key).or_default().push(event);
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
                let item_record = ConvItemRow {
                    stable_id: crate::domain::stable_id(
                        "item",
                        &(turn_id.as_str(), event.stable_id.as_str()),
                    ),
                    turn_id: turn_id.clone(),
                    item_type: derive_item_type(&event)?,
                    tool_name: {
                        let payload = payload_value(&event.payload_json)?;
                        payload_string(&payload, &["tool_name", "tool"])
                    },
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

                let payload = payload_value(&item_record.payload_json)?;
                if let Some(items) = payload.get("artifacts").and_then(Value::as_array) {
                    for artifact in items {
                        let uri = payload_string(artifact, &["uri", "path"]).unwrap_or_default();
                        artifacts.push(ArtifactRow {
                            stable_id: crate::domain::stable_id(
                                "artifact",
                                &(item_id.as_str(), uri.as_str()),
                            ),
                            item_id: item_id.clone(),
                            uri,
                            mime: payload_string(artifact, &["mime"]),
                            sha256_hex: payload_string(artifact, &["sha256"]),
                            bytes: artifact
                                .get("bytes")
                                .and_then(Value::as_u64)
                                .or_else(|| artifact.get("size").and_then(Value::as_u64)),
                        });
                    }
                }

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

    let plan = ProjectionPlan {
        workspaces: workspaces_by_id.into_values().collect(),
        conv_sessions,
        conv_turns,
        conv_items,
        artifacts,
        evidence_anchors,
    };
    plan.validate()?;
    Ok(plan)
}
