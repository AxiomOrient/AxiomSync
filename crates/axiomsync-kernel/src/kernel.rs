use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::domain::{
    ConnectorBatchInput, ConnectorsConfig, DerivationContext, DerivationEnrichment, DerivePlan,
    DoctorReport, EvidenceView, IngestPlan, ProjectionPlan, PurgePlan, RawEventRow, RepairPlan,
    ReplayPlan, RunbookRecord, SearchCommandsResult, SearchEpisodesRequest, SearchEpisodesResult,
    SourceCursorRow, ThreadView, WorkspaceTokenPlan, stable_hash,
};
use crate::error::{AxiomError, Result};
use crate::logic::{
    EpisodeSearchRows, apply_workspace_token_plan, merge_verification_extractions,
    parse_verification_transcript, plan_derivation, plan_derivation_contexts, plan_ingest,
    plan_projection, plan_purge, plan_repair, plan_replay, plan_workspace_token_grant,
    search_command_results, search_episode_results, synthesize_runbook,
};
use crate::ports::{
    McpResourcePort, McpToolPort, SharedAuthStorePort, SharedConnectorConfigPort,
    SharedLlmExtractionPort, SharedRepositoryPort,
};

#[derive(Clone)]
pub struct AxiomSync {
    repo: SharedRepositoryPort,
    auth: SharedAuthStorePort,
    connectors: SharedConnectorConfigPort,
    llm: SharedLlmExtractionPort,
}

impl std::fmt::Debug for AxiomSync {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AxiomSync")
            .field("root", &self.root())
            .finish_non_exhaustive()
    }
}

impl AxiomSync {
    pub fn new(
        repo: SharedRepositoryPort,
        auth: SharedAuthStorePort,
        connectors: SharedConnectorConfigPort,
        llm: SharedLlmExtractionPort,
    ) -> Self {
        Self {
            repo,
            auth,
            connectors,
            llm,
        }
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        self.repo.root()
    }

    #[must_use]
    pub fn db_path(&self) -> &Path {
        self.repo.db_path()
    }

    #[must_use]
    pub fn auth_path(&self) -> PathBuf {
        self.auth.path()
    }

    #[must_use]
    pub fn connectors_path(&self) -> PathBuf {
        self.connectors.path()
    }

    pub fn init(&self) -> Result<Value> {
        let mut report = self.repo.init_report()?;
        self.connectors.ensure_default()?;
        if let Some(object) = report.as_object_mut() {
            object.insert("auth_path".to_string(), serde_json::json!(self.auth_path()));
            object.insert(
                "connectors_path".to_string(),
                serde_json::json!(self.connectors_path()),
            );
        }
        Ok(report)
    }

    pub fn load_connectors_config(&self) -> Result<ConnectorsConfig> {
        self.connectors.load()
    }

    pub fn plan_ingest(&self, input: &ConnectorBatchInput) -> Result<IngestPlan> {
        let existing = self.repo.existing_raw_event_keys()?;
        plan_ingest(&existing, input)
    }

    pub fn apply_ingest(&self, plan: &IngestPlan) -> Result<Value> {
        self.repo.apply_ingest_tx(plan)
    }

    pub fn plan_projection(&self) -> Result<ProjectionPlan> {
        plan_projection(&self.repo.load_raw_events()?)
    }

    pub fn apply_projection(&self, plan: &ProjectionPlan) -> Result<Value> {
        self.repo.apply_projection_tx(plan)
    }

    pub fn plan_replay(&self) -> Result<ReplayPlan> {
        let raw_events = self.repo.load_raw_events()?;
        self.plan_replay_for_raw_events(&raw_events)
    }

    pub fn apply_replay(&self, plan: &ReplayPlan) -> Result<Value> {
        self.repo.apply_replay_tx(plan)
    }

    pub fn derivation_contexts(&self) -> Result<Vec<DerivationContext>> {
        Ok(plan_derivation_contexts(
            &self.repo.load_sessions()?,
            &self.repo.load_turns()?,
            &self.repo.load_items()?,
        ))
    }

    pub fn plan_derivation(&self) -> Result<DerivePlan> {
        let sessions = self.repo.load_sessions()?;
        let turns = self.repo.load_turns()?;
        let items = self.repo.load_items()?;
        let anchors = self.repo.load_evidence_anchors()?;
        let contexts = plan_derivation_contexts(&sessions, &turns, &items);
        let enrichment = self.collect_derivation_inputs(&contexts)?;
        plan_derivation(
            &contexts,
            &turns,
            &items,
            &anchors,
            &enrichment.extractions,
            &enrichment.verifications,
        )
    }

