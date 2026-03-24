use axiomsync_domain::{
    EpisodeRow, SearchCasesRequest, SearchFilter, SearchHit, SessionRow, normalize_search_query,
};

pub fn search_cases(
    sessions: &[SessionRow],
    episodes: &[EpisodeRow],
    request: &SearchCasesRequest,
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
                kind: "case".to_string(),
                title: episode.summary.clone(),
                snippet: snippet(&episode.summary, &request.query),
                score: score(&episode.summary, &request.query),
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
