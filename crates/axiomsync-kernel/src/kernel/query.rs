use super::*;
use crate::domain::{InsightAnchorRow, InsightRow, VerificationRow};

impl AxiomSync {
    pub fn search_episodes(
        &self,
        request: SearchEpisodesRequest,
    ) -> Result<Vec<SearchEpisodesResult>> {
        let fts_rows = self
            .repo
            .load_episode_search_fts_rows(&request.query, request.limit)?;
        let insights = self.repo.load_insights()?;
        // Only load the fallback data when FTS found no results
        let (episodes, evidence_rows, verifications, connectors) = if fts_rows.is_empty() {
            (
                self.repo.load_episodes()?,
                self.repo.load_episode_evidence_search_rows()?,
                self.repo.load_verifications()?,
                self.repo.load_episode_connectors()?,
            )
        } else {
            (Vec::new(), Vec::new(), Vec::new(), Vec::new())
        };
        Ok(search_episode_results(
            &request.query,
            request.limit,
            &request.filter,
            EpisodeSearchRows {
                fts_rows: &fts_rows,
                evidence_rows: &evidence_rows,
                episodes: &episodes,
                insights: &insights,
                verifications: &verifications,
                connector_rows: &connectors,
            },
        ))
    }

    pub fn search_commands(&self, query: &str, limit: usize) -> Result<Vec<SearchCommandsResult>> {
        let candidates = self.repo.load_command_search_candidates()?;
        Ok(search_command_results(query, limit, &candidates, None))
    }

    pub fn search_commands_in_workspace(
        &self,
        query: &str,
        limit: usize,
        workspace_id: &str,
    ) -> Result<Vec<SearchCommandsResult>> {
        let candidates = self.repo.load_command_search_candidates()?;
        Ok(search_command_results(
            query,
            limit,
            &candidates,
            Some(workspace_id),
        ))
    }

    pub fn get_runbook(&self, episode_id: &str) -> Result<RunbookRecord> {
        let episode = self
            .repo
            .load_episodes()?
            .into_iter()
            .find(|row| row.stable_id == episode_id)
            .ok_or_else(|| AxiomError::NotFound(format!("runbook {episode_id}")))?;
        let insights = self
            .repo
            .load_insights()?
            .into_iter()
            .filter(|row| row.episode_id == episode_id)
            .collect::<Vec<_>>();
        let insight_anchors = self
            .repo
            .load_insight_anchors()?
            .into_iter()
            .filter(|row| {
                insights
                    .iter()
                    .any(|insight| insight.stable_id == row.insight_id)
            })
            .collect::<Vec<_>>();
        let verifications = self
            .repo
            .load_verifications()?
            .into_iter()
            .filter(|row| row.episode_id == episode_id)
            .collect::<Vec<_>>();
        synthesize_runbook(&episode, &insights, &insight_anchors, &verifications)
    }

    pub fn list_runbooks(&self) -> Result<Vec<RunbookRecord>> {
        let episodes = self.repo.load_episodes()?;
        let insights = self.repo.load_insights()?;
        let insight_anchors = self.repo.load_insight_anchors()?;
        let verifications = self.repo.load_verifications()?;

        // Map insight stable_id → episode_id for anchor grouping
        let insight_to_episode: std::collections::HashMap<&str, &str> = insights
            .iter()
            .map(|row| (row.stable_id.as_str(), row.episode_id.as_str()))
            .collect();

        // Pre-group all collections by episode_id
        let mut insights_by_episode: std::collections::HashMap<&str, Vec<InsightRow>> =
            std::collections::HashMap::new();
        for row in &insights {
            insights_by_episode
                .entry(row.episode_id.as_str())
                .or_default()
                .push(row.clone());
        }
        let mut anchors_by_episode: std::collections::HashMap<&str, Vec<InsightAnchorRow>> =
            std::collections::HashMap::new();
        for row in &insight_anchors {
            if let Some(&episode_id) = insight_to_episode.get(row.insight_id.as_str()) {
                anchors_by_episode
                    .entry(episode_id)
                    .or_default()
                    .push(row.clone());
            }
        }
        let mut verifications_by_episode: std::collections::HashMap<&str, Vec<VerificationRow>> =
            std::collections::HashMap::new();
        for row in &verifications {
            verifications_by_episode
                .entry(row.episode_id.as_str())
                .or_default()
                .push(row.clone());
        }

        let mut runbooks = Vec::new();
        for episode in &episodes {
            let ep_id = episode.stable_id.as_str();
            let ep_insights = insights_by_episode.get(ep_id).map_or(&[][..], Vec::as_slice);
            let ep_anchors = anchors_by_episode.get(ep_id).map_or(&[][..], Vec::as_slice);
            let ep_verifs = verifications_by_episode.get(ep_id).map_or(&[][..], Vec::as_slice);
            runbooks.push(synthesize_runbook(episode, ep_insights, ep_anchors, ep_verifs)?);
        }
        Ok(runbooks)
    }

    pub fn get_thread(&self, session_id: &str) -> Result<ThreadView> {
        self.repo.get_thread(session_id)
    }

    pub fn get_evidence(&self, evidence_id: &str) -> Result<EvidenceView> {
        self.repo.get_evidence(evidence_id)
    }

    pub fn runbook_workspace_id(&self, episode_id: &str) -> Result<Option<String>> {
        self.repo.episode_workspace_id(episode_id)
    }

    pub fn thread_workspace_id(&self, session_id: &str) -> Result<Option<String>> {
        self.repo.thread_workspace_id(session_id)
    }

    pub fn evidence_workspace_id(&self, evidence_id: &str) -> Result<Option<String>> {
        self.repo.evidence_workspace_id(evidence_id)
    }

    pub fn connector_status(&self) -> Result<Value> {
        let cursors = self.repo.load_source_cursors()?;
        let config = self.load_connectors_config()?;
        Ok(serde_json::json!({
            "config": config,
            "cursors": cursors,
        }))
    }

    pub fn source_cursors(&self) -> Result<Vec<SourceCursorRow>> {
        self.repo.load_source_cursors()
    }
}
