use axiomsync::domain::{
    DerivePlan, EpisodeStatus, InsightKind, InsightRow, SearchDocRedactedRow, SelectorType,
    VerificationKind, VerificationStatus,
};
use axiomsync::logic::parse_verification_transcript;

#[test]
fn string_enums_reject_invalid_values() {
    assert!(SelectorType::parse("bogus").is_err());
    assert!(EpisodeStatus::parse("closed").is_err());
    assert!(VerificationKind::parse("lint").is_err());
    assert!(VerificationStatus::parse("green").is_err());
}

#[test]
fn derive_plan_requires_anchor_for_every_insight() {
    let plan = DerivePlan {
        episodes: vec![],
        episode_members: vec![],
        insights: vec![InsightRow {
            stable_id: "insight_1".to_string(),
            episode_id: "episode_1".to_string(),
            kind: InsightKind::Problem,
            summary: "timeout".to_string(),
            normalized_text: "timeout".to_string(),
            extractor_version: "episode_extractor_v1".to_string(),
            confidence: 0.9,
            stale: false,
        }],
        insight_anchors: vec![],
        verifications: vec![],
        search_docs_redacted: vec![SearchDocRedactedRow {
            stable_id: "doc_1".to_string(),
            episode_id: "episode_1".to_string(),
            body: "problem: timeout".to_string(),
        }],
    };
    assert!(plan.validate().is_err());
}

#[test]
fn transcript_parser_emits_rule_based_verifications() {
    let rows = parse_verification_transcript(
        "tool: tests passed\nshell: exit code: 2\nassistant: it worked after retry",
    );
    assert!(rows.iter().any(|row| {
        row.kind == VerificationKind::Test && row.status == VerificationStatus::Pass
    }));
    assert!(rows.iter().any(|row| {
        row.kind == VerificationKind::CommandExit
            && row.status == VerificationStatus::Fail
            && row.exit_code == Some(2)
    }));
    assert!(rows.iter().any(|row| {
        row.kind == VerificationKind::HumanConfirm
            && row.status == VerificationStatus::Pass
            && row.human_confirmed
    }));
}
