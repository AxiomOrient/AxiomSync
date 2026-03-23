use std::collections::{HashMap, HashSet};

use axiomsync_domain::domain::{
    AnchorRow, ClaimEvidenceRow, ClaimRow, DerivePlan, EntryRow, EpisodeRow, InsightAnchorRow,
    InsightRow, ProcedureEvidenceRow, ProcedureRow, SearchDocsRow, SessionRow, VerificationRow,
    stable_id,
};
use axiomsync_domain::error::Result;
use serde_json::json;

pub fn plan_derivation(
    sessions: &[SessionRow],
    entries: &[EntryRow],
    anchors: &[AnchorRow],
) -> Result<DerivePlan> {
    let mut episodes = Vec::new();
    let mut insights = Vec::new();
    let mut insight_anchors = Vec::new();
    let mut verifications = Vec::new();
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
            extractor_version: "heuristic-v2".to_string(),
            stale: false,
        });

        let anchor_ids = session_entries
            .iter()
            .flat_map(|entry| {
                anchors_by_entry
                    .get(entry.entry_id.as_str())
                    .into_iter()
                    .flatten()
                    .map(|anchor| anchor.anchor_id.clone())
            })
            .collect::<Vec<_>>();
        let Some(primary_anchor_id) = anchor_ids.first().cloned() else {
            continue;
        };

        let mut generated_insights = Vec::<(String, String, String, String, f64)>::new();
        generated_insights.push((
            stable_id(
                "insight",
                &(episode_id.as_str(), "summary", summary.as_str()),
            ),
            "summary".to_string(),
            summary.clone(),
            primary_anchor_id.clone(),
            0.7,
        ));
        if let Some(observation) = session_entries
            .iter()
            .find_map(|entry| entry.text_body.as_deref())
            .map(|value| truncate(value, 160))
            .filter(|value| !value.trim().is_empty() && value != &summary)
        {
            generated_insights.push((
                stable_id(
                    "insight",
                    &(episode_id.as_str(), "observation", observation.as_str()),
                ),
                "observation".to_string(),
                observation,
                primary_anchor_id.clone(),
                0.68,
            ));
        }
        if let Some(root_cause) = extract_claim(&session_entries, &["root cause", "because"]) {
            generated_insights.push((
                stable_id(
                    "insight",
                    &(episode_id.as_str(), "root_cause", root_cause.as_str()),
                ),
                "root_cause".to_string(),
                root_cause,
                primary_anchor_id.clone(),
                0.62,
            ));
        }
        if let Some(fix) = extract_claim(&session_entries, &["fix", "resolved", "solution"]) {
            generated_insights.push((
                stable_id("insight", &(episode_id.as_str(), "fix", fix.as_str())),
                "fix".to_string(),
                fix,
                primary_anchor_id.clone(),
                0.64,
            ));
        }
        if let Some(decision) = extract_claim(&session_entries, &["decision", "we chose", "choose"])
        {
            generated_insights.push((
                stable_id(
                    "insight",
                    &(episode_id.as_str(), "decision", decision.as_str()),
                ),
                "decision".to_string(),
                decision,
                primary_anchor_id.clone(),
                0.58,
            ));
        }

        for (insight_id, insight_kind, statement, anchor_id, confidence) in &generated_insights {
            insights.push(InsightRow {
                insight_id: insight_id.clone(),
                episode_id: Some(episode_id.clone()),
                insight_kind: insight_kind.clone(),
                statement: statement.clone(),
                confidence: *confidence,
                scope_json: json!({
                    "session_id": session.session_id,
                    "session_kind": session.session_kind,
                    "connector": session.connector,
                }),
                metadata_json: json!({}),
            });
            insight_anchors.push(InsightAnchorRow {
                insight_id: insight_id.clone(),
                anchor_id: anchor_id.clone(),
            });
            claims.push(ClaimRow {
                claim_id: stable_id(
                    "claim",
                    &(
                        episode_id.as_str(),
                        insight_kind.as_str(),
                        statement.as_str(),
                    ),
                ),
                episode_id: Some(episode_id.clone()),
                claim_kind: insight_kind.clone(),
                statement: statement.clone(),
                confidence: *confidence,
                metadata_json: json!({ "insight_id": insight_id }),
            });
            claim_evidence.push(ClaimEvidenceRow {
                claim_id: stable_id(
                    "claim",
                    &(
                        episode_id.as_str(),
                        insight_kind.as_str(),
                        statement.as_str(),
                    ),
                ),
                anchor_id: anchor_id.clone(),
                support_kind: if insight_kind == "summary" {
                    "primary".to_string()
                } else {
                    "supporting".to_string()
                },
            });
        }

        let commands = extract_procedure_steps(&session_entries);
        let mut procedure_ids = Vec::new();
        if !commands.is_empty() {
            let procedure_id = stable_id("procedure", &(episode_id.as_str(), commands.join("\n")));
            procedure_ids.push(procedure_id.clone());
            procedures.push(ProcedureRow {
                procedure_id: procedure_id.clone(),
                title: session
                    .title
                    .clone()
                    .unwrap_or_else(|| format!("{} procedure", session.session_kind)),
                goal: Some(summary.clone()),
                steps_json: json!(commands),
                status: Some("active".to_string()),
                confidence: 0.7,
                extractor_version: "heuristic-v2".to_string(),
                stale: false,
            });
            procedure_evidence.push(ProcedureEvidenceRow {
                procedure_id: procedure_id.clone(),
                anchor_id: primary_anchor_id.clone(),
                support_kind: "primary".to_string(),
            });
        }

        let verification = build_verification(
            session,
            &session_entries,
            &generated_insights,
            procedure_ids,
            primary_anchor_id,
        );
        verifications.extend(verification);
    }

    let search_docs = plan_search_docs(sessions, &episodes, &insights, &procedures);

    Ok(DerivePlan {
        episodes,
        insights,
        insight_anchors,
        verifications,
        claims,
        claim_evidence,
        procedures,
        procedure_evidence,
        search_docs,
    })
}

