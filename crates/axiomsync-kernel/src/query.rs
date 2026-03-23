use axiomsync_domain::domain::{
    AnchorRow, ClaimRow, EntryRow, EpisodeRow, EvidencePreview, InsightAnchorRow, InsightRow,
    ProcedureRow, SearchClaimsRequest, SearchDocsRequest, SearchDocsRow, SearchEntriesRequest,
    SearchEpisodesRequest, SearchFilter, SearchHit, SearchInsightsRequest, SearchProceduresRequest,
    SessionRow, normalize_search_query,
};
use serde_json::Value;

pub fn search_entries(
    sessions: &[SessionRow],
    entries: &[EntryRow],
    request: &SearchEntriesRequest,
) -> Vec<SearchHit> {
    search_text(
        normalize_search_query(&request.query).as_deref(),
        entries
            .iter()
            .filter(|entry| session_matches(sessions, &entry.session_id, &request.filter))
            .filter_map(|entry| {
                let text = entry.text_body.as_deref()?;
                if !matches_query(text, &request.query) {
                    return None;
                }
                Some(SearchHit {
                    id: entry.entry_id.clone(),
                    kind: "entry".to_string(),
                    title: format!("entry {}", entry.seq_no),
                    snippet: snippet(text, &request.query),
                    score: score(text, &request.query),
                    evidence: Vec::new(),
                })
            })
            .collect(),
    )
}

pub fn search_episodes(
    sessions: &[SessionRow],
    episodes: &[EpisodeRow],
    request: &SearchEpisodesRequest,
) -> Vec<SearchHit> {
    search_text(
        normalize_search_query(&request.query).as_deref(),
        episodes
            .iter()
            .filter(|episode| {
                episode.session_id.as_deref().is_some_and(|session_id| {
                    session_matches(sessions, session_id, &request.filter)
                })
            })
            .filter(|episode| matches_query(&episode.summary, &request.query))
            .map(|episode| SearchHit {
                id: episode.episode_id.clone(),
                kind: "episode".to_string(),
                title: episode.summary.clone(),
                snippet: snippet(&episode.summary, &request.query),
                score: score(&episode.summary, &request.query),
                evidence: Vec::new(),
            })
            .collect(),
    )
}

pub fn search_insights(
    sessions: &[SessionRow],
    episodes: &[EpisodeRow],
    insights: &[InsightRow],
    insight_anchors: &[InsightAnchorRow],
    anchors: &[AnchorRow],
    request: &SearchInsightsRequest,
) -> Vec<SearchHit> {
    search_text(
        normalize_search_query(&request.query).as_deref(),
        insights
            .iter()
            .filter(|insight| {
                insight.episode_id.as_deref().is_some_and(|episode_id| {
                    episodes
                        .iter()
                        .find(|episode| episode.episode_id == episode_id)
                        .and_then(|episode| episode.session_id.as_deref())
                        .is_some_and(|session_id| {
                            session_matches(sessions, session_id, &request.filter)
                        })
                })
            })
            .filter(|insight| matches_query(&insight.statement, &request.query))
            .map(|insight| SearchHit {
                id: insight.insight_id.clone(),
                kind: "insight".to_string(),
                title: insight.insight_kind.clone(),
                snippet: snippet(&insight.statement, &request.query),
                score: score(&insight.statement, &request.query),
                evidence: insight_anchors
                    .iter()
                    .filter(|row| row.insight_id == insight.insight_id)
                    .filter_map(|row| {
                        anchors
                            .iter()
                            .find(|anchor| anchor.anchor_id == row.anchor_id)
                    })
                    .map(|anchor| EvidencePreview {
                        anchor_id: anchor.anchor_id.clone(),
                        preview_text: anchor.preview_text.clone(),
                    })
                    .collect(),
            })
            .collect(),
    )
}

pub fn search_claims(
    sessions: &[SessionRow],
    episodes: &[EpisodeRow],
    claims: &[ClaimRow],
    request: &SearchClaimsRequest,
) -> Vec<SearchHit> {
    search_text(
        normalize_search_query(&request.query).as_deref(),
        claims
            .iter()
            .filter(|claim| {
                claim.episode_id.as_deref().is_some_and(|episode_id| {
                    episodes
                        .iter()
                        .find(|episode| episode.episode_id == episode_id)
                        .and_then(|episode| episode.session_id.as_deref())
                        .is_some_and(|session_id| {
                            session_matches(sessions, session_id, &request.filter)
                        })
                })
            })
            .filter(|claim| matches_query(&claim.statement, &request.query))
            .map(|claim| SearchHit {
                id: claim.claim_id.clone(),
                kind: "claim".to_string(),
                title: claim.claim_kind.clone(),
                snippet: snippet(&claim.statement, &request.query),
                score: score(&claim.statement, &request.query),
                evidence: Vec::new(),
            })
            .collect(),
    )
}

