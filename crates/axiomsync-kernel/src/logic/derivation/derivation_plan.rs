use std::collections::HashMap;

use crate::domain::{
    ConvItemRow, ConvTurnRow, DerivationContext, DerivePlan, EpisodeExtraction, EpisodeMemberRow,
    EpisodeRow, EpisodeStatus, EvidenceAnchorRow, InsightAnchorRow, InsightKind, InsightRow,
    VerificationExtraction, VerificationRow, VerificationStatus, build_search_doc_redacted,
    stable_hash, stable_id,
};
use crate::error::Result;

fn find_anchor_for_text(
    anchor_pool: &[EvidenceAnchorRow],
    items_by_id: &HashMap<String, ConvItemRow>,
    needle: Option<&str>,
) -> Option<String> {
    let needle = needle.map(str::trim).filter(|text| !text.is_empty())?;
    let needle_lower = needle.to_ascii_lowercase();
    anchor_pool.iter().find_map(|anchor| {
        let quoted = anchor
            .quoted_text
            .as_ref()
            .map(|value| value.to_ascii_lowercase())
            .unwrap_or_default();
        let body = items_by_id
            .get(&anchor.item_id)
            .and_then(|item| item.body_text.as_ref())
            .map(|value| value.to_ascii_lowercase())
            .unwrap_or_default();
        if quoted.contains(&needle_lower) || body.contains(&needle_lower) {
            Some(anchor.stable_id.clone())
        } else {
            None
        }
    })
}