fn build_verification(
    session: &SessionRow,
    entries: &[&EntryRow],
    insights: &[(String, String, String, String, f64)],
    procedure_ids: Vec<String>,
    evidence_anchor_id: String,
) -> Vec<VerificationRow> {
    let texts = entries
        .iter()
        .filter_map(|entry| entry.text_body.as_deref())
        .map(|value| value.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let deterministic_verified = entries.iter().any(|entry| {
        let kind = entry.entry_kind.to_ascii_lowercase();
        let text = entry
            .text_body
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase();
        kind.contains("verification_passed")
            || kind.contains("check_result")
            || text.contains("check passed")
            || text.contains("passed")
    });
    let human_verified = texts
        .iter()
        .any(|text| text.contains("human confirmed") || text.contains("approved"));
    let conflict = has_conflict(insights);
    let checked_at = session
        .closed_at
        .clone()
        .or_else(|| session.opened_at.clone())
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());

    let (status, method) = if conflict {
        ("conflicted", "heuristic")
    } else if deterministic_verified {
        ("verified", "deterministic")
    } else if human_verified {
        ("verified", "human")
    } else {
        ("proposed", "heuristic")
    };

    let mut rows = insights
        .iter()
        .filter(|(_, kind, _, _, _)| kind != "summary")
        .map(
            |(insight_id, insight_kind, statement, _, _)| VerificationRow {
                verification_id: stable_id(
                    "verification",
                    &(insight_id.as_str(), status, method, checked_at.as_str()),
                ),
                subject_kind: "insight".to_string(),
                subject_id: insight_id.clone(),
                method: method.to_string(),
                status: status.to_string(),
                checked_at: checked_at.clone(),
                checker: None,
                details_json: json!({
                    "insight_kind": insight_kind,
                    "summary": statement,
                    "evidence_anchor_id": evidence_anchor_id,
                }),
            },
        )
        .collect::<Vec<_>>();
    rows.extend(
        procedure_ids
            .into_iter()
            .map(|procedure_id| VerificationRow {
                verification_id: stable_id(
                    "verification",
                    &(procedure_id.as_str(), status, method, checked_at.as_str()),
                ),
                subject_kind: "procedure".to_string(),
                subject_id: procedure_id,
                method: method.to_string(),
                status: status.to_string(),
                checked_at: checked_at.clone(),
                checker: None,
                details_json: json!({
                    "evidence_anchor_id": evidence_anchor_id,
                }),
            }),
    );
    rows
}

