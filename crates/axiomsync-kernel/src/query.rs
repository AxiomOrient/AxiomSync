use std::collections::{BTreeMap, BTreeSet};

use axiomsync_domain::{
    AnchorRow, EpisodeRow, InsightAnchorRow, InsightRow, ProcedureRow, SearchCasesRequest,
    SearchDocsRow, SearchFilter, SearchHit, SessionRow, VerificationRow, normalize_search_query,
};

pub struct SearchCorpus<'a> {
    pub sessions: &'a [SessionRow],
    pub episodes: &'a [EpisodeRow],
    pub insights: &'a [InsightRow],
    pub procedures: &'a [ProcedureRow],
    pub insight_anchors: &'a [InsightAnchorRow],
    pub verifications: &'a [VerificationRow],
    pub search_docs: &'a [SearchDocsRow],
    pub anchors: &'a [AnchorRow],
}

pub fn search_cases(corpus: SearchCorpus<'_>, request: &SearchCasesRequest) -> Vec<SearchHit> {
    let Some(normalized_query) = normalize_search_query(&request.query) else {
        return Vec::new();
    };
    let tokens = normalized_query.split_whitespace().collect::<Vec<_>>();

    let insight_to_episode = corpus
        .insights
        .iter()
        .filter_map(|insight| {
            insight
                .episode_id
                .as_ref()
                .map(|episode_id| (insight.insight_id.as_str(), episode_id.as_str()))
        })
        .collect::<BTreeMap<_, _>>();
    let procedure_to_episode = corpus
        .procedures
        .iter()
        .filter_map(|procedure| {
            procedure
                .episode_id
                .as_ref()
                .map(|episode_id| (procedure.procedure_id.as_str(), episode_id.as_str()))
        })
        .collect::<BTreeMap<_, _>>();

    let anchor_lookup = corpus
        .anchors
        .iter()
        .map(|anchor| (anchor.anchor_id.as_str(), anchor))
        .collect::<BTreeMap<_, _>>();
    let mut evidence_by_episode = BTreeMap::<&str, Vec<&AnchorRow>>::new();
    for row in corpus.insight_anchors {
        let Some(episode_id) = insight_to_episode.get(row.insight_id.as_str()).copied() else {
            continue;
        };
        let Some(anchor) = anchor_lookup.get(row.anchor_id.as_str()).copied() else {
            continue;
        };
        evidence_by_episode
            .entry(episode_id)
            .or_default()
            .push(anchor);
    }

    let mut verification_by_episode = BTreeMap::<&str, Vec<&VerificationRow>>::new();
    for verification in corpus.verifications {
        let episode_id = match verification.subject_kind.as_str() {
            "insight" => insight_to_episode
                .get(verification.subject_id.as_str())
                .copied(),
            "procedure" => procedure_to_episode
                .get(verification.subject_id.as_str())
                .copied(),
            "episode" => Some(verification.subject_id.as_str()),
            _ => None,
        };
        if let Some(episode_id) = episode_id {
            verification_by_episode
                .entry(episode_id)
                .or_default()
                .push(verification);
        }
    }

    let mut docs_by_episode = BTreeMap::<&str, Vec<&SearchDocsRow>>::new();
    for doc in corpus.search_docs {
        if let Some(episode_id) = doc_episode_id(doc, &insight_to_episode, &procedure_to_episode) {
            docs_by_episode.entry(episode_id).or_default().push(doc);
        }
    }

    let mut hits = corpus
        .episodes
        .iter()
        .filter_map(|episode| {
            let session = episode.session_id.as_deref().and_then(|session_id| {
                corpus
                    .sessions
                    .iter()
                    .find(|session| session.session_id == session_id)
            })?;
            if !session_matches(session, &request.filter) {
                return None;
            }

            let docs = docs_by_episode
                .get(episode.episode_id.as_str())
                .cloned()
                .unwrap_or_default();
            let evidence = evidence_by_episode
                .get(episode.episode_id.as_str())
                .cloned()
                .unwrap_or_default();
            let verification = verification_by_episode
                .get(episode.episode_id.as_str())
                .cloned()
                .unwrap_or_default();

            let mut matched_docs = Vec::new();
            let mut total_score = 0.0;
            for doc in docs {
                let doc_score = doc_score(doc, &normalized_query, &tokens);
                if doc_score > 0.0 {
                    matched_docs.push((doc, doc_score));
                    total_score += doc_score;
                }
            }

            let summary_score = summary_score(&episode.summary, &normalized_query, &tokens);
            total_score += summary_score;
            if matched_docs.is_empty() && summary_score <= 0.0 {
                return None;
            }

            total_score += evidence_score(&evidence);
            total_score += verification_score(&verification);

            let snippet = matched_docs
                .iter()
                .max_by(|left, right| left.1.total_cmp(&right.1))
                .map(|(doc, _)| snippet(doc, &request.query))
                .unwrap_or_else(|| fallback_snippet(&episode.summary, &request.query));

            let evidence = unique_evidence(&evidence);
            Some(SearchHit {
                id: episode.episode_id.clone(),
                kind: "case".to_string(),
                title: episode.summary.clone(),
                snippet,
                score: total_score,
                evidence,
            })
        })
        .collect::<Vec<_>>();

    hits.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.id.cmp(&right.id))
    });
    hits
}

