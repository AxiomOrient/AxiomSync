use serde::{Deserialize, Serialize};

use super::enums::{VerificationKind, VerificationStatus};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct EpisodeExtraction {
    pub problem: String,
    pub root_cause: Option<String>,
    pub fix: Option<String>,
    pub commands: Vec<String>,
    pub decisions: Vec<String>,
    pub snippets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerificationExtraction {
    pub kind: VerificationKind,
    pub status: VerificationStatus,
    pub summary: Option<String>,
    pub evidence: Option<String>,
    pub pass_condition: Option<String>,
    pub exit_code: Option<i64>,
    pub human_confirmed: bool,
}

impl Default for VerificationExtraction {
    fn default() -> Self {
        Self {
            kind: VerificationKind::Test,
            status: VerificationStatus::Unknown,
            summary: None,
            evidence: None,
            pass_condition: None,
            exit_code: None,
            human_confirmed: false,
        }
    }
}
