use axiomsync_domain::domain::{
    AnchorRow, ArtifactRow, ArtifactView, CaseRecord, ClaimRow, EntryBundle, EntryRow, EpisodeRow,
    EpisodeView, ProcedureRow, SessionRow, SessionView, TaskView, workspace_stable_id,
};
use axiomsync_domain::error::{AxiomError, Result};

pub fn session_view(
    session: &SessionRow,
    entries: &[EntryRow],
    artifacts: &[ArtifactRow],
    anchors: &[AnchorRow],
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

pub fn document_view_from_artifact(
    artifact: &ArtifactRow,
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

pub fn episode_view(
    episode: &EpisodeRow,
    claims: &[ClaimRow],
    procedures: &[ProcedureRow],
) -> EpisodeView {
    EpisodeView {
        episode: episode.clone(),
        claims: claims
            .iter()
            .filter(|claim| claim.episode_id.as_deref() == Some(episode.episode_id.as_str()))
            .cloned()
            .collect(),
        procedures: procedures.to_vec(),
    }
}

pub fn case_from_episode(
    episode: &EpisodeRow,
    sessions: &[SessionRow],
    claims: &[ClaimRow],
    procedures: &[ProcedureRow],
) -> CaseRecord {
    let workspace_root = episode
        .session_id
        .as_deref()
        .and_then(|session_id| sessions.iter().find(|session| session.session_id == session_id))
        .and_then(|session| session.workspace_root.clone());
    let related_claims = claims
        .iter()
        .filter(|claim| claim.episode_id.as_deref() == Some(episode.episode_id.as_str()))
        .collect::<Vec<_>>();
    let root_cause = related_claims
        .iter()
        .find(|claim| claim.claim_kind == "root_cause")
        .map(|claim| claim.statement.clone());
    let resolution = related_claims
        .iter()
        .find(|claim| claim.claim_kind == "fix")
        .map(|claim| claim.statement.clone());
    let commands = procedures
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
    CaseRecord {
        case_id: episode.episode_id.clone(),
        workspace_root,
        problem: episode.summary.clone(),
        root_cause,
        resolution,
        commands,
        evidence: related_claims.iter().map(|claim| claim.claim_id.clone()).collect(),
    }
}

pub fn task_view(view: SessionView, task_id: &str) -> Result<TaskView> {
    if view.session.session_kind == "task" {
        Ok(view)
    } else {
        Err(AxiomError::NotFound(format!("task {task_id}")))
    }
}

pub fn session_workspace(sessions: &[SessionRow], session_id: &str) -> Option<String> {
    sessions
        .iter()
        .find(|session| session.session_id == session_id)
        .and_then(|session| session.workspace_root.as_deref())
        .map(workspace_stable_id)
}
