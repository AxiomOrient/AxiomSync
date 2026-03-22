use crate::domain::{
    EpisodeRow, InsightAnchorRow, InsightKind, InsightRow, RunbookRecord, RunbookVerification,
    VerificationRow,
};
use crate::error::Result;

pub fn synthesize_runbook(
    episode: &EpisodeRow,
    insights: &[InsightRow],
    insight_anchors: &[InsightAnchorRow],
    verifications: &[VerificationRow],
) -> Result<RunbookRecord> {
    let problem = insights
        .iter()
        .find(|insight| insight.kind == InsightKind::Problem)
        .map(|insight| insight.summary.clone())
        .unwrap_or_else(|| episode.problem_signature.clone());
    let root_cause = insights
        .iter()
        .find(|insight| insight.kind == InsightKind::RootCause)
        .map(|insight| insight.summary.clone());
    let fix = insights
        .iter()
        .find(|insight| insight.kind == InsightKind::Fix)
        .map(|insight| insight.summary.clone());
    let mut commands = insights
        .iter()
        .filter(|insight| insight.kind == InsightKind::Command)
        .map(|insight| insight.summary.clone())
        .collect::<Vec<_>>();
    commands.sort();
    commands.dedup();
    let mut evidence = insight_anchors
        .iter()
        .filter(|anchor| {
            insights
                .iter()
                .any(|insight| insight.stable_id == anchor.insight_id)
        })
        .map(|anchor| format!("axiom://evidence/{}", anchor.anchor_id))
        .collect::<Vec<_>>();
    evidence.sort();
    evidence.dedup();
    let verification = verifications
        .iter()
        .map(|row| RunbookVerification {
            kind: row.kind,
            status: row.status,
            summary: row.summary.clone(),
            evidence: row
                .evidence_id
                .as_ref()
                .map(|value| format!("axiom://evidence/{value}")),
        })
        .collect::<Vec<_>>();
    let runbook = RunbookRecord {
        episode_id: episode.stable_id.clone(),
        workspace_id: episode.workspace_id.clone(),
        problem,
        root_cause,
        fix,
        commands,
        verification,
        evidence,
    };
    runbook.validate()?;
    Ok(runbook)
}