fn session_matches(session: &SessionRow, filter: &SearchFilter) -> bool {
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

fn doc_episode_id<'a>(
    doc: &'a SearchDocsRow,
    insight_to_episode: &BTreeMap<&'a str, &'a str>,
    procedure_to_episode: &BTreeMap<&'a str, &'a str>,
) -> Option<&'a str> {
    match doc.subject_kind.as_str() {
        "episode" => Some(doc.subject_id.as_str()),
        "insight" => insight_to_episode.get(doc.subject_id.as_str()).copied(),
        "procedure" => procedure_to_episode.get(doc.subject_id.as_str()).copied(),
        _ => None,
    }
}

fn doc_score(doc: &SearchDocsRow, normalized_query: &str, tokens: &[&str]) -> f64 {
    let title = doc
        .title
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let body = doc.body.to_ascii_lowercase();
    let all_tokens_match = tokens
        .iter()
        .all(|token| title.contains(token) || body.contains(token));
    if !all_tokens_match {
        return 0.0;
    }

    let mut score = match doc.doc_kind.as_str() {
        "episode" => 4.0,
        "insight" => 5.0,
        "procedure" => 3.5,
        _ => 2.0,
    };
    if title == normalized_query {
        score += 6.0;
    }
    if body == normalized_query {
        score += 4.0;
    }
    if body.contains(normalized_query) {
        score += 3.0;
    }
    for token in tokens {
        if title.contains(token) {
            score += 2.0;
        }
        if body.contains(token) {
            score += 1.0;
        }
    }
    score
}

fn summary_score(summary: &str, normalized_query: &str, tokens: &[&str]) -> f64 {
    let lowered = summary.to_ascii_lowercase();
    if !tokens.iter().all(|token| lowered.contains(token)) {
        return 0.0;
    }
    let mut score = 3.0;
    if lowered == normalized_query {
        score += 5.0;
    }
    if lowered.contains(normalized_query) {
        score += 2.0;
    }
    score
}

fn evidence_score(evidence: &[&AnchorRow]) -> f64 {
    let distinct = evidence
        .iter()
        .map(|anchor| anchor.anchor_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    (distinct.min(4) as f64) * 0.5
}

fn verification_score(verifications: &[&VerificationRow]) -> f64 {
    let mut score = 0.0;
    if verifications.iter().any(|row| row.status == "verified") {
        score += 2.5;
    }
    if verifications.iter().any(|row| row.status == "proposed") {
        score += 0.5;
    }
    if verifications.iter().any(|row| row.status == "conflicted") {
        score -= 1.5;
    }
    score
}

fn snippet(doc: &SearchDocsRow, query: &str) -> String {
    let text = doc
        .title
        .as_deref()
        .filter(|title| !title.trim().is_empty())
        .map(|title| format!("{title}\n{}", doc.body))
        .unwrap_or_else(|| doc.body.clone());
    fallback_snippet(&text, query)
}

fn fallback_snippet(text: &str, query: &str) -> String {
    let lowered = text.to_ascii_lowercase();
    let query_lower = query.to_ascii_lowercase();
    if let Some(byte_pos) = lowered.find(query_lower.as_str()) {
        let start_hint = byte_pos.saturating_sub(24);
        let start = (start_hint..=byte_pos)
            .find(|&i| text.is_char_boundary(i))
            .unwrap_or(0);
        let end_hint = (byte_pos + query_lower.len() + 72).min(text.len());
        let end = (0..=end_hint)
            .rev()
            .find(|&i| text.is_char_boundary(i))
            .unwrap_or(text.len());
        text[start..end].to_string()
    } else {
        text.chars().take(120).collect()
    }
}

fn unique_evidence(evidence: &[&AnchorRow]) -> Vec<axiomsync_domain::EvidencePreview> {
    let mut seen = BTreeSet::new();
    evidence
        .iter()
        .filter(|anchor| seen.insert(anchor.anchor_id.clone()))
        .take(3)
        .map(|anchor| axiomsync_domain::EvidencePreview {
            anchor_id: anchor.anchor_id.clone(),
            preview_text: anchor.preview_text.clone(),
        })
        .collect()
}
