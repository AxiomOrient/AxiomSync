use super::*;
use crate::domain::{
    CaseRecord, DocumentRecordRow, DocumentView, ExecutionRunRow, InsightAnchorRow, InsightRow,
    RunView, SearchCasesRequest, SearchCasesResult, SearchEpisodesFilter, TaskView,
    VerificationRow,
};

impl AxiomSync {
    pub fn search_cases(&self, request: SearchCasesRequest) -> Result<Vec<SearchCasesResult>> {
        let rows = self.search_episodes(SearchEpisodesRequest {
            query: request.query,
            limit: request.limit,
            filter: SearchEpisodesFilter {
                source: request.filter.producer,
                workspace_id: request.filter.workspace_id,
                status: request.filter.status,
            },
        })?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

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

    pub fn get_case(&self, case_id: &str) -> Result<CaseRecord> {
        let episode = self
            .repo
            .load_episodes()?
            .into_iter()
            .find(|row| row.stable_id == case_id)
            .ok_or_else(|| AxiomError::NotFound(format!("case {case_id}")))?;
        let insights = self
            .repo
            .load_insights()?
            .into_iter()
            .filter(|row| row.episode_id == case_id)
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
            .filter(|row| row.episode_id == case_id)
            .collect::<Vec<_>>();
        crate::logic::synthesize_case(&episode, &insights, &insight_anchors, &verifications)
    }

    pub fn get_runbook(&self, episode_id: &str) -> Result<RunbookRecord> {
        Ok(self.get_case(episode_id)?.into())
    }

    pub fn list_cases(&self) -> Result<Vec<CaseRecord>> {
        let episodes = self.repo.load_episodes()?;
        let insights = self.repo.load_insights()?;
        let insight_anchors = self.repo.load_insight_anchors()?;
        let verifications = self.repo.load_verifications()?;

        let insight_to_episode: std::collections::HashMap<&str, &str> = insights
            .iter()
            .map(|row| (row.stable_id.as_str(), row.episode_id.as_str()))
            .collect();

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

        let mut cases = Vec::new();
        for episode in &episodes {
            let ep_id = episode.stable_id.as_str();
            let ep_insights = insights_by_episode
                .get(ep_id)
                .map_or(&[][..], Vec::as_slice);
            let ep_anchors = anchors_by_episode.get(ep_id).map_or(&[][..], Vec::as_slice);
            let ep_verifs = verifications_by_episode
                .get(ep_id)
                .map_or(&[][..], Vec::as_slice);
            cases.push(crate::logic::synthesize_case(
                episode,
                ep_insights,
                ep_anchors,
                ep_verifs,
            )?);
        }
        Ok(cases)
    }

    pub fn list_runbooks(&self) -> Result<Vec<RunbookRecord>> {
        Ok(self
            .list_cases()?
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>())
    }

    pub fn get_thread(&self, session_id: &str) -> Result<ThreadView> {
        self.repo.get_thread(session_id)
    }

    pub fn list_runs(&self, workspace_id: Option<&str>) -> Result<Vec<ExecutionRunRow>> {
        Ok(self
            .repo
            .load_execution_runs()?
            .into_iter()
            .filter(|run| {
                workspace_id.is_none_or(|expected| run.workspace_id.as_deref() == Some(expected))
            })
            .collect())
    }

    pub fn get_run(&self, run_id: &str) -> Result<RunView> {
        self.repo.get_run(run_id)
    }

    pub fn get_task(&self, task_id: &str) -> Result<TaskView> {
        self.repo.get_task(task_id)
    }

    pub fn list_documents(
        &self,
        workspace_id: Option<&str>,
        kind: Option<&str>,
    ) -> Result<Vec<DocumentRecordRow>> {
        Ok(self
            .repo
            .load_document_records()?
            .into_iter()
            .filter(|document| {
                workspace_id
                    .is_none_or(|expected| document.workspace_id.as_deref() == Some(expected))
            })
            .filter(|document| kind.is_none_or(|expected| document.kind == expected))
            .collect())
    }

    pub fn get_document(&self, document_id: &str) -> Result<DocumentView> {
        self.repo.get_document(document_id)
    }

    pub fn get_evidence(&self, evidence_id: &str) -> Result<EvidenceView> {
        self.repo.get_evidence(evidence_id)
    }

    pub fn case_workspace_id(&self, case_id: &str) -> Result<Option<String>> {
        self.repo.episode_workspace_id(case_id)
    }

    pub fn runbook_workspace_id(&self, episode_id: &str) -> Result<Option<String>> {
        self.repo.episode_workspace_id(episode_id)
    }

    pub fn thread_workspace_id(&self, session_id: &str) -> Result<Option<String>> {
        self.repo.thread_workspace_id(session_id)
    }

    pub fn run_workspace_id(&self, run_id: &str) -> Result<Option<String>> {
        self.repo.run_workspace_id(run_id)
    }

    pub fn task_workspace_id(&self, task_id: &str) -> Result<Option<String>> {
        self.repo.task_workspace_id(task_id)
    }

    pub fn document_workspace_id(&self, document_id: &str) -> Result<Option<String>> {
        self.repo.document_workspace_id(document_id)
    }

    pub fn evidence_workspace_id(&self, evidence_id: &str) -> Result<Option<String>> {
        self.repo.evidence_workspace_id(evidence_id)
    }

    pub fn source_cursors(&self) -> Result<Vec<SourceCursorRow>> {
        self.repo.load_source_cursors()
    }
}