fn has_conflict(insights: &[(String, String, String, String, f64)]) -> bool {
    let mut grouped = HashMap::<&str, HashSet<&str>>::new();
    for (_, kind, statement, _, _) in insights {
        if matches!(kind.as_str(), "root_cause" | "fix" | "decision") {
            grouped
                .entry(kind.as_str())
                .or_default()
                .insert(statement.as_str());
        }
    }
    grouped.values().any(|values| values.len() > 1)
}

fn infer_episode_kind(session: &SessionRow) -> String {
    match session.session_kind.as_str() {
        "run" => "summary".to_string(),
        "task" => "investigation".to_string(),
        "import" => "summary".to_string(),
        _ => "investigation".to_string(),
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

fn plan_search_docs(
    sessions: &[SessionRow],
    episodes: &[EpisodeRow],
    insights: &[InsightRow],
    procedures: &[ProcedureRow],
) -> Vec<SearchDocsRow> {
    let mut docs = Vec::new();

    docs.extend(episodes.iter().map(|episode| SearchDocsRow {
        doc_id: stable_id("search_doc", &("episode", episode.episode_id.as_str())),
        doc_kind: "episode".to_string(),
        subject_kind: "episode".to_string(),
        subject_id: episode.episode_id.clone(),
        title: Some(episode.summary.clone()),
        body: episode.summary.clone(),
        metadata_json: search_doc_metadata(sessions, episode.session_id.as_deref()),
    }));

    docs.extend(insights.iter().map(|insight| SearchDocsRow {
        doc_id: stable_id("search_doc", &("insight", insight.insight_id.as_str())),
        doc_kind: "insight".to_string(),
        subject_kind: "insight".to_string(),
        subject_id: insight.insight_id.clone(),
        title: Some(insight.insight_kind.clone()),
        body: insight.statement.clone(),
        metadata_json: search_doc_metadata(
            sessions,
            insight.episode_id.as_deref().and_then(|episode_id| {
                episodes
                    .iter()
                    .find(|episode| episode.episode_id == episode_id)
                    .and_then(|episode| episode.session_id.as_deref())
            }),
        ),
    }));

    docs.extend(procedures.iter().map(|procedure| {
        SearchDocsRow {
            doc_id: stable_id(
                "search_doc",
                &("procedure", procedure.procedure_id.as_str()),
            ),
            doc_kind: "procedure".to_string(),
            subject_kind: "procedure".to_string(),
            subject_id: procedure.procedure_id.clone(),
            title: Some(procedure.title.clone()),
            body: procedure
                .goal
                .clone()
                .unwrap_or_else(|| procedure.steps_json.to_string()),
            metadata_json: search_doc_metadata(
                sessions,
                episodes
                    .iter()
                    .find(|episode| procedure.goal.as_deref() == Some(episode.summary.as_str()))
                    .and_then(|episode| episode.session_id.as_deref()),
            ),
        }
    }));

    docs
}

fn search_doc_metadata(sessions: &[SessionRow], session_id: Option<&str>) -> serde_json::Value {
    let session =
        session_id.and_then(|id| sessions.iter().find(|session| session.session_id == id));
    json!({
        "session_id": session.map(|session| session.session_id.clone()),
        "session_kind": session.map(|session| session.session_kind.clone()),
        "connector": session.map(|session| session.connector.clone()),
        "workspace_root": session.and_then(|session| session.workspace_root.clone()),
    })
}
