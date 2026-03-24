use std::collections::{BTreeMap, HashMap};

use serde_json::Value;

use axiomsync_domain::error::Result;
use axiomsync_domain::{
    ActorRow, AnchorRow, ArtifactRow, EntryRow, IngressReceiptRow, ProjectionPlan,
    RawArtifactInput, SessionRow, empty_object, stable_id,
};

pub fn plan_projection(receipts: &[IngressReceiptRow]) -> Result<ProjectionPlan> {
    let mut ordered = receipts.to_vec();
    ordered.sort_by(|a, b| {
        a.observed_at
            .cmp(&b.observed_at)
            .then_with(|| a.receipt_id.cmp(&b.receipt_id))
    });

    let mut session_map: BTreeMap<String, SessionRow> = BTreeMap::new();
    let mut actor_map: BTreeMap<String, ActorRow> = BTreeMap::new();
    let mut entries = Vec::new();
    let mut artifacts = Vec::new();
    let mut anchors = Vec::new();
    let mut seq_by_session: HashMap<String, i64> = HashMap::new();

    for receipt in ordered {
        let normalized: Value = serde_json::from_str(&receipt.normalized_json)
            .or_else(|_| serde_json::from_str(&receipt.payload_json))
            .unwrap_or(Value::Null);
        let payload = normalized
            .get("payload")
            .cloned()
            .unwrap_or_else(|| normalized.clone());
        let hints = normalized
            .get("hints")
            .cloned()
            .unwrap_or_else(axiomsync_domain::empty_object);
        let session_id = stable_id(
            "session",
            &(
                receipt.session_kind.as_str(),
                receipt.connector.as_str(),
                receipt.external_session_key.as_deref(),
                receipt.workspace_root.as_deref(),
            ),
        );
        let title = payload_string(
            &payload,
            &[
                "page_title",
                "title",
                "session_title",
                "thread_title",
                "summary",
                "name",
            ],
        )
        .or_else(|| pointer_string(&payload, &["/thread/title", "/session/title"]));
        let session = session_map
            .entry(session_id.clone())
            .or_insert_with(|| SessionRow {
                session_id: session_id.clone(),
                session_kind: receipt.session_kind.clone(),
                connector: receipt.connector.clone(),
                external_session_key: receipt.external_session_key.clone(),
                title,
                workspace_root: receipt.workspace_root.clone(),
                opened_at: Some(receipt.observed_at.clone()),
                closed_at: Some(receipt.observed_at.clone()),
                metadata_json: empty_object(),
            });
        if session
            .opened_at
            .as_ref()
            .is_none_or(|opened| receipt.observed_at < *opened)
        {
            session.opened_at = Some(receipt.observed_at.clone());
        }
        if session
            .closed_at
            .as_ref()
            .is_none_or(|closed| receipt.observed_at > *closed)
        {
            session.closed_at = Some(receipt.observed_at.clone());
        }
        if session.title.is_none() {
            session.title = payload_string(&payload, &["page_title", "title", "summary"]);
        }

        let actor_kind = derive_actor_kind(&receipt.event_kind, &payload, &hints);
        let actor_id = stable_id(
            "actor",
            &(
                receipt.connector.as_str(),
                actor_kind.as_str(),
                payload_string(&payload, &["actor_name", "author", "display_name"])
                    .or_else(|| pointer_string(&payload, &["/source_message/message_id"])),
            ),
        );
        actor_map
            .entry(actor_id.clone())
            .or_insert_with(|| ActorRow {
                actor_id: actor_id.clone(),
                actor_kind: actor_kind.clone(),
                stable_key: Some(stable_id(
                    "actor_key",
                    &(receipt.connector.as_str(), actor_kind.as_str()),
                )),
                display_name: payload_string(&payload, &["actor_name", "author", "display_name"]),
                metadata_json: actor_metadata(&payload, &hints),
            });

        let seq_no = seq_by_session
            .entry(session_id.clone())
            .and_modify(|value| *value += 1)
            .or_insert(1);
        let text_body = payload_text(&payload);
        let entry_kind =
            hint_string(&hints, "entry_kind").unwrap_or_else(|| receipt.event_kind.clone());
        let entry_id = stable_id(
            "entry",
            &(
                session_id.as_str(),
                *seq_no,
                receipt.receipt_id.as_str(),
                receipt.external_entry_key.as_deref(),
            ),
        );
        entries.push(EntryRow {
            entry_id: entry_id.clone(),
            session_id: session_id.clone(),
            seq_no: *seq_no,
            entry_kind: entry_kind.clone(),
            actor_id: Some(actor_id.clone()),
            parent_entry_id: None,
            external_entry_key: receipt.external_entry_key.clone(),
            text_body: text_body.clone(),
            started_at: Some(receipt.observed_at.clone()),
            ended_at: receipt
                .captured_at
                .clone()
                .or_else(|| Some(receipt.observed_at.clone())),
            metadata_json: entry_metadata(&payload, &hints),
        });

        if let Some(preview_text) = text_body.clone().filter(|value| !value.trim().is_empty()) {
            let dom_fingerprint = pointer_string(&payload, &["/selection/dom_fingerprint"]);
            anchors.push(AnchorRow {
                anchor_id: stable_id("anchor", &(entry_id.as_str(), "text")),
                entry_id: Some(entry_id.clone()),
                artifact_id: None,
                anchor_kind: if dom_fingerprint.is_some() {
                    "text_selection".to_string()
                } else {
                    "text_span".to_string()
                },
                locator_json: serde_json::json!({
                    "kind": if dom_fingerprint.is_some() { "selection_text" } else { "entry_text" },
                    "page_url": payload_string(&payload, &["page_url"]),
                    "start_hint": pointer_string(&payload, &["/selection/start_hint"]),
                    "end_hint": pointer_string(&payload, &["/selection/end_hint"]),
                    "dom_fingerprint": dom_fingerprint,
                })
                .to_string(),
                fingerprint: pointer_string(&payload, &["/selection/dom_fingerprint"]).or_else(
                    || {
                        Some(stable_id(
                            "fingerprint",
                            &(preview_text.as_str(), receipt.content_hash.as_str()),
                        ))
                    },
                ),
                preview_text: Some(preview_text),
            });
        }

        let artifact_inputs: Vec<RawArtifactInput> =
            serde_json::from_str(&receipt.artifacts_json).unwrap_or_default();
        for artifact_input in artifact_inputs {
            let artifact_id = stable_id(
                "artifact",
                &(
                    session_id.as_str(),
                    entry_id.as_str(),
                    artifact_input.uri.as_str(),
                ),
            );
            artifacts.push(ArtifactRow {
                artifact_id: artifact_id.clone(),
                session_id: session_id.clone(),
                entry_id: Some(entry_id.clone()),
                artifact_kind: artifact_input.artifact_kind.clone(),
                uri: artifact_input.uri.clone(),
                mime_type: artifact_input.mime_type.clone(),
                sha256: artifact_input.sha256.clone(),
                size_bytes: artifact_input.size_bytes,
                metadata_json: artifact_input.metadata_json.clone(),
            });
            anchors.push(AnchorRow {
                anchor_id: stable_id("anchor", &(artifact_id.as_str(), "artifact")),
                entry_id: Some(entry_id.clone()),
                artifact_id: Some(artifact_id),
                anchor_kind: "artifact_range".to_string(),
                locator_json:
                    serde_json::json!({ "kind": "artifact_uri", "uri": artifact_input.uri })
                        .to_string(),
                preview_text: None,
                fingerprint: artifact_input.sha256,
            });
        }
    }

    Ok(ProjectionPlan {
        sessions: session_map.into_values().collect(),
        actors: actor_map.into_values().collect(),
        entries,
        artifacts,
        anchors,
    })
}