pub fn plan_derivation(
    contexts: &[DerivationContext],
    turns: &[ConvTurnRow],
    items: &[ConvItemRow],
    anchors: &[EvidenceAnchorRow],
    extractions: &HashMap<String, EpisodeExtraction>,
    verifications: &HashMap<String, Vec<VerificationExtraction>>,
) -> Result<DerivePlan> {
    let turns_by_id: HashMap<_, _> = turns
        .iter()
        .map(|turn| (turn.stable_id.clone(), turn.clone()))
        .collect();
    let items_by_turn: HashMap<_, _> = items.iter().fold(
        HashMap::<String, Vec<ConvItemRow>>::new(),
        |mut acc, item| {
            acc.entry(item.turn_id.clone())
                .or_default()
                .push(item.clone());
            acc
        },
    );
    let items_by_id: HashMap<_, _> = items
        .iter()
        .map(|item| (item.stable_id.clone(), item.clone()))
        .collect();
    let anchors_by_item_id: HashMap<String, &EvidenceAnchorRow> =
        anchors.iter().fold(HashMap::new(), |mut acc, anchor| {
            acc.entry(anchor.item_id.clone()).or_insert(anchor);
            acc
        });

    let mut episodes = Vec::new();
    let mut episode_members = Vec::new();
    let mut insights = Vec::new();
    let mut insight_anchors = Vec::new();
    let mut verification_rows = Vec::new();

    for context in contexts {
        let extraction = extractions
            .get(&context.episode_id)
            .cloned()
            .unwrap_or_else(|| EpisodeExtraction {
                problem: context
                    .transcript
                    .lines()
                    .next()
                    .unwrap_or("Conversation episode")
                    .to_string(),
                ..EpisodeExtraction::default()
            });
        let turn_anchor_pool = context
            .turn_ids
            .iter()
            .flat_map(|turn_id| {
                items_by_turn
                    .get(turn_id)
                    .into_iter()
                    .flat_map(|items| items.iter())
                    .filter_map(|item| {
                        anchors_by_item_id.get(&item.stable_id).map(|a| (*a).clone())
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let episode_status = if verifications
            .get(&context.episode_id)
            .into_iter()
            .flatten()
            .any(|verification| verification.status == VerificationStatus::Pass)
        {
            EpisodeStatus::Solved
        } else {
            EpisodeStatus::Open
        };

        episodes.push(EpisodeRow {
            stable_id: context.episode_id.clone(),
            workspace_id: context.workspace_id.clone(),
            problem_signature: stable_hash(&[extraction.problem.as_str()]),
            status: episode_status,
            opened_at_ms: context.opened_at_ms,
            closed_at_ms: context.closed_at_ms,
        });

        for turn_id in &context.turn_ids {
            if turns_by_id.contains_key(turn_id) {
                episode_members.push(EpisodeMemberRow {
                    episode_id: context.episode_id.clone(),
                    turn_id: turn_id.clone(),
                });
            }
        }

        let mut push_insight = |kind: InsightKind, summary: Option<String>, confidence: f64| {
            let Some(summary) = summary.filter(|value| !value.trim().is_empty()) else {
                return;
            };
            let stable_id_value = stable_id(
                "insight",
                &(context.episode_id.as_str(), kind.as_str(), summary.as_str()),
            );
            let anchor_id = find_anchor_for_text(&turn_anchor_pool, &items_by_id, Some(&summary))
                .or_else(|| {
                    turn_anchor_pool
                        .first()
                        .map(|anchor| anchor.stable_id.clone())
                });
            insights.push(InsightRow {
                stable_id: stable_id_value.clone(),
                episode_id: context.episode_id.clone(),
                kind,
                summary: summary.clone(),
                normalized_text: summary.to_ascii_lowercase(),
                extractor_version: "episode_extractor_v1".to_string(),
                confidence,
                stale: false,
            });
            if let Some(anchor_id) = anchor_id {
                insight_anchors.push(InsightAnchorRow {
                    insight_id: stable_id_value,
                    anchor_id,
                });
            }
        };

        push_insight(InsightKind::Problem, Some(extraction.problem.clone()), 0.9);
        push_insight(InsightKind::RootCause, extraction.root_cause.clone(), 0.7);
        push_insight(InsightKind::Fix, extraction.fix.clone(), 0.8);
        for decision in extraction.decisions {
            push_insight(InsightKind::Decision, Some(decision), 0.6);
        }
        for command in extraction.commands {
            push_insight(InsightKind::Command, Some(command), 0.85);
        }
        for snippet in extraction.snippets {
            push_insight(InsightKind::Snippet, Some(snippet), 0.55);
        }

        for verification in verifications
            .get(&context.episode_id)
            .cloned()
            .unwrap_or_default()
        {
            let evidence_id = find_anchor_for_text(
                &turn_anchor_pool,
                &items_by_id,
                verification
                    .evidence
                    .as_deref()
                    .or(verification.summary.as_deref()),
            );
            verification_rows.push(VerificationRow {
                stable_id: stable_id(
                    "verification",
                    &(
                        context.episode_id.as_str(),
                        verification.kind.as_str(),
                        verification.status.as_str(),
                        verification.summary.as_deref().unwrap_or(""),
                    ),
                ),
                episode_id: context.episode_id.clone(),
                kind: verification.kind,
                status: verification.status,
                summary: verification.summary,
                evidence_id,
            });
        }
    }

    insights.sort_by(|left, right| left.stable_id.cmp(&right.stable_id));
    insight_anchors.sort_by(|left, right| {
        left.insight_id
            .cmp(&right.insight_id)
            .then(left.anchor_id.cmp(&right.anchor_id))
    });
    verification_rows.sort_by(|left, right| left.stable_id.cmp(&right.stable_id));

    let mut insights_by_episode: HashMap<&str, Vec<&InsightRow>> = HashMap::new();
    for insight in &insights {
        insights_by_episode
            .entry(insight.episode_id.as_str())
            .or_default()
            .push(insight);
    }
    let mut verifs_by_episode: HashMap<&str, Vec<&VerificationRow>> = HashMap::new();
    for row in &verification_rows {
        verifs_by_episode
            .entry(row.episode_id.as_str())
            .or_default()
            .push(row);
    }
    let search_docs_redacted = episodes
        .iter()
        .map(|episode| {
            let ep_insights = insights_by_episode
                .get(episode.stable_id.as_str())
                .map(|v| v.iter().map(|r| (*r).clone()).collect::<Vec<_>>())
                .unwrap_or_default();
            let ep_verifications = verifs_by_episode
                .get(episode.stable_id.as_str())
                .map(|v| v.iter().map(|r| (*r).clone()).collect::<Vec<_>>())
                .unwrap_or_default();
            build_search_doc_redacted(&episode.stable_id, &ep_insights, &ep_verifications)
        })
        .collect::<Vec<_>>();

    let plan = DerivePlan {
        episodes,
        episode_members,
        insights,
        insight_anchors,
        verifications: verification_rows,
        search_docs_redacted,
    };
    plan.validate()?;
    Ok(plan)
}
