use std::collections::BTreeMap;

use crate::domain::{VerificationExtraction, VerificationKind, VerificationStatus};

pub fn parse_verification_transcript(transcript: &str) -> Vec<VerificationExtraction> {
    let mut rows = Vec::new();
    for line in transcript.lines() {
        let lowered = line.to_ascii_lowercase();
        if lowered.contains("tests passed") || lowered.contains("all tests passed") {
            rows.push(VerificationExtraction {
                kind: VerificationKind::Test,
                status: VerificationStatus::Pass,
                summary: Some(line.trim().to_string()),
                evidence: Some(line.trim().to_string()),
                pass_condition: None,
                exit_code: None,
                human_confirmed: false,
            });
        }
        if let Some(exit_code) = extract_exit_code(&lowered) {
            rows.push(VerificationExtraction {
                kind: VerificationKind::CommandExit,
                status: if exit_code == 0 {
                    VerificationStatus::Pass
                } else {
                    VerificationStatus::Fail
                },
                summary: Some(line.trim().to_string()),
                evidence: Some(line.trim().to_string()),
                pass_condition: None,
                exit_code: Some(exit_code),
                human_confirmed: false,
            });
        }
        if is_human_confirmation(&lowered) {
            rows.push(VerificationExtraction {
                kind: VerificationKind::HumanConfirm,
                status: VerificationStatus::Pass,
                summary: Some(line.trim().to_string()),
                evidence: Some(line.trim().to_string()),
                pass_condition: None,
                exit_code: None,
                human_confirmed: true,
            });
        }
    }
    rows
}

pub fn merge_verification_extractions(
    candidates: &[VerificationExtraction],
) -> Vec<VerificationExtraction> {
    let mut ordered = BTreeMap::<String, VerificationExtraction>::new();
    for candidate in candidates {
        let key = format!(
            "{}:{}:{}:{}",
            candidate.kind,
            candidate.status,
            candidate.summary.as_deref().unwrap_or_default(),
            candidate.exit_code.unwrap_or_default()
        );
        ordered.entry(key).or_insert_with(|| candidate.clone());
    }
    ordered.into_values().collect()
}

fn extract_exit_code(text: &str) -> Option<i64> {
    let marker = "exit code:";
    let index = text.find(marker)?;
    text[index + marker.len()..]
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

fn is_human_confirmation(lowered: &str) -> bool {
    lowered.contains("it worked")
        || lowered.contains("works now")
        || lowered.contains("confirmed")
        || lowered.contains("resolved")
}
