use std::collections::HashMap;

use axiomsync_domain::domain::{
    AnchorRow, ClaimEvidenceRow, ClaimRow, DerivePlan, EntryRow, EpisodeRow, ProcedureEvidenceRow,
    ProcedureRow, stable_id,
};
use axiomsync_domain::error::Result;

pub fn plan_derivation(
    sessions: &[axiomsync_domain::domain::SessionRow],
    entries: &[EntryRow],
    anchors: &[AnchorRow],
) -> Result<DerivePlan> {
    let mut episodes = Vec::new();
    let mut claims = Vec::new();
    let mut claim_evidence = Vec::new();
    let mut procedures = Vec::new();
    let mut procedure_evidence = Vec::new();

    let mut anchors_by_entry: HashMap<&str, Vec<&AnchorRow>> = HashMap::new();
    for anchor in anchors {
        if let Some(entry_id) = anchor.entry_id.as_deref() {
            anchors_by_entry.entry(entry_id).or_default().push(anchor);
        }
    }

    for session in sessions {
        let session_entries = entries
            .iter()
            .filter(|entry| entry.session_id == session.session_id)
            .collect::<Vec<_>>();
        if session_entries.is_empty() {
            continue;
        }
        let summary = session
            .title
            .clone()
            .or_else(|| {
                session_entries
                    .iter()
                    .find_map(|entry| entry.text_body.as_ref())
                    .map(|value| truncate(value, 120))
            })
            .unwrap_or_else(|| format!("{} session", session.session_kind));
        let episode_id = stable_id("episode", &(session.session_id.as_str(), summary.as_str()));
        episodes.push(EpisodeRow {
            episode_id: episode_id.clone(),
            session_id: Some(session.session_id.clone()),
            episode_kind: infer_episode_kind(session),
            summary: summary.clone(),
            status: Some("open".to_string()),
            confidence: 0.65,
            extractor_version: "heuristic-v1".to_string(),
            stale: false,
        });

        let mut best_anchor = None;
        for entry in &session_entries {
            if let Some(found) = anchors_by_entry
                .get(entry.entry_id.as_str())
                .and_then(|rows| rows.first())
                .copied()
            {
                best_anchor = Some(found.anchor_id.clone());
                break;
            }
        }

        let summary_claim = ClaimRow {
            claim_id: stable_id("claim", &(episode_id.as_str(), "summary")),
            episode_id: Some(episode_id.clone()),
            claim_kind: "summary".to_string(),
            statement: summary.clone(),
            confidence: 0.7,
            metadata_json: serde_json::json!({ "session_kind": session.session_kind }),
        };
        if let Some(anchor_id) = best_anchor.clone() {
            claim_evidence.push(ClaimEvidenceRow {
                claim_id: summary_claim.claim_id.clone(),
                anchor_id,
                support_kind: "primary".to_string(),
            });
        }
        claims.push(summary_claim);

        if let Some(root_cause) = extract_claim(&session_entries, &["root cause", "because"]) {
            let claim = ClaimRow {
                claim_id: stable_id("claim", &(episode_id.as_str(), "root_cause", root_cause.as_str())),
                episode_id: Some(episode_id.clone()),
                claim_kind: "root_cause".to_string(),
                statement: root_cause.clone(),
                confidence: 0.58,
                metadata_json: serde_json::json!({}),
            };
            if let Some(anchor_id) = best_anchor.clone() {
                claim_evidence.push(ClaimEvidenceRow {
                    claim_id: claim.claim_id.clone(),
                    anchor_id,
                    support_kind: "supporting".to_string(),
                });
            }
            claims.push(claim);
        }

        if let Some(fix) = extract_claim(&session_entries, &["fix", "resolved", "solution"]) {
            let claim = ClaimRow {
                claim_id: stable_id("claim", &(episode_id.as_str(), "fix", fix.as_str())),
                episode_id: Some(episode_id.clone()),
                claim_kind: "fix".to_string(),
                statement: fix.clone(),
                confidence: 0.58,
                metadata_json: serde_json::json!({}),
            };
            if let Some(anchor_id) = best_anchor.clone() {
                claim_evidence.push(ClaimEvidenceRow {
                    claim_id: claim.claim_id.clone(),
                    anchor_id,
                    support_kind: "supporting".to_string(),
                });
            }
            claims.push(claim);
        }

        if let Some(decision) = extract_claim(&session_entries, &["decision", "we chose", "choose"]) {
            let claim = ClaimRow {
                claim_id: stable_id("claim", &(episode_id.as_str(), "decision", decision.as_str())),
                episode_id: Some(episode_id.clone()),
                claim_kind: "decision".to_string(),
                statement: decision.clone(),
                confidence: 0.55,
                metadata_json: serde_json::json!({}),
            };
            if let Some(anchor_id) = best_anchor.clone() {
                claim_evidence.push(ClaimEvidenceRow {
                    claim_id: claim.claim_id.clone(),
                    anchor_id,
                    support_kind: "supporting".to_string(),
                });
            }
            claims.push(claim);
        }

        let commands = extract_procedure_steps(&session_entries);
        if !commands.is_empty() {
            let procedure = ProcedureRow {
                procedure_id: stable_id("procedure", &(episode_id.as_str(), commands.join("\n"))),
                title: session
                    .title
                    .clone()
                    .unwrap_or_else(|| format!("{} procedure", session.session_kind)),
                goal: Some(summary),
                steps_json: serde_json::json!(commands),
                confidence: 0.62,
                extractor_version: "heuristic-v1".to_string(),
                stale: false,
            };
            if let Some(anchor_id) = best_anchor {
                procedure_evidence.push(ProcedureEvidenceRow {
                    procedure_id: procedure.procedure_id.clone(),
                    anchor_id,
                    support_kind: "primary".to_string(),
                });
            }
            procedures.push(procedure);
        }
    }

    Ok(DerivePlan {
        episodes,
        claims,
        claim_evidence,
        procedures,
        procedure_evidence,
    })
}

