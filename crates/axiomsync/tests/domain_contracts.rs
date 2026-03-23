use axiomsync::domain::{
    ConvSessionRow, ConvTurnRow, DerivePlan, EpisodeMemberRow, EpisodeRow, EpisodeStatus,
    EvidenceAnchorRow, InsightAnchorRow, InsightKind, InsightRow, ProjectionPlan, RunbookRecord,
    SearchDocRedactedRow, SelectorType, VerificationKind, VerificationRow, VerificationStatus,
    WorkspaceRow,
};
use axiomsync::logic::parse_verification_transcript;
use serde_json::json;

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
        episodes: vec![EpisodeRow {
            stable_id: "episode_1".to_string(),
            workspace_id: None,
            problem_signature: "sig".to_string(),
            status: EpisodeStatus::Open,
            opened_at_ms: 1,
            closed_at_ms: None,
        }],
        episode_members: vec![EpisodeMemberRow {
            episode_id: "episode_1".to_string(),
            turn_id: "turn_1".to_string(),
        }],
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
fn projection_plan_requires_item_for_every_turn() {
    let plan = ProjectionPlan {
        workspaces: vec![WorkspaceRow {
            stable_id: "ws_1".to_string(),
            canonical_root: "/repo/app".to_string(),
            repo_remote: None,
            branch: None,
            worktree_path: None,
        }],
        conv_sessions: vec![ConvSessionRow {
            stable_id: "session_1".to_string(),
            connector: "codex".to_string(),
            native_session_id: "native-1".to_string(),
            workspace_id: Some("ws_1".to_string()),
            title: None,
            transcript_uri: None,
            status: "active".to_string(),
            started_at_ms: Some(1),
            ended_at_ms: Some(2),
        }],
        conv_turns: vec![ConvTurnRow {
            stable_id: "turn_1".to_string(),
            session_id: "session_1".to_string(),
            native_turn_id: Some("native-turn-1".to_string()),
            turn_index: 0,
            actor: "user".to_string(),
        }],
        conv_items: vec![],
        artifacts: vec![],
        evidence_anchors: vec![],
        execution_runs: vec![],
        execution_tasks: vec![],
        execution_checks: vec![],
        execution_approvals: vec![],
        execution_events: vec![],
        document_records: vec![],
    };
    assert!(plan.validate().is_err());
}

#[test]
fn derive_plan_requires_member_for_every_episode() {
    let plan = DerivePlan {
        episodes: vec![EpisodeRow {
            stable_id: "episode_1".to_string(),
            workspace_id: None,
            problem_signature: "sig".to_string(),
            status: EpisodeStatus::Open,
            opened_at_ms: 1,
            closed_at_ms: None,
        }],
        episode_members: vec![],
        insights: vec![],
        insight_anchors: vec![],
        verifications: vec![],
        search_docs_redacted: vec![],
    };
    assert!(plan.validate().is_err());
}

#[test]
fn derive_plan_rejects_unknown_episode_references() {
    let plan = DerivePlan {
        episodes: vec![EpisodeRow {
            stable_id: "episode_1".to_string(),
            workspace_id: None,
            problem_signature: "sig".to_string(),
            status: EpisodeStatus::Open,
            opened_at_ms: 1,
            closed_at_ms: None,
        }],
        episode_members: vec![EpisodeMemberRow {
            episode_id: "episode_1".to_string(),
            turn_id: "turn_1".to_string(),
        }],
        insights: vec![],
        insight_anchors: vec![],
        verifications: vec![VerificationRow {
            stable_id: "verification_1".to_string(),
            episode_id: "episode_missing".to_string(),
            kind: VerificationKind::Test,
            status: VerificationStatus::Pass,
            summary: Some("ok".to_string()),
            evidence_id: None,
        }],
        search_docs_redacted: vec![],
    };
    assert!(plan.validate().is_err());
}

#[test]
fn evidence_anchor_validates_supported_selector_types() {
    for (selector_type, selector_json) in [
        (SelectorType::TextSpan, json!({"start": 0, "end": 4})),
        (
            SelectorType::JsonPointer,
            json!({"pointer": "/payload/text"}),
        ),
        (
            SelectorType::DiffHunk,
            json!({"path": "src/lib.rs", "hunk": "@@ -1 +1 @@"}),
        ),
        (
            SelectorType::ArtifactRange,
            json!({"uri": "file:///tmp/out.log", "start": 1, "end": 2}),
        ),
        (
            SelectorType::DomSelector,
            json!({"selector": "#main > pre"}),
        ),
    ] {
        let anchor = EvidenceAnchorRow {
            stable_id: format!("anchor_{}", selector_type.as_str()),
            item_id: "item_1".to_string(),
            selector_type,
            selector_json: selector_json.to_string(),
            quoted_text: Some("evidence".to_string()),
        };
        assert!(anchor.validate().is_ok(), "{}", selector_type.as_str());
    }
}

#[test]
fn runbook_requires_non_empty_problem() {
    let runbook = RunbookRecord {
        episode_id: "episode_1".to_string(),
        workspace_id: None,
        problem: "".to_string(),
        root_cause: None,
        fix: None,
        commands: vec!["cargo test".to_string()],
        verification: vec![],
        evidence: vec![],
    };
    assert!(runbook.validate().is_err());
}

#[test]
fn derive_plan_accepts_consistent_anchor_links() {
    let plan = DerivePlan {
        episodes: vec![EpisodeRow {
            stable_id: "episode_1".to_string(),
            workspace_id: None,
            problem_signature: "sig".to_string(),
            status: EpisodeStatus::Solved,
            opened_at_ms: 1,
            closed_at_ms: Some(2),
        }],
        episode_members: vec![EpisodeMemberRow {
            episode_id: "episode_1".to_string(),
            turn_id: "turn_1".to_string(),
        }],
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
        insight_anchors: vec![InsightAnchorRow {
            insight_id: "insight_1".to_string(),
            anchor_id: "anchor_1".to_string(),
        }],
        verifications: vec![VerificationRow {
            stable_id: "verification_1".to_string(),
            episode_id: "episode_1".to_string(),
            kind: VerificationKind::Test,
            status: VerificationStatus::Pass,
            summary: Some("passed".to_string()),
            evidence_id: Some("anchor_1".to_string()),
        }],
        search_docs_redacted: vec![SearchDocRedactedRow {
            stable_id: "doc_1".to_string(),
            episode_id: "episode_1".to_string(),
            body: "problem: timeout".to_string(),
        }],
    };
    assert!(plan.validate().is_ok());
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