    pub fn apply_derivation(&self, plan: &DerivePlan) -> Result<Value> {
        self.repo.apply_derivation_tx(plan)
    }

    pub fn plan_purge(
        &self,
        connector: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<PurgePlan> {
        let raw_events = self.repo.load_raw_events()?;
        let replay_contexts =
            self.derive_contexts_after_purge(&raw_events, connector, workspace_id)?;
        plan_purge(
            &raw_events,
            connector,
            workspace_id,
            &replay_contexts.extractions,
            &replay_contexts.verifications,
        )
    }

    pub fn apply_purge(&self, plan: &PurgePlan) -> Result<Value> {
        self.repo.apply_purge_tx(plan)
    }

    pub fn doctor(&self) -> Result<DoctorReport> {
        self.repo.doctor_report()
    }

    pub fn plan_repair(&self, input: &ConnectorBatchInput) -> Result<RepairPlan> {
        let ingest = self.plan_ingest(input)?;
        let raw_events = self.repo.load_raw_events()?;
        let replay_contexts = self.derive_contexts_after_repair(&raw_events, &ingest)?;
        plan_repair(
            &raw_events,
            &ingest,
            &replay_contexts.extractions,
            &replay_contexts.verifications,
        )
    }

    pub fn apply_repair(&self, plan: &RepairPlan) -> Result<Value> {
        self.repo.apply_repair_tx(plan)
    }

    pub fn search_episodes(
        &self,
        request: SearchEpisodesRequest,
    ) -> Result<Vec<SearchEpisodesResult>> {
        let fts_rows = self
            .repo
            .load_episode_search_fts_rows(&request.query, request.limit)?;
        let evidence_rows = self.repo.load_episode_evidence_search_rows()?;
        let episodes = self.repo.load_episodes()?;
        let insights = self.repo.load_insights()?;
        let verifications = self.repo.load_verifications()?;
        let connectors = self.repo.load_episode_connectors()?;
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
        let mut runbooks = Vec::new();
        for episode in episodes {
            runbooks.push(synthesize_runbook(
                &episode,
                &insights
                    .iter()
                    .filter(|row| row.episode_id == episode.stable_id)
                    .cloned()
                    .collect::<Vec<_>>(),
                &insight_anchors
                    .iter()
                    .filter(|row| {
                        insights.iter().any(|insight| {
                            insight.episode_id == episode.stable_id
                                && insight.stable_id == row.insight_id
                        })
                    })
                    .cloned()
                    .collect::<Vec<_>>(),
                &verifications
                    .iter()
                    .filter(|row| row.episode_id == episode.stable_id)
                    .cloned()
                    .collect::<Vec<_>>(),
            )?);
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

    fn collect_derivation_inputs(
        &self,
        contexts: &[DerivationContext],
    ) -> Result<DerivationEnrichment> {
        let mut enrichment = DerivationEnrichment::default();
        for context in contexts {
            match self.llm.extract_episode(&context.transcript) {
                Ok(extraction) => {
                    enrichment
                        .extractions
                        .insert(context.episode_id.clone(), extraction);
                }
                Err(AxiomError::LlmUnavailable(_)) => {}
                Err(error) => return Err(error),
            }
            let mut verification_candidates = parse_verification_transcript(&context.transcript);
            match self.llm.synthesize_verifications(&context.transcript) {
                Ok(extractions) => verification_candidates.extend(extractions),
                Err(AxiomError::LlmUnavailable(_)) => {}
                Err(error) => return Err(error),
            }
            enrichment.verifications.insert(
                context.episode_id.clone(),
                merge_verification_extractions(&verification_candidates),
            );
        }
        Ok(enrichment)
    }

    pub fn plan_workspace_token_grant(
        &self,
        canonical_root: &str,
        token: &str,
    ) -> Result<WorkspaceTokenPlan> {
        plan_workspace_token_grant(canonical_root, token)
    }

    pub fn apply_workspace_token_grant(&self, plan: &WorkspaceTokenPlan) -> Result<Value> {
        let snapshot = self.auth.read()?;
        let next = apply_workspace_token_plan(&snapshot, plan);
        self.auth.write(&next)?;
        Ok(serde_json::json!({
            "workspace_id": plan.workspace_id,
        }))
    }

    pub fn authorize_workspace(
        &self,
        token: &str,
        workspace_id: Option<&str>,
    ) -> Result<Option<String>> {
        let snapshot = self.auth.read()?;
        let token_sha = stable_hash(&[token]);
        let mut matched = snapshot
            .grants
            .iter()
            .filter(|entry| entry.token_sha256 == token_sha)
            .map(|entry| entry.workspace_id.clone())
            .collect::<Vec<_>>();
        matched.sort();
        matched.dedup();
        if matched.is_empty() {
            return Err(AxiomError::PermissionDenied(
                "invalid bearer token".to_string(),
            ));
        }
        if let Some(target) = workspace_id {
            if matched.iter().any(|candidate| candidate == target) {
                return Ok(Some(target.to_string()));
            }
            return Err(AxiomError::PermissionDenied(
                "token does not grant access to requested workspace".to_string(),
            ));
        }
        Ok(matched.into_iter().next())
    }

    pub fn mcp_resources(&self) -> Result<Value> {
        Ok(serde_json::json!([
            {"uri": "axiom://episode/{id}", "name": "Episode runbook"},
            {"uri": "axiom://thread/{id}", "name": "Conversation thread"},
            {"uri": "axiom://evidence/{id}", "name": "Evidence anchor"}
        ]))
    }

    pub fn mcp_roots(&self, bound_workspace_id: Option<&str>) -> Result<Value> {
        let mut roots = self
            .repo
            .load_workspaces()?
            .into_iter()
            .filter(|workspace| bound_workspace_id.is_none_or(|bound| workspace.stable_id == bound))
            .map(|workspace| {
                serde_json::json!({
                    "uri": workspace.canonical_root,
                    "name": workspace.canonical_root,
                    "workspace_id": workspace.stable_id,
                })
            })
            .collect::<Vec<_>>();
        roots.sort_by(|left, right| {
            left.get("workspace_id")
                .and_then(Value::as_str)
                .cmp(&right.get("workspace_id").and_then(Value::as_str))
        });
        Ok(Value::Array(roots))
    }

    fn plan_replay_for_raw_events(&self, raw_events: &[RawEventRow]) -> Result<ReplayPlan> {
        let projection = plan_projection(raw_events)?;
        let contexts = plan_derivation_contexts(
            &projection.conv_sessions,
            &projection.conv_turns,
            &projection.conv_items,
        );
        let enrichment = self.collect_derivation_inputs(&contexts)?;
        plan_replay(
            raw_events,
            &enrichment.extractions,
            &enrichment.verifications,
        )
    }

    fn derive_contexts_after_purge(
        &self,
        raw_events: &[RawEventRow],
        connector: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<DerivationEnrichment> {
        let surviving_events = raw_events.iter().try_fold(Vec::new(), |mut acc, event| {
            if !crate::logic::raw_event_matches_purge(event, connector, workspace_id)? {
                acc.push(event.clone());
            }
            Ok::<_, AxiomError>(acc)
        })?;
        let projection = plan_projection(&surviving_events)?;
        let contexts = plan_derivation_contexts(
            &projection.conv_sessions,
            &projection.conv_turns,
            &projection.conv_items,
        );
        self.collect_derivation_inputs(&contexts)
    }

    fn derive_contexts_after_repair(
        &self,
        raw_events: &[RawEventRow],
        ingest: &IngestPlan,
    ) -> Result<DerivationEnrichment> {
        let mut combined = raw_events.to_vec();
        combined.extend(ingest.adds.iter().map(|event| event.row.clone()));
        let projection = plan_projection(&combined)?;
        let contexts = plan_derivation_contexts(
            &projection.conv_sessions,
            &projection.conv_turns,
            &projection.conv_items,
        );
        self.collect_derivation_inputs(&contexts)
    }
}

impl McpResourcePort for AxiomSync {
    fn mcp_resources(&self) -> Result<Value> {
        AxiomSync::mcp_resources(self)
    }

    fn mcp_roots(&self, bound_workspace_id: Option<&str>) -> Result<Value> {
        AxiomSync::mcp_roots(self, bound_workspace_id)
    }

    fn read_mcp_resource(&self, uri: &str) -> Result<Value> {
        match parse_resource_uri(uri)? {
            ResourceUri::Episode(id) => {
                serde_json::to_value(self.get_runbook(&id)?).map_err(Into::into)
            }
            ResourceUri::Thread(id) => {
                serde_json::to_value(self.get_thread(&id)?).map_err(Into::into)
            }
            ResourceUri::Evidence(id) => {
                serde_json::to_value(self.get_evidence(&id)?).map_err(Into::into)
            }
        }
    }

    fn resource_workspace_requirement(&self, uri: &str) -> Result<Option<String>> {
        match parse_resource_uri(uri)? {
            ResourceUri::Episode(id) => self.runbook_workspace_id(&id),
            ResourceUri::Thread(id) => self.thread_workspace_id(&id),
            ResourceUri::Evidence(id) => self.evidence_workspace_id(&id),
        }
    }
}

impl McpToolPort for AxiomSync {
    fn mcp_tools(&self) -> Result<Value> {
        Ok(serde_json::json!([
            {"name": "search_episodes"},
            {"name": "get_runbook"},
            {"name": "get_thread"},
            {"name": "get_evidence"},
            {"name": "search_commands"}
        ]))
    }

    fn call_mcp_tool(
        &self,
        name: &str,
        arguments: &Value,
        bound_workspace_id: Option<&str>,
    ) -> Result<Value> {
        match name {
            "search_episodes" => serde_json::to_value(
                self.search_episodes(crate::domain::SearchEpisodesRequest {
                    query: required_arg(arguments, "query")?.to_string(),
                    limit: arguments.get("limit").and_then(Value::as_u64).unwrap_or(10) as usize,
                    filter: crate::domain::SearchEpisodesFilter {
                        connector: arguments
                            .get("filters")
                            .and_then(|filters| filters.get("connector"))
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        workspace_id: self
                            .tool_workspace_requirement(name, arguments)?
                            .or_else(|| bound_workspace_id.map(ToOwned::to_owned)),
                        status: arguments
                            .get("filters")
                            .and_then(|filters| filters.get("status"))
                            .and_then(Value::as_str)
                            .map(crate::domain::EpisodeStatus::parse)
                            .transpose()?,
                    },
                })?,
            )
            .map_err(Into::into),
            "get_runbook" => {
                serde_json::to_value(self.get_runbook(required_arg(arguments, "episode_id")?)?)
                    .map_err(Into::into)
            }
            "get_thread" => {
                serde_json::to_value(self.get_thread(required_arg(arguments, "thread_id")?)?)
                    .map_err(Into::into)
            }
            "get_evidence" => {
                serde_json::to_value(self.get_evidence(required_arg(arguments, "evidence_id")?)?)
                    .map_err(Into::into)
            }
            "search_commands" => {
                if let Some(workspace_id) = arguments.get("workspace_id").and_then(Value::as_str) {
                    serde_json::to_value(self.search_commands_in_workspace(
                        required_arg(arguments, "query")?,
                        arguments.get("limit").and_then(Value::as_u64).unwrap_or(5) as usize,
                        workspace_id,
                    )?)
                    .map_err(Into::into)
                } else if let Some(bound) = bound_workspace_id {
                    serde_json::to_value(self.search_commands_in_workspace(
                        required_arg(arguments, "query")?,
                        arguments.get("limit").and_then(Value::as_u64).unwrap_or(5) as usize,
                        bound,
                    )?)
                    .map_err(Into::into)
                } else {
                    serde_json::to_value(self.search_commands(
                        required_arg(arguments, "query")?,
                        arguments.get("limit").and_then(Value::as_u64).unwrap_or(5) as usize,
                    )?)
                    .map_err(Into::into)
                }
            }
            other => Err(AxiomError::Validation(format!("unknown tool {other}"))),
        }
    }

    fn tool_workspace_requirement(&self, name: &str, arguments: &Value) -> Result<Option<String>> {
        match name {
            "get_runbook" => self.runbook_workspace_id(required_arg(arguments, "episode_id")?),
            "get_thread" => self.thread_workspace_id(required_arg(arguments, "thread_id")?),
            "get_evidence" => self.evidence_workspace_id(required_arg(arguments, "evidence_id")?),
            "search_episodes" => Ok(arguments
                .get("filters")
                .and_then(|filters| filters.get("workspace_id"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)),
            "search_commands" => Ok(arguments
                .get("workspace_id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)),
            _ => Ok(None),
        }
    }
}

enum ResourceUri {
    Episode(String),
    Thread(String),
    Evidence(String),
}

fn parse_resource_uri(uri: &str) -> Result<ResourceUri> {
    if let Some(id) = uri.strip_prefix("axiom://episode/") {
        return Ok(ResourceUri::Episode(id.to_string()));
    }
    if let Some(id) = uri.strip_prefix("axiom://thread/") {
        return Ok(ResourceUri::Thread(id.to_string()));
    }
    if let Some(id) = uri.strip_prefix("axiom://evidence/") {
        return Ok(ResourceUri::Evidence(id.to_string()));
    }
    Err(AxiomError::Validation(format!(
        "invalid resource uri {uri}"
    )))
}

fn required_arg<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| AxiomError::Validation(format!("missing argument {key}")))
}
