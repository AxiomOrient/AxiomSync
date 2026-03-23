use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::compat::{
    case_from_episode, document_view_from_artifact, episode_view, session_view, session_workspace,
    task_view,
};
use crate::derive::plan_derivation;
use crate::domain::{
    AdminTokenPlan, AnchorView, AppendRawEventsRequest, ArtifactView, AuthSnapshot, CaseRecord,
    DoctorReport, DocumentView, EntryRow, EpisodeView, EvidenceBundle, EvidencePreview,
    EvidenceView, IngestPlan, ReplayPlan, RunView, RunbookRecord, SearchClaimsRequest,
    SearchDocsRequest, SearchEntriesRequest, SearchEpisodesRequest, SearchHit,
    SearchInsightsRequest, SearchProceduresRequest, SessionRow, SessionView,
    SourceCursorUpsertPlan, TaskView, ThreadView, UpsertSourceCursorRequest, VerificationSummary,
    WorkspaceTokenPlan,
};
use crate::error::{AxiomError, Result};
use crate::ingest::{plan_append_raw_events, plan_source_cursor_upsert};
use crate::ports::{
    SharedAuthStorePort, SharedLlmExtractionPort, SharedRepositoryPort, filter_hits,
};
use crate::projection::plan_projection;

mod auth;