pub fn search_procedures(
    sessions: &[SessionRow],
    procedures: &[ProcedureRow],
    request: &SearchProceduresRequest,
) -> Vec<SearchHit> {
    let workspace_root = request.filter.workspace_root.as_deref();
    let session_kind = request.filter.session_kind.as_deref();
    let connector = request.filter.connector.as_deref();
    let allow_all = workspace_root.is_none() && session_kind.is_none() && connector.is_none();

    search_text(
        normalize_search_query(&request.query).as_deref(),
        procedures
            .iter()
            .filter(|_| allow_all || !sessions.is_empty())
            .filter(|procedure| {
                matches_query(&procedure.title, &request.query)
                    || procedure
                        .goal
                        .as_deref()
                        .is_some_and(|goal| matches_query(goal, &request.query))
                    || procedure.steps_json.to_string().contains(&request.query)
            })
            .map(|procedure| SearchHit {
                id: procedure.procedure_id.clone(),
                kind: "procedure".to_string(),
                title: procedure.title.clone(),
                snippet: procedure
                    .goal
                    .clone()
                    .unwrap_or_else(|| procedure.steps_json.to_string()),
                score: score(&procedure.title, &request.query),
                evidence: Vec::new(),
            })
            .collect(),
    )
}

pub fn search_docs(docs: &[SearchDocsRow], request: &SearchDocsRequest) -> Vec<SearchHit> {
    search_text(
        normalize_search_query(&request.query).as_deref(),
        docs.iter()
            .filter(|doc| search_doc_matches(doc, &request.filter))
            .filter(|doc| {
                matches_query(&doc.body, &request.query)
                    || doc
                        .title
                        .as_deref()
                        .is_some_and(|title| matches_query(title, &request.query))
            })
            .map(|doc| SearchHit {
                id: doc.subject_id.clone(),
                kind: doc.subject_kind.clone(),
                title: doc.title.clone().unwrap_or_else(|| doc.doc_kind.clone()),
                snippet: snippet(&doc.body, &request.query),
                score: score(&doc.body, &request.query)
                    + doc
                        .title
                        .as_deref()
                        .map(|title| score(title, &request.query))
                        .unwrap_or(0.0),
                evidence: Vec::new(),
            })
            .collect(),
    )
}

fn session_matches(sessions: &[SessionRow], session_id: &str, filter: &SearchFilter) -> bool {
    let Some(session) = sessions
        .iter()
        .find(|session| session.session_id == session_id)
    else {
        return false;
    };
    filter
        .session_kind
        .as_deref()
        .is_none_or(|expected| session.session_kind == expected)
        && filter
            .connector
            .as_deref()
            .is_none_or(|expected| session.connector == expected)
        && filter
            .workspace_root
            .as_deref()
            .is_none_or(|expected| session.workspace_root.as_deref() == Some(expected))
}

fn search_doc_matches(doc: &SearchDocsRow, filter: &SearchFilter) -> bool {
    filter.session_kind.as_deref().is_none_or(|expected| {
        metadata_string(&doc.metadata_json, "session_kind").as_deref() == Some(expected)
    }) && filter.connector.as_deref().is_none_or(|expected| {
        metadata_string(&doc.metadata_json, "connector").as_deref() == Some(expected)
    }) && filter.workspace_root.as_deref().is_none_or(|expected| {
        metadata_string(&doc.metadata_json, "workspace_root").as_deref() == Some(expected)
    })
}

fn metadata_string(metadata: &Value, key: &str) -> Option<String> {
    metadata
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn matches_query(text: &str, query: &str) -> bool {
    let normalized = normalize_search_query(query).unwrap_or_else(|| query.to_ascii_lowercase());
    let haystack = text.to_ascii_lowercase();
    normalized
        .split_whitespace()
        .all(|token| haystack.contains(token))
}

fn snippet(text: &str, query: &str) -> String {
    let query = query.to_ascii_lowercase();
    let lowered = text.to_ascii_lowercase();
    if let Some(index) = lowered.find(&query) {
        let start = index.saturating_sub(24);
        let end = (index + query.len() + 72).min(text.len());
        text[start..end].to_string()
    } else {
        text.chars().take(96).collect()
    }
}

fn score(text: &str, query: &str) -> f64 {
    let normalized = normalize_search_query(query).unwrap_or_default();
    let lowered = text.to_ascii_lowercase();
    normalized
        .split_whitespace()
        .filter(|token| lowered.contains(token))
        .count() as f64
}

fn search_text(_normalized: Option<&str>, mut hits: Vec<SearchHit>) -> Vec<SearchHit> {
    hits.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.id.cmp(&right.id))
    });
    hits
}