fn payload_string(payload: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .filter_map(|key| payload.get(key))
        .find_map(Value::as_str)
        .map(ToOwned::to_owned)
}

fn pointer_string(payload: &Value, pointers: &[&str]) -> Option<String> {
    pointers
        .iter()
        .find_map(|pointer| payload.pointer(pointer))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn payload_text(payload: &Value) -> Option<String> {
    payload_string(
        payload,
        &[
            "text", "body", "content", "summary", "message", "output", "details",
        ],
    )
    .or_else(|| {
        pointer_string(
            payload,
            &[
                "/body/text",
                "/content/text",
                "/result/text",
                "/message/content",
                "/document/body",
                "/selection/text",
            ],
        )
    })
    .or_else(|| {
        payload
            .get("checks")
            .and_then(Value::as_array)
            .map(|checks| {
                checks
                    .iter()
                    .filter_map(|check| {
                        let name = check.get("name").and_then(Value::as_str)?;
                        let status = check
                            .get("status")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown");
                        Some(format!("{name}: {status}"))
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .filter(|value| !value.trim().is_empty())
    })
}

fn derive_actor_kind(event_kind: &str, payload: &Value, hints: &Value) -> String {
    payload_string(payload, &["actor", "role", "author_role"])
        .or_else(|| {
            pointer_string(
                payload,
                &[
                    "/actor/kind",
                    "/actor/role",
                    "/role",
                    "/source_message/role",
                ],
            )
        })
        .or_else(|| hint_string(hints, "role"))
        .unwrap_or_else(|| {
            let lowered = event_kind.to_ascii_lowercase();
            if lowered.contains("assistant") {
                "assistant".to_string()
            } else if lowered.contains("tool") || lowered.contains("command") {
                "tool".to_string()
            } else {
                "user".to_string()
            }
        })
}

fn hint_string(hints: &Value, key: &str) -> Option<String> {
    hints
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
}

fn actor_metadata(payload: &Value, hints: &Value) -> Value {
    serde_json::json!({
        "source_message_id": pointer_string(payload, &["/source_message/message_id"]),
        "source_message_role": pointer_string(payload, &["/source_message/role"]),
        "role_hint": hint_string(hints, "role"),
    })
}

fn entry_metadata(payload: &Value, hints: &Value) -> Value {
    serde_json::json!({
        "page_title": payload_string(payload, &["page_title"]),
        "page_url": payload_string(payload, &["page_url"]),
        "entry_kind_hint": hint_string(hints, "entry_kind"),
        "session_kind_hint": hint_string(hints, "session_kind"),
        "workspace_root_hint": hint_string(hints, "workspace_root"),
        "dom_fingerprint": pointer_string(payload, &["/selection/dom_fingerprint"]),
    })
}
