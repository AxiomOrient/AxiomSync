use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::derive::plan_derivation;
use crate::domain::{
    AdminTokenPlan, AnchorView, AppendRawEventsRequest, ArtifactView, AuthSnapshot, CaseRecord,
    DoctorReport, DocumentView, EntryBundle, EntryRow, EvidenceView, IngestPlan, ReplayPlan,
    RunView, SearchCasesRequest, SearchHit, SessionRow, SessionView, SourceCursorUpsertPlan,
    TaskView, ThreadView, UpsertSourceCursorRequest, VerificationSummary, WorkspaceTokenPlan,
    workspace_stable_id,
};
use crate::error::{AxiomError, Result};
use crate::ingest::{plan_append_raw_events, plan_source_cursor_upsert};
use crate::ports::{SharedAuthStorePort, SharedRepositoryPort, filter_hits};
use crate::projection::plan_projection;

#[derive(Clone)]
pub struct AxiomSync {
    repo: SharedRepositoryPort,
    auth: SharedAuthStorePort,
}

impl std::fmt::Debug for AxiomSync {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AxiomSync")
            .field("root", &self.root())
            .finish_non_exhaustive()
    }
}

impl AxiomSync {
    pub fn new(repo: SharedRepositoryPort, auth: SharedAuthStorePort) -> Self {
        Self { repo, auth }
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
            artifacts: Vec::new(),
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

    fn get_session(&self, session_id: &str) -> Result<SessionView> {
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

    fn get_artifact(&self, artifact_id: &str) -> Result<ArtifactView> {
        let sessions = self.repo.load_sessions()?;
        let entries = self.repo.load_entries()?;
        let artifacts = self.repo.load_artifacts()?;
        let artifact = artifacts
            .iter()
            .find(|artifact| artifact.artifact_id == artifact_id)
            .ok_or_else(|| AxiomError::NotFound(format!("artifact {artifact_id}")))?;
        Ok(document_view_from_artifact(artifact, &sessions, &entries))
    }

    fn get_anchor(&self, anchor_id: &str) -> Result<AnchorView> {
        let sessions = self.repo.load_sessions()?;
        let entries = self.repo.load_entries()?;
        let artifacts = self.repo.load_artifacts()?;
        let anchors = self.repo.load_anchors()?;
        let anchor = anchors
            .iter()
            .find(|anchor| anchor.anchor_id == anchor_id)
            .ok_or_else(|| AxiomError::NotFound(format!("anchor {anchor_id}")))?;
        Ok(anchor_view(anchor, &sessions, &entries, &artifacts))
    }

    pub fn search_cases(&self, request: SearchCasesRequest) -> Result<Vec<SearchHit>> {
        Ok(filter_hits(
            crate::query::search_cases(
                &self.repo.load_sessions()?,
                &self.repo.load_episodes()?,
                &request,
            ),
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

    pub fn case_workspace_id(&self, case_id: &str) -> Result<Option<String>> {
        let case = self.get_case(case_id)?;
        Ok(case
            .workspace_root
            .as_deref()
            .map(crate::domain::workspace_stable_id))
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

    pub fn task_workspace_id(&self, task_id: &str) -> Result<Option<String>> {
        let task = self.get_task(task_id)?;
        Ok(task
            .session
            .workspace_root
            .as_deref()
            .map(crate::domain::workspace_stable_id))
    }

    pub fn run_workspace_id(&self, run_id: &str) -> Result<Option<String>> {
        let run = self.get_run(run_id)?;
        Ok(run
            .session
            .workspace_root
            .as_deref()
            .map(crate::domain::workspace_stable_id))
    }

    pub fn document_workspace_id(&self, document_id: &str) -> Result<Option<String>> {
        let document = self.get_document(document_id)?;
        Ok(document
            .session
            .and_then(|session| session.workspace_root)
            .as_deref()
            .map(crate::domain::workspace_stable_id))
    }

    pub fn evidence_workspace_id(&self, evidence_id: &str) -> Result<Option<String>> {
        let evidence = self.get_evidence(evidence_id)?;
        Ok(evidence
            .session
            .and_then(|session| session.workspace_root)
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
        crate::logic::auth::authorize_workspace_token(&snapshot, token, workspace_id)
    }

    pub fn authorize_admin(&self, token: &str) -> Result<()> {
        let snapshot: AuthSnapshot = self.auth.read()?;
        crate::logic::auth::authorize_admin_token(&snapshot, token)
    }

    pub fn pending_counts(&self) -> Result<(usize, usize, usize)> {
        self.repo.pending_counts()
    }
}

fn effective_limit(limit: usize) -> usize {
    if limit == 0 { 10 } else { limit }
}

fn session_view(
    session: &SessionRow,
    entries: &[EntryRow],
    artifacts: &[crate::domain::ArtifactRow],
    anchors: &[crate::domain::AnchorRow],
) -> SessionView {
    let mut bundles = entries
        .iter()
        .filter(|entry| entry.session_id == session.session_id)
        .map(|entry| EntryBundle {
            entry: entry.clone(),
            artifacts: artifacts
                .iter()
                .filter(|artifact| artifact.entry_id.as_deref() == Some(entry.entry_id.as_str()))
                .cloned()
                .collect(),
            anchors: anchors
                .iter()
                .filter(|anchor| anchor.entry_id.as_deref() == Some(entry.entry_id.as_str()))
                .cloned()
                .collect(),
        })
        .collect::<Vec<_>>();
    bundles.sort_by_key(|bundle| bundle.entry.seq_no);
    SessionView {
        session: session.clone(),
        entries: bundles,
    }
}

fn document_view_from_artifact(
    artifact: &crate::domain::ArtifactRow,
    sessions: &[SessionRow],
    entries: &[EntryRow],
) -> ArtifactView {
    ArtifactView {
        artifact: artifact.clone(),
        session: sessions
            .iter()
            .find(|session| session.session_id == artifact.session_id)
            .cloned(),
        entry: artifact
            .entry_id
            .as_deref()
            .and_then(|entry_id| entries.iter().find(|entry| entry.entry_id == entry_id))
            .cloned(),
    }
}

fn anchor_view(
    anchor: &crate::domain::AnchorRow,
    sessions: &[SessionRow],
    entries: &[EntryRow],
    artifacts: &[crate::domain::ArtifactRow],
) -> AnchorView {
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
    AnchorView {
        anchor: anchor.clone(),
        entry,
        artifact,
        session,
    }
}

fn case_from_episode(
    episode: &crate::domain::EpisodeRow,
    sessions: &[SessionRow],
    insights: &[crate::domain::InsightRow],
    verifications: &[crate::domain::VerificationRow],
    claims: &[crate::domain::ClaimRow],
    procedures: &[crate::domain::ProcedureRow],
) -> CaseRecord {
    let workspace_root = episode
        .session_id
        .as_deref()
        .and_then(|session_id| {
            sessions
                .iter()
                .find(|session| session.session_id == session_id)
        })
        .and_then(|session| session.workspace_root.clone());
    let related_claims = claims
        .iter()
        .filter(|claim| claim.episode_id.as_deref() == Some(episode.episode_id.as_str()))
        .collect::<Vec<_>>();
    let root_cause = related_claims
        .iter()
        .find(|claim| claim.claim_kind == "root_cause")
        .map(|claim| claim.statement.clone())
        .or_else(|| {
            insights
                .iter()
                .find(|insight| {
                    insight.episode_id.as_deref() == Some(episode.episode_id.as_str())
                        && insight.insight_kind == "root_cause"
                })
                .map(|insight| insight.statement.clone())
        });
    let resolution = related_claims
        .iter()
        .find(|claim| claim.claim_kind == "fix")
        .map(|claim| claim.statement.clone())
        .or_else(|| {
            insights
                .iter()
                .find(|insight| {
                    insight.episode_id.as_deref() == Some(episode.episode_id.as_str())
                        && insight.insight_kind == "fix"
                })
                .map(|insight| insight.statement.clone())
        });
    let related_procedures = procedures
        .iter()
        .filter(|procedure| procedure.episode_id.as_deref() == Some(episode.episode_id.as_str()))
        .collect::<Vec<_>>();
    let commands = related_procedures
        .iter()
        .flat_map(|procedure| {
            procedure
                .steps_json
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|step| step.as_str())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let related_insight_ids = insights
        .iter()
        .filter(|insight| insight.episode_id.as_deref() == Some(episode.episode_id.as_str()))
        .map(|insight| insight.insight_id.as_str())
        .collect::<Vec<_>>();
    let related_procedure_ids = related_procedures
        .iter()
        .map(|procedure| procedure.procedure_id.as_str())
        .collect::<Vec<_>>();
    CaseRecord {
        case_id: episode.episode_id.clone(),
        workspace_root,
        problem: episode.summary.clone(),
        root_cause,
        resolution,
        commands,
        verification: verifications
            .iter()
            .filter(|verification| {
                (verification.subject_kind == "insight"
                    && related_insight_ids.contains(&verification.subject_id.as_str()))
                    || (verification.subject_kind == "procedure"
                        && related_procedure_ids.contains(&verification.subject_id.as_str()))
            })
            .map(|verification| VerificationSummary {
                status: verification.status.clone(),
                method: verification.method.clone(),
            })
            .collect(),
        evidence: related_claims
            .iter()
            .map(|claim| claim.claim_id.clone())
            .collect(),
    }
}

fn task_view(view: SessionView, task_id: &str) -> Result<TaskView> {
    if view.session.session_kind == "task" {
        Ok(view)
    } else {
        Err(AxiomError::NotFound(format!("task {task_id}")))
    }
}

fn session_workspace(sessions: &[SessionRow], session_id: &str) -> Option<String> {
    sessions
        .iter()
        .find(|session| session.session_id == session_id)
        .and_then(|session| session.workspace_root.as_deref())
        .map(workspace_stable_id)
}