fn infer_episode_kind(session: &axiomsync_domain::domain::SessionRow) -> String {
    match session.session_kind.as_str() {
        "run" => "workflow_outcome".to_string(),
        "task" => "task_progress".to_string(),
        "import" => "imported_context".to_string(),
        _ => "conversation".to_string(),
    }
}

fn extract_claim(entries: &[&EntryRow], needles: &[&str]) -> Option<String> {
    for entry in entries {
        let text = entry.text_body.as_deref()?.trim();
        let lowered = text.to_ascii_lowercase();
        if needles.iter().any(|needle| lowered.contains(needle)) {
            return Some(truncate(text, 160));
        }
    }
    None
}

fn extract_procedure_steps(entries: &[&EntryRow]) -> Vec<String> {
    let mut steps = Vec::new();
    for entry in entries {
        let Some(text) = entry.text_body.as_deref() else {
            continue;
        };
        for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
            if is_command_like(line) {
                steps.push(line.to_string());
            }
        }
    }
    steps.dedup();
    steps
}

fn is_command_like(line: &str) -> bool {
    let lowered = line.to_ascii_lowercase();
    lowered.starts_with("$ ")
        || lowered.starts_with("git ")
        || lowered.starts_with("cargo ")
        || lowered.starts_with("npm ")
        || lowered.starts_with("pnpm ")
        || lowered.starts_with("yarn ")
        || lowered.starts_with("make ")
        || lowered.starts_with("python ")
        || lowered.starts_with("uv ")
        || lowered.starts_with("pytest ")
}

fn truncate(value: &str, max_len: usize) -> String {
    if value.chars().count() <= max_len {
        value.to_string()
    } else {
        value.chars().take(max_len).collect::<String>()
    }
}
