use serde::{Deserialize, Serialize};

use super::derived::{InsightRow, SearchDocRedactedRow, VerificationRow};
use super::enums::{EpisodeStatus, InsightKind};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchEpisodesResult {
    pub episode_id: String,
    pub workspace_id: Option<String>,
    pub connector: Option<String>,
    pub status: EpisodeStatus,
    pub problem: String,
    pub root_cause: Option<String>,
    pub fix: Option<String>,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchEpisodesRequest {
    pub query: String,
    pub limit: usize,
    pub filter: SearchEpisodesFilter,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct SearchEpisodesFilter {
    pub connector: Option<String>,
    pub workspace_id: Option<String>,
    pub status: Option<EpisodeStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchCommandsResult {
    pub episode_id: String,
    pub command: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EpisodeConnectorRow {
    pub episode_id: String,
    pub connector: Option<String>,
    pub turn_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchEpisodeFtsRow {
    pub episode_id: String,
    pub workspace_id: Option<String>,
    pub connector: Option<String>,
    pub status: EpisodeStatus,
    pub matched_kind: Option<InsightKind>,
    pub matched_summary: Option<String>,
    pub pass_boost: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EpisodeEvidenceSearchRow {
    pub episode_id: String,
    pub workspace_id: Option<String>,
    pub connector: Option<String>,
    pub status: EpisodeStatus,
    pub evidence_id: String,
    pub quoted_text: Option<String>,
    pub body_text: Option<String>,
    pub pass_boost: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchCommandCandidateRow {
    pub episode_id: String,
    pub workspace_id: Option<String>,
    pub command: String,
}

pub fn build_search_doc_redacted(
    episode_id: &str,
    insights: &[InsightRow],
    verifications: &[VerificationRow],
) -> SearchDocRedactedRow {
    let mut body = insights
        .iter()
        .map(|insight| format!("{}: {}", insight.kind, insight.summary))
        .collect::<Vec<_>>();
    body.extend(
        verifications
            .iter()
            .filter_map(|verification| verification.summary.as_ref())
            .map(|summary| format!("verification: {summary}")),
    );
    SearchDocRedactedRow {
        stable_id: super::canonical::stable_id("search_doc", &(episode_id, &body)),
        episode_id: episode_id.to_string(),
        body: body.join("\n"),
    }
}