#[derive(Clone)]
pub struct AxiomSync {
    repo: SharedRepositoryPort,
    auth: SharedAuthStorePort,
    #[allow(dead_code)]
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
        llm: SharedLlmExtractionPort,
    ) -> Self {
        Self { repo, auth, llm }
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

    pub fn init(&self) -> Result<Value> {
        let mut report = self.repo.init_report()?;
        if let Some(object) = report.as_object_mut() {
            object.insert("auth_path".to_string(), json!(self.auth.path()));
        }
        Ok(report)
    }

    pub fn doctor_report(&self) -> Result<DoctorReport> {
        self.repo.doctor_report()
    }

    pub fn plan_append_raw_events(&self, request: AppendRawEventsRequest) -> Result<IngestPlan> {
        plan_append_raw_events(&request, &self.repo.existing_dedupe_keys()?)
    }

    pub fn apply_ingest_plan(&self, plan: &IngestPlan) -> Result<Value> {
        self.repo.apply_ingest(plan)
    }

    pub fn plan_source_cursor_upsert(
        &self,
        request: UpsertSourceCursorRequest,
    ) -> Result<SourceCursorUpsertPlan> {
        plan_source_cursor_upsert(&request)
    }

    pub fn apply_source_cursor_plan(&self, plan: &SourceCursorUpsertPlan) -> Result<Value> {
        self.repo.apply_source_cursor_upsert(plan)
    }

    pub fn build_replay_plan(&self) -> Result<ReplayPlan> {
        let projection = self.build_projection_plan()?;
        let derivation = self.build_derivation_plan_for_projection(&projection)?;
        Ok(ReplayPlan {
            projection,
            derivation,
        })
    }

    pub fn build_projection_plan(&self) -> Result<crate::domain::ProjectionPlan> {
        let receipts = self.repo.load_receipts()?;
        plan_projection(&receipts)
    }

    pub fn apply_projection_plan(&self, plan: &crate::domain::ProjectionPlan) -> Result<Value> {
        self.repo.replace_projection(plan)
    }

    pub fn build_derivation_plan(&self) -> Result<crate::domain::DerivePlan> {
        let projection = crate::domain::ProjectionPlan {
            sessions: self.repo.load_sessions()?,
            actors: Vec::new(),
            entries: self.repo.load_entries()?,
            artifacts: self.repo.load_artifacts()?,
            anchors: self.repo.load_anchors()?,
        };
        self.build_derivation_plan_for_projection(&projection)
    }

    pub fn apply_derivation_plan(&self, plan: &crate::domain::DerivePlan) -> Result<Value> {
        self.repo.replace_derivation(plan)
    }

    fn build_derivation_plan_for_projection(
        &self,
        projection: &crate::domain::ProjectionPlan,
    ) -> Result<crate::domain::DerivePlan> {
        plan_derivation(
            &projection.sessions,
            &projection.entries,
            &projection.anchors,
        )
    }

    pub fn apply_replay(&self, plan: &ReplayPlan) -> Result<Value> {
        self.repo.apply_replay(plan)
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionRow>> {
        self.repo.load_sessions()
    }

    pub fn get_session(&self, session_id: &str) -> Result<SessionView> {
        let sessions = self.repo.load_sessions()?;
        let entries = self.repo.load_entries()?;
        let artifacts = self.repo.load_artifacts()?;
        let anchors = self.repo.load_anchors()?;
        let session = sessions
            .iter()
            .find(|session| session.session_id == session_id)
            .ok_or_else(|| AxiomError::NotFound(format!("session {session_id}")))?;
        Ok(session_view(session, &entries, &artifacts, &anchors))
    }

    pub fn get_entry(&self, entry_id: &str) -> Result<EntryRow> {
        self.repo
            .load_entries()?
            .into_iter()
            .find(|entry| entry.entry_id == entry_id)
            .ok_or_else(|| AxiomError::NotFound(format!("entry {entry_id}")))
    }

    pub fn get_artifact(&self, artifact_id: &str) -> Result<ArtifactView> {
        let sessions = self.repo.load_sessions()?;
        let entries = self.repo.load_entries()?;
        let artifacts = self.repo.load_artifacts()?;
        let artifact = artifacts
            .iter()
            .find(|artifact| artifact.artifact_id == artifact_id)
            .ok_or_else(|| AxiomError::NotFound(format!("artifact {artifact_id}")))?;
        Ok(document_view_from_artifact(artifact, &sessions, &entries))
    }

    pub fn get_anchor(&self, anchor_id: &str) -> Result<AnchorView> {
        let sessions = self.repo.load_sessions()?;
        let entries = self.repo.load_entries()?;
        let artifacts = self.repo.load_artifacts()?;
        let anchors = self.repo.load_anchors()?;
        let anchor = anchors
            .iter()
            .find(|anchor| anchor.anchor_id == anchor_id)
            .ok_or_else(|| AxiomError::NotFound(format!("anchor {anchor_id}")))?;
        let entry = anchor
            .entry_id
            .as_deref()
            .and_then(|entry_id| entries.iter().find(|entry| entry.entry_id == entry_id))
            .cloned();
        let artifact = anchor
            .artifact_id
            .as_deref()
            .and_then(|artifact_id| {
                artifacts
                    .iter()
                    .find(|artifact| artifact.artifact_id == artifact_id)
            })
            .cloned();
        let session = entry
            .as_ref()
            .and_then(|entry| {
                sessions
                    .iter()
                    .find(|session| session.session_id == entry.session_id)
            })
            .cloned()
            .or_else(|| {
                artifact.as_ref().and_then(|artifact| {
                    sessions
                        .iter()
                        .find(|session| session.session_id == artifact.session_id)
                        .cloned()
                })
            });
        Ok(AnchorView {
            anchor: anchor.clone(),
            entry,
            artifact,
            session,
        })
    }

    pub fn get_episode(&self, episode_id: &str) -> Result<EpisodeView> {
        let episodes = self.repo.load_episodes()?;
        let insights = self.repo.load_insights()?;
        let verifications = self.repo.load_verifications()?;
        let claims = self.repo.load_claims()?;
        let procedures = self.repo.load_procedures()?;
        let episode = episodes
            .iter()
            .find(|episode| episode.episode_id == episode_id)
            .ok_or_else(|| AxiomError::NotFound(format!("episode {episode_id}")))?;
        Ok(episode_view(
            episode,
            &insights,
            &verifications,
            &claims,
            &procedures,
        ))
    }

    pub fn get_claim(&self, claim_id: &str) -> Result<crate::domain::ClaimRow> {
        self.repo
            .load_claims()?
            .into_iter()
            .find(|claim| claim.claim_id == claim_id)
            .ok_or_else(|| AxiomError::NotFound(format!("claim {claim_id}")))
    }

    pub fn get_procedure(&self, procedure_id: &str) -> Result<crate::domain::ProcedureRow> {
        self.repo
            .load_procedures()?
            .into_iter()
            .find(|procedure| procedure.procedure_id == procedure_id)
            .ok_or_else(|| AxiomError::NotFound(format!("procedure {procedure_id}")))
    }

    pub fn search_entries(&self, request: SearchEntriesRequest) -> Result<Vec<SearchHit>> {
        Ok(filter_hits(
            crate::query::search_entries(
                &self.repo.load_sessions()?,
                &self.repo.load_entries()?,
                &request,
            ),
            effective_limit(request.limit),
        ))
    }

    pub fn search_episodes(&self, request: SearchEpisodesRequest) -> Result<Vec<SearchHit>> {
        Ok(filter_hits(
            crate::query::search_episodes(
                &self.repo.load_sessions()?,
                &self.repo.load_episodes()?,
                &request,
            ),
            effective_limit(request.limit),
        ))
    }

    pub fn search_claims(&self, request: SearchClaimsRequest) -> Result<Vec<SearchHit>> {
        Ok(filter_hits(
            crate::query::search_claims(
                &self.repo.load_sessions()?,
                &self.repo.load_episodes()?,
                &self.repo.load_claims()?,
                &request,
            ),
            effective_limit(request.limit),
        ))
    }

    pub fn search_insights(&self, request: SearchInsightsRequest) -> Result<Vec<SearchHit>> {
        Ok(filter_hits(
            crate::query::search_insights(
                &self.repo.load_sessions()?,
                &self.repo.load_episodes()?,
                &self.repo.load_insights()?,
                &self.repo.load_insight_anchors()?,
                &self.repo.load_anchors()?,
                &request,
            ),
            effective_limit(request.limit),
        ))
    }

    pub fn search_procedures(&self, request: SearchProceduresRequest) -> Result<Vec<SearchHit>> {
        Ok(filter_hits(
            crate::query::search_procedures(
                &self.repo.load_sessions()?,
                &self.repo.load_procedures()?,
                &request,
            ),
            effective_limit(request.limit),
        ))
    }

    pub fn search_docs(&self, request: SearchDocsRequest) -> Result<Vec<SearchHit>> {
        let hits = crate::query::search_docs(&self.repo.load_search_docs()?, &request);
        Ok(filter_hits(
            hits.into_iter()
                .map(|hit| self.enrich_search_hit(hit))
                .collect::<Result<Vec<_>>>()?,
            effective_limit(request.limit),
        ))
    }

    pub fn list_cases(&self) -> Result<Vec<CaseRecord>> {
        let sessions = self.repo.load_sessions()?;
        let episodes = self.repo.load_episodes()?;
        let insights = self.repo.load_insights()?;
        let verifications = self.repo.load_verifications()?;
        let claims = self.repo.load_claims()?;
        let procedures = self.repo.load_procedures()?;
        Ok(episodes
            .iter()
            .map(|episode| {
                case_from_episode(
                    episode,
                    &sessions,
                    &insights,
                    &verifications,
                    &claims,
                    &procedures,
                )
            })
            .collect())
    }

    pub fn get_case(&self, case_id: &str) -> Result<CaseRecord> {
        let sessions = self.repo.load_sessions()?;
        let insights = self.repo.load_insights()?;
        let verifications = self.repo.load_verifications()?;
        let claims = self.repo.load_claims()?;
        let procedures = self.repo.load_procedures()?;
        let episode = self
            .repo
            .load_episodes()?
            .into_iter()
            .find(|episode| episode.episode_id == case_id)
            .ok_or_else(|| AxiomError::NotFound(format!("case {case_id}")))?;
        Ok(case_from_episode(
            &episode,
            &sessions,
            &insights,
            &verifications,
            &claims,
            &procedures,
        ))
    }

    pub fn get_runbook(&self, episode_id: &str) -> Result<RunbookRecord> {
        let case = self.get_case(episode_id)?;
        Ok(RunbookRecord {
            episode_id: case.case_id,
            workspace_root: case.workspace_root,
            problem: case.problem,
            root_cause: case.root_cause,
            fix: case.resolution,
            commands: case.commands,
            verification: case.verification,
            evidence: case.evidence,
        })
    }

    pub fn find_fix(&self, request: SearchInsightsRequest) -> Result<Option<EvidenceBundle>> {
        self.find_first_bundle(&request, Some("fix"))
    }

    pub fn find_decision(&self, request: SearchInsightsRequest) -> Result<Option<EvidenceBundle>> {
        self.find_first_bundle(&request, Some("decision"))
    }

    pub fn find_runbook(&self, request: SearchProceduresRequest) -> Result<Option<EvidenceBundle>> {
        let hit = self.search_procedures(request)?.into_iter().next();
        hit.map(|hit| self.get_evidence_bundle("procedure", &hit.id))
            .transpose()
    }

    pub fn get_evidence_bundle(
        &self,
        subject_kind: &str,
        subject_id: &str,
    ) -> Result<EvidenceBundle> {
        let anchors = self.repo.load_anchors()?;
        let insight_anchors = self.repo.load_insight_anchors()?;
        let claim_evidence = self.repo.load_claim_evidence()?;
        let procedure_evidence = self.repo.load_procedure_evidence()?;
        let verifications = self.repo.load_verifications()?;
        match subject_kind {
            "insight" => {
                let insight = self
                    .repo
                    .load_insights()?
                    .into_iter()
                    .find(|insight| insight.insight_id == subject_id)
                    .ok_or_else(|| AxiomError::NotFound(format!("insight {subject_id}")))?;
                let evidence = insight_anchors
                    .into_iter()
                    .filter(|row| row.insight_id == insight.insight_id)
                    .filter_map(|row| {
                        anchors
                            .iter()
                            .find(|anchor| anchor.anchor_id == row.anchor_id)
                    })
                    .map(anchor_preview)
                    .collect::<Vec<_>>();
                Ok(EvidenceBundle {
                    subject_kind: "insight".to_string(),
                    subject_id: insight.insight_id,
                    title: Some(insight.insight_kind),
                    summary: Some(insight.statement),
                    verification: self.collect_verification_summary(
                        &verifications,
                        "insight",
                        subject_id,
                    ),
                    evidence,
                })
            }
            "procedure" => {
                let procedure = self
                    .repo
                    .load_procedures()?
                    .into_iter()
                    .find(|procedure| procedure.procedure_id == subject_id)
                    .ok_or_else(|| AxiomError::NotFound(format!("procedure {subject_id}")))?;
                let evidence = procedure_evidence
                    .into_iter()
                    .filter(|row| row.procedure_id == procedure.procedure_id)
                    .filter_map(|row| {
                        anchors
                            .iter()
                            .find(|anchor| anchor.anchor_id == row.anchor_id)
                    })
                    .map(anchor_preview)
                    .collect::<Vec<_>>();
                Ok(EvidenceBundle {
                    subject_kind: "procedure".to_string(),
                    subject_id: procedure.procedure_id,
                    title: Some(procedure.title),
                    summary: procedure.goal,
                    verification: self.collect_verification_summary(
                        &verifications,
                        "procedure",
                        subject_id,
                    ),
                    evidence,
                })
            }
            "episode" => {
                let episode = self
                    .repo
                    .load_episodes()?
                    .into_iter()
                    .find(|episode| episode.episode_id == subject_id)
                    .ok_or_else(|| AxiomError::NotFound(format!("episode {subject_id}")))?;
                let claim_rows = self
                    .repo
                    .load_claims()?
                    .into_iter()
                    .filter(|claim| claim.episode_id.as_deref() == Some(subject_id))
                    .collect::<Vec<_>>();
                let evidence = claim_evidence
                    .into_iter()
                    .filter(|row| {
                        claim_rows
                            .iter()
                            .any(|claim| claim.claim_id == row.claim_id)
                    })
                    .filter_map(|row| {
                        anchors
                            .iter()
                            .find(|anchor| anchor.anchor_id == row.anchor_id)
                    })
                    .map(anchor_preview)
                    .collect::<Vec<_>>();
                Ok(EvidenceBundle {
                    subject_kind: "episode".to_string(),
                    subject_id: episode.episode_id,
                    title: Some(episode.episode_kind),
                    summary: Some(episode.summary),
                    verification: self.collect_verification_summary(
                        &verifications,
                        "episode",
                        subject_id,
                    ),
                    evidence,
                })
            }
            other => Err(AxiomError::Validation(format!(
                "unsupported evidence bundle subject_kind {other}"
            ))),
        }
    }

    pub fn get_thread(&self, session_id: &str) -> Result<ThreadView> {
        self.get_session(session_id)
    }

    pub fn list_runs(&self, workspace_root: Option<&str>) -> Result<Vec<SessionRow>> {
        Ok(self
            .repo
            .load_sessions()?
            .into_iter()
            .filter(|session| session.session_kind == "run")
            .filter(|session| {
                workspace_root
                    .is_none_or(|expected| session.workspace_root.as_deref() == Some(expected))
            })
            .collect())
    }

    pub fn get_run(&self, run_id: &str) -> Result<RunView> {
        self.get_session(run_id)
    }

    pub fn get_task(&self, task_id: &str) -> Result<TaskView> {
        task_view(self.get_session(task_id)?, task_id)
    }

    pub fn list_documents(
        &self,
        workspace_root: Option<&str>,
        kind: Option<&str>,
    ) -> Result<Vec<ArtifactView>> {
        let sessions = self.repo.load_sessions()?;
        let entries = self.repo.load_entries()?;
        Ok(self
            .repo
            .load_artifacts()?
            .into_iter()
            .filter(|artifact| {
                workspace_root.is_none_or(|expected| {
                    sessions
                        .iter()
                        .find(|session| session.session_id == artifact.session_id)
                        .and_then(|session| session.workspace_root.as_deref())
                        == Some(expected)
                })
            })
            .filter(|artifact| kind.is_none_or(|expected| artifact.artifact_kind == expected))
            .map(|artifact| document_view_from_artifact(&artifact, &sessions, &entries))
            .collect())
    }

    pub fn get_document(&self, document_id: &str) -> Result<DocumentView> {
        self.get_artifact(document_id)
    }

    pub fn get_evidence(&self, evidence_id: &str) -> Result<EvidenceView> {
        self.get_anchor(evidence_id)
    }

    pub fn episode_workspace_id(&self, episode_id: &str) -> Result<Option<String>> {
        let episode = self
            .repo
            .load_episodes()?
            .into_iter()
            .find(|episode| episode.episode_id == episode_id)
            .ok_or_else(|| AxiomError::NotFound(format!("episode {episode_id}")))?;
        let sessions = self.repo.load_sessions()?;
        Ok(episode
            .session_id
            .as_deref()
            .and_then(|session_id| session_workspace(&sessions, session_id)))
    }

    pub fn session_workspace_id(&self, session_id: &str) -> Result<Option<String>> {
        Ok(session_workspace(&self.repo.load_sessions()?, session_id))
    }

    pub fn artifact_workspace_id(&self, artifact_id: &str) -> Result<Option<String>> {
        let sessions = self.repo.load_sessions()?;
        let artifact = self
            .repo
            .load_artifacts()?
            .into_iter()
            .find(|artifact| artifact.artifact_id == artifact_id)
            .ok_or_else(|| AxiomError::NotFound(format!("artifact {artifact_id}")))?;
        Ok(session_workspace(&sessions, &artifact.session_id))
    }

    pub fn anchor_workspace_id(&self, anchor_id: &str) -> Result<Option<String>> {
        Ok(self.get_anchor(anchor_id)?.session.and_then(|session| {
            session
                .workspace_root
                .map(|root| crate::domain::workspace_stable_id(&root))
        }))
    }

    pub fn insight_workspace_id(&self, insight_id: &str) -> Result<Option<String>> {
        let insight = self
            .repo
            .load_insights()?
            .into_iter()
            .find(|insight| insight.insight_id == insight_id)
            .ok_or_else(|| AxiomError::NotFound(format!("insight {insight_id}")))?;
        match insight.episode_id {
            Some(episode_id) => self.episode_workspace_id(&episode_id),
            None => Ok(None),
        }
    }

    pub fn procedure_workspace_id(&self, procedure_id: &str) -> Result<Option<String>> {
        let procedure_anchor = self
            .repo
            .load_procedure_evidence()?
            .into_iter()
            .find(|row| row.procedure_id == procedure_id)
            .ok_or_else(|| AxiomError::NotFound(format!("procedure {procedure_id}")))?;
        self.anchor_workspace_id(&procedure_anchor.anchor_id)
    }

    pub fn task_workspace_id(&self, task_id: &str) -> Result<Option<String>> {
        let task = self.get_task(task_id)?;
        Ok(task
            .session
            .workspace_root
            .as_deref()
            .map(crate::domain::workspace_stable_id))
    }

    pub fn plan_workspace_token_grant(
        &self,
        canonical_root: &str,
        token: &str,
    ) -> Result<WorkspaceTokenPlan> {
        crate::logic::auth::plan_workspace_token_grant(canonical_root, token)
    }

    pub fn apply_workspace_token_grant(&self, plan: &WorkspaceTokenPlan) -> Result<Value> {
        let snapshot = self.auth.read()?;
        let next = crate::logic::auth::apply_workspace_token_plan(&snapshot, plan);
        self.auth.write(&next)?;
        Ok(json!({ "workspace_id": plan.workspace_id }))
    }

    pub fn plan_admin_token_grant(&self, token: &str) -> Result<AdminTokenPlan> {
        crate::logic::auth::plan_admin_token_grant(token)
    }

    pub fn apply_admin_token_grant(&self, plan: &AdminTokenPlan) -> Result<Value> {
        let snapshot = self.auth.read()?;
        let next = crate::logic::auth::apply_admin_token_plan(&snapshot, plan);
        let admin_tokens = next.admin_tokens.len();
        self.auth.write(&next)?;
        Ok(json!({ "admin_tokens": admin_tokens }))
    }

    pub fn authorize_workspace(
        &self,
        token: &str,
        workspace_id: Option<&str>,
    ) -> Result<Option<String>> {
        let snapshot = self.auth.read()?;
        let token_sha256 = crate::domain::stable_hash(&["workspace-token", token]);
        let grant = snapshot
            .grants
            .iter()
            .find(|grant| grant.token_sha256 == token_sha256)
            .ok_or_else(|| {
                AxiomError::PermissionDenied("token does not grant workspace access".to_string())
            })?;
        if let Some(expected) = workspace_id
            && expected != grant.workspace_id
        {
            return Err(AxiomError::PermissionDenied(
                "token does not grant access to requested workspace".to_string(),
            ));
        }
        Ok(Some(grant.workspace_id.clone()))
    }

    pub fn authorize_admin(&self, token: &str) -> Result<()> {
        let snapshot: AuthSnapshot = self.auth.read()?;
        let token_sha256 = crate::domain::stable_hash(&["admin-token", token]);
        if snapshot.admin_tokens.contains(&token_sha256) {
            Ok(())
        } else {
            Err(AxiomError::PermissionDenied(
                "token does not grant admin access".to_string(),
            ))
        }
    }

    pub fn pending_counts(&self) -> Result<(usize, usize, usize)> {
        self.repo.pending_counts()
    }

    fn find_first_bundle(
        &self,
        request: &SearchInsightsRequest,
        kind: Option<&str>,
    ) -> Result<Option<EvidenceBundle>> {
        let hit = self
            .search_insights(request.clone())?
            .into_iter()
            .find(|hit| kind.is_none_or(|expected| hit.title == expected));
        hit.map(|hit| self.get_evidence_bundle("insight", &hit.id))
            .transpose()
    }

    fn collect_verification_summary(
        &self,
        verifications: &[crate::domain::VerificationRow],
        subject_kind: &str,
        subject_id: &str,
    ) -> Vec<VerificationSummary> {
        verifications
            .iter()
            .filter(|verification| {
                verification.subject_kind == subject_kind && verification.subject_id == subject_id
            })
            .map(|verification| VerificationSummary {
                status: verification.status.clone(),
                method: verification.method.clone(),
            })
            .collect()
    }

    fn enrich_search_hit(&self, mut hit: SearchHit) -> Result<SearchHit> {
        if matches!(hit.kind.as_str(), "episode" | "insight" | "procedure") {
            hit.evidence = self.get_evidence_bundle(&hit.kind, &hit.id)?.evidence;
        }
        Ok(hit)
    }
}

fn effective_limit(limit: usize) -> usize {
    if limit == 0 { 10 } else { limit }
}

fn anchor_preview(anchor: &crate::domain::AnchorRow) -> EvidencePreview {
    EvidencePreview {
        anchor_id: anchor.anchor_id.clone(),
        preview_text: anchor.preview_text.clone(),
    }
}
