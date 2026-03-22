use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{AxiomError, Result};

use super::conversation::{
    ArtifactRow, ConvItemRow, ConvSessionRow, ConvTurnRow, EvidenceAnchorRow, ImportJournalRow,
    RawEventRow, SourceCursorRow, WorkspaceRow,
};
use super::connectors::{EpisodeExtraction, VerificationExtraction};
use super::derived::{
    EpisodeMemberRow, EpisodeRow, InsightAnchorRow, InsightRow, SearchDocRedactedRow,
    VerificationRow,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RawEventInput {
    pub connector: String,
    pub native_schema_version: Option<String>,
    pub native_session_id: String,
    pub native_event_id: Option<String>,
    pub event_type: String,
    pub ts_ms: i64,
    pub payload: Value,
}

impl RawEventInput {
    pub fn validate(&self) -> Result<()> {
        if self.connector.trim().is_empty()
            || self.native_session_id.trim().is_empty()
            || self.event_type.trim().is_empty()
        {
            return Err(AxiomError::Validation(
                "raw event input requires connector, native_session_id, event_type".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CursorInput {
    pub cursor_key: String,
    pub cursor_value: String,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectorBatchInput {
    pub events: Vec<RawEventInput>,
    pub cursor: Option<CursorInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExistingRawEventKey {
    pub stable_id: String,
    pub dedupe_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizedRawEvent {
    pub row: RawEventRow,
    pub dedupe_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IngestPlan {
    pub adds: Vec<NormalizedRawEvent>,
    pub cursor_update: Option<SourceCursorRow>,
    pub skipped_dedupe_keys: Vec<String>,
    pub journal: Option<ImportJournalRow>,
}

impl IngestPlan {
    pub fn validate(&self) -> Result<()> {
        for add in &self.adds {
            add.row.validate()?;
        }
        if let Some(cursor) = &self.cursor_update {
            cursor.validate()?;
        }
        if let Some(journal) = &self.journal {
            journal.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectionPlan {
    pub workspaces: Vec<WorkspaceRow>,
    pub conv_sessions: Vec<ConvSessionRow>,
    pub conv_turns: Vec<ConvTurnRow>,
    pub conv_items: Vec<ConvItemRow>,
    pub artifacts: Vec<ArtifactRow>,
    pub evidence_anchors: Vec<EvidenceAnchorRow>,
}

impl ProjectionPlan {
    pub fn validate(&self) -> Result<()> {
        let workspace_ids: HashSet<_> = self
            .workspaces
            .iter()
            .map(|workspace| workspace.stable_id.as_str())
            .collect();
        let session_ids: HashSet<_> = self
            .conv_sessions
            .iter()
            .map(|session| session.stable_id.as_str())
            .collect();
        let turn_ids: HashSet<_> = self
            .conv_turns
            .iter()
            .map(|turn| turn.stable_id.as_str())
            .collect();
        let item_ids: HashSet<_> = self
            .conv_items
            .iter()
            .map(|item| item.stable_id.as_str())
            .collect();

        for workspace in &self.workspaces {
            workspace.validate()?;
        }
        for session in &self.conv_sessions {
            session.validate()?;
            if session
                .workspace_id
                .as_deref()
                .is_some_and(|workspace_id| !workspace_ids.contains(workspace_id))
            {
                return Err(AxiomError::Validation(format!(
                    "conv_session {} references unknown workspace {}",
                    session.stable_id,
                    session.workspace_id.as_deref().unwrap_or_default()
                )));
            }
        }
        for turn in &self.conv_turns {
            turn.validate()?;
            if !session_ids.contains(turn.session_id.as_str()) {
                return Err(AxiomError::Validation(format!(
                    "conv_turn {} references unknown session {}",
                    turn.stable_id, turn.session_id
                )));
            }
            if !self
                .conv_items
                .iter()
                .any(|item| item.turn_id == turn.stable_id)
            {
                return Err(AxiomError::Validation(format!(
                    "turn {} must contain at least one item",
                    turn.stable_id
                )));
            }
        }
        for item in &self.conv_items {
            item.validate()?;
            if !turn_ids.contains(item.turn_id.as_str()) {
                return Err(AxiomError::Validation(format!(
                    "conv_item {} references unknown turn {}",
                    item.stable_id, item.turn_id
                )));
            }
        }
        for artifact in &self.artifacts {
            artifact.validate()?;
            if !item_ids.contains(artifact.item_id.as_str()) {
                return Err(AxiomError::Validation(format!(
                    "artifact {} references unknown item {}",
                    artifact.stable_id, artifact.item_id
                )));
            }
        }
        for anchor in &self.evidence_anchors {
            anchor.validate()?;
            if !item_ids.contains(anchor.item_id.as_str()) {
                return Err(AxiomError::Validation(format!(
                    "evidence_anchor {} references unknown item {}",
                    anchor.stable_id, anchor.item_id
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DerivePlan {
    pub episodes: Vec<EpisodeRow>,
    pub episode_members: Vec<EpisodeMemberRow>,
    pub insights: Vec<InsightRow>,
    pub insight_anchors: Vec<InsightAnchorRow>,
    pub verifications: Vec<VerificationRow>,
    pub search_docs_redacted: Vec<SearchDocRedactedRow>,
}

impl DerivePlan {
    pub fn validate(&self) -> Result<()> {
        let episode_ids: HashSet<_> = self
            .episodes
            .iter()
            .map(|episode| episode.stable_id.as_str())
            .collect();
        let insight_ids: HashSet<_> = self
            .insights
            .iter()
            .map(|insight| insight.stable_id.as_str())
            .collect();
        let anchored: HashSet<_> = self
            .insight_anchors
            .iter()
            .map(|row| row.insight_id.as_str())
            .collect();
        for episode in &self.episodes {
            episode.validate()?;
            if !self
                .episode_members
                .iter()
                .any(|member| member.episode_id == episode.stable_id)
            {
                return Err(AxiomError::Validation(format!(
                    "episode {} is missing members",
                    episode.stable_id
                )));
            }
        }
        for member in &self.episode_members {
            member.validate()?;
            if !episode_ids.contains(member.episode_id.as_str()) {
                return Err(AxiomError::Validation(format!(
                    "episode_member references unknown episode {}",
                    member.episode_id
                )));
            }
        }
        for insight in &self.insights {
            insight.validate()?;
            if !episode_ids.contains(insight.episode_id.as_str()) {
                return Err(AxiomError::Validation(format!(
                    "insight {} references unknown episode {}",
                    insight.stable_id, insight.episode_id
                )));
            }
            if !anchored.contains(insight.stable_id.as_str()) {
                return Err(AxiomError::Validation(format!(
                    "insight {} is missing evidence anchor",
                    insight.stable_id
                )));
            }
        }
        for link in &self.insight_anchors {
            link.validate()?;
            if !insight_ids.contains(link.insight_id.as_str()) {
                return Err(AxiomError::Validation(format!(
                    "insight_anchor references unknown insight {}",
                    link.insight_id
                )));
            }
        }
        for verification in &self.verifications {
            verification.validate()?;
            if !episode_ids.contains(verification.episode_id.as_str()) {
                return Err(AxiomError::Validation(format!(
                    "verification {} references unknown episode {}",
                    verification.stable_id, verification.episode_id
                )));
            }
        }
        for doc in &self.search_docs_redacted {
            doc.validate()?;
            if !episode_ids.contains(doc.episode_id.as_str()) {
                return Err(AxiomError::Validation(format!(
                    "search_doc_redacted {} references unknown episode {}",
                    doc.stable_id, doc.episode_id
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReplayPlan {
    pub projection: ProjectionPlan,
    pub derivation: DerivePlan,
}

impl ReplayPlan {
    pub fn validate(&self) -> Result<()> {
        self.projection.validate()?;
        self.derivation.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PurgePlan {
    pub connector: Option<String>,
    pub workspace_id: Option<String>,
    pub deleted_raw_event_ids: Vec<String>,
    pub projection: ProjectionPlan,
    pub derivation: DerivePlan,
}

impl PurgePlan {
    pub fn validate(&self) -> Result<()> {
        if self
            .connector
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(AxiomError::Validation(
                "purge connector must not be empty".to_string(),
            ));
        }
        if self
            .workspace_id
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(AxiomError::Validation(
                "purge workspace_id must not be empty".to_string(),
            ));
        }
        self.projection.validate()?;
        self.derivation.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepairPlan {
    pub ingest: IngestPlan,
    pub replay: ReplayPlan,
}

impl RepairPlan {
    pub fn validate(&self) -> Result<()> {
        self.ingest.validate()?;
        self.replay.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DerivationContext {
    pub episode_id: String,
    pub workspace_id: Option<String>,
    pub turn_ids: Vec<String>,
    pub opened_at_ms: i64,
    pub closed_at_ms: Option<i64>,
    pub transcript: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DerivationEnrichment {
    pub extractions: std::collections::HashMap<String, EpisodeExtraction>,
    pub verifications: std::collections::HashMap<String, Vec<VerificationExtraction>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceTokenPlan {
    pub workspace_id: String,
    pub token_sha256: String,
}
