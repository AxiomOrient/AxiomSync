use serde::{Deserialize, Serialize};

use crate::error::{AxiomError, Result};

use super::enums::{EpisodeStatus, InsightKind, VerificationKind, VerificationStatus};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EpisodeRow {
    pub stable_id: String,
    pub workspace_id: Option<String>,
    pub problem_signature: String,
    pub status: EpisodeStatus,
    pub opened_at_ms: i64,
    pub closed_at_ms: Option<i64>,
}

impl EpisodeRow {
    pub fn validate(&self) -> Result<()> {
        if self.stable_id.trim().is_empty() || self.problem_signature.trim().is_empty() {
            return Err(AxiomError::Validation(
                "episode requires stable_id and problem_signature".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EpisodeMemberRow {
    pub episode_id: String,
    pub turn_id: String,
}

impl EpisodeMemberRow {
    pub fn validate(&self) -> Result<()> {
        if self.episode_id.trim().is_empty() || self.turn_id.trim().is_empty() {
            return Err(AxiomError::Validation(
                "episode_member requires episode_id and turn_id".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InsightRow {
    pub stable_id: String,
    pub episode_id: String,
    pub kind: InsightKind,
    pub summary: String,
    pub normalized_text: String,
    pub extractor_version: String,
    pub confidence: f64,
    pub stale: bool,
}

impl InsightRow {
    pub fn validate(&self) -> Result<()> {
        if self.stable_id.trim().is_empty()
            || self.episode_id.trim().is_empty()
            || self.summary.trim().is_empty()
            || self.extractor_version.trim().is_empty()
        {
            return Err(AxiomError::Validation(
                "insight requires stable_id, episode_id, summary, extractor_version".to_string(),
            ));
        }
        if !(0.0..=1.0).contains(&self.confidence) {
            return Err(AxiomError::Validation(
                "insight confidence must be between 0 and 1".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InsightAnchorRow {
    pub insight_id: String,
    pub anchor_id: String,
}

impl InsightAnchorRow {
    pub fn validate(&self) -> Result<()> {
        if self.insight_id.trim().is_empty() || self.anchor_id.trim().is_empty() {
            return Err(AxiomError::Validation(
                "insight_anchor requires insight_id and anchor_id".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationRow {
    pub stable_id: String,
    pub episode_id: String,
    pub kind: VerificationKind,
    pub status: VerificationStatus,
    pub summary: Option<String>,
    pub evidence_id: Option<String>,
}

impl VerificationRow {
    pub fn validate(&self) -> Result<()> {
        if self.stable_id.trim().is_empty() || self.episode_id.trim().is_empty() {
            return Err(AxiomError::Validation(
                "verification requires stable_id and episode_id".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchDocRedactedRow {
    pub stable_id: String,
    pub episode_id: String,
    pub body: String,
}

impl SearchDocRedactedRow {
    pub fn validate(&self) -> Result<()> {
        if self.stable_id.trim().is_empty()
            || self.episode_id.trim().is_empty()
            || self.body.trim().is_empty()
        {
            return Err(AxiomError::Validation(
                "search_doc_redacted requires stable_id, episode_id, body".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThreadView {
    pub session: super::conversation::ConvSessionRow,
    pub turns: Vec<ThreadTurnView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThreadTurnView {
    pub turn: super::conversation::ConvTurnRow,
    pub items: Vec<ThreadItemView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThreadItemView {
    pub item: super::conversation::ConvItemRow,
    pub artifacts: Vec<super::conversation::ArtifactRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceView {
    pub evidence: super::conversation::EvidenceAnchorRow,
    pub item: super::conversation::ConvItemRow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunbookRecord {
    pub episode_id: String,
    pub workspace_id: Option<String>,
    pub problem: String,
    pub root_cause: Option<String>,
    pub fix: Option<String>,
    pub commands: Vec<String>,
    pub verification: Vec<RunbookVerification>,
    pub evidence: Vec<String>,
}

impl RunbookRecord {
    pub fn validate(&self) -> Result<()> {
        if self.problem.trim().is_empty() {
            return Err(AxiomError::Validation(
                "runbook.problem must not be empty".to_string(),
            ));
        }
        if self
            .commands
            .iter()
            .any(|command| command.trim().is_empty())
        {
            return Err(AxiomError::Validation(
                "runbook.commands must not contain empty values".to_string(),
            ));
        }
        for verification in &self.verification {
            verification.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunbookVerification {
    pub kind: VerificationKind,
    pub status: VerificationStatus,
    pub summary: Option<String>,
    pub evidence: Option<String>,
}

impl RunbookVerification {
    pub fn validate(&self) -> Result<()> {
        if let Some(evidence) = &self.evidence
            && evidence.trim().is_empty()
        {
            return Err(AxiomError::Validation(
                "runbook verification evidence must not be empty".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DoctorReport {
    pub schema_version: String,
    pub stored_schema_version: Option<String>,
    pub version_mismatch: bool,
    pub fts_rebuild_required: bool,
    pub drift_detected: bool,
    pub missing_tables: Vec<String>,
    pub missing_indexes: Vec<String>,
}
