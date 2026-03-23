use std::collections::BTreeMap;

use crate::domain::{
    EpisodeConnectorRow, EpisodeEvidenceSearchRow, EpisodeRow, InsightKind, InsightRow,
    SearchCommandCandidateRow, SearchCommandsResult, SearchEpisodeFtsRow, SearchEpisodesFilter,
    SearchEpisodesResult, VerificationRow, VerificationStatus,
};

pub struct EpisodeSearchRows<'a> {
    pub fts_rows: &'a [SearchEpisodeFtsRow],
    pub evidence_rows: &'a [EpisodeEvidenceSearchRow],
    pub episodes: &'a [EpisodeRow],
    pub insights: &'a [InsightRow],
    pub verifications: &'a [VerificationRow],
    pub connector_rows: &'a [EpisodeConnectorRow],
}

pub fn filter_matches(
    filter: &SearchEpisodesFilter,
    workspace_id: Option<&str>,
    connector: Option<&str>,
    status: crate::domain::EpisodeStatus,
) -> bool {
    if let Some(expected) = filter.workspace_id.as_deref()
        && workspace_id != Some(expected)
    {
        return false;
    }
    if let Some(expected) = filter.source.as_deref()
        && connector != Some(expected)
    {
        return false;
    }
    if let Some(expected) = filter.status
        && status != expected
    {
        return false;
    }
    true
}

pub fn search_episode_results(
    query: &str,
    limit: usize,
    filter: &SearchEpisodesFilter,
    rows: EpisodeSearchRows<'_>,
) -> Vec<SearchEpisodesResult> {
    let mut results = aggregate_episode_search_results(rows.fts_rows, filter, rows.insights);
    if results.is_empty() {
        results = fallback_episode_search_results(
            query,
            filter,
            rows.evidence_rows,
            rows.episodes,
            rows.insights,
            rows.verifications,
            rows.connector_rows,
        );
    }
    sort_episode_search_results(&mut results);
    results.truncate(limit);
    results
}

pub fn search_command_results(
    query: &str,
    limit: usize,
    candidates: &[SearchCommandCandidateRow],
    workspace_id: Option<&str>,
) -> Vec<SearchCommandsResult> {
    let lowered = query.to_ascii_lowercase();
    let mut rows = candidates
        .iter()
        .filter(|candidate| {
            workspace_id.is_none_or(|expected| candidate.workspace_id.as_deref() == Some(expected))
        })
        .filter_map(|candidate| {
            let command_lower = candidate.command.to_ascii_lowercase();
            if !command_lower.contains(&lowered) {
                return None;
            }
            Some(SearchCommandsResult {
                episode_id: candidate.episode_id.clone(),
                command: candidate.command.clone(),
                score: lowered.len() as f64 / command_lower.len().max(1) as f64,
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(left.command.cmp(&right.command))
            .then(left.episode_id.cmp(&right.episode_id))
    });
    rows.truncate(limit);
    rows
}

fn aggregate_episode_search_results(
    fts_rows: &[SearchEpisodeFtsRow],
    filter: &SearchEpisodesFilter,
    insights: &[InsightRow],
) -> Vec<SearchEpisodesResult> {
    let mut aggregated = BTreeMap::<String, SearchEpisodesResult>::new();
    for row in fts_rows.iter().filter(|row| {
        filter_matches(
            filter,
            row.workspace_id.as_deref(),
            row.connector.as_deref(),
            row.status,
        )
    }) {
        let entry =
            aggregated
                .entry(row.episode_id.clone())
                .or_insert_with(|| SearchEpisodesResult {
                    episode_id: row.episode_id.clone(),
                    workspace_id: row.workspace_id.clone(),
                    source: row.connector.clone(),
                    status: row.status,
                    problem: String::new(),
                    root_cause: None,
                    fix: None,
                    score: f64::MIN,
                });
        match (row.matched_kind, row.matched_summary.as_ref()) {
            (Some(InsightKind::Problem), Some(summary)) if entry.problem.is_empty() => {
                entry.problem = summary.clone()
            }
            (Some(InsightKind::RootCause), Some(summary)) if entry.root_cause.is_none() => {
                entry.root_cause = Some(summary.clone())
            }
            (Some(InsightKind::Fix), Some(summary)) if entry.fix.is_none() => {
                entry.fix = Some(summary.clone())
            }
            _ => {}
        }
        entry.score = entry.score.max(1.0 + f64::from(row.pass_boost));
    }
    for entry in aggregated.values_mut() {
        hydrate_episode_summary(entry, insights);
    }
    aggregated.into_values().collect()
}

fn fallback_episode_search_results(
    query: &str,
    filter: &SearchEpisodesFilter,
    evidence_rows: &[EpisodeEvidenceSearchRow],
    episodes: &[EpisodeRow],
    insights: &[InsightRow],
    verifications: &[VerificationRow],
    connector_rows: &[EpisodeConnectorRow],
) -> Vec<SearchEpisodesResult> {
    let query_lower = query.to_ascii_lowercase();
    let connectors = first_connector_by_episode(connector_rows);
    let evidence_by_episode = evidence_rows.iter().fold(
        BTreeMap::<String, Vec<&EpisodeEvidenceSearchRow>>::new(),
        |mut acc, row| {
            acc.entry(row.episode_id.clone()).or_default().push(row);
            acc
        },
    );
    let mut results = Vec::new();
    for episode in episodes {
        let connector = connectors.get(&episode.stable_id).cloned().flatten();
        if !filter_matches(
            filter,
            episode.workspace_id.as_deref(),
            connector.as_deref(),
            episode.status,
        ) {
            continue;
        }
        let episode_insights = insights
            .iter()
            .filter(|insight| insight.episode_id == episode.stable_id)
            .collect::<Vec<_>>();
        let haystack = episode_insights
            .iter()
            .map(|insight| format!("{} {}", insight.summary, insight.normalized_text))
            .collect::<Vec<_>>()
            .join("\n")
            .to_ascii_lowercase();
        let evidence_haystack = evidence_by_episode
            .get(&episode.stable_id)
            .into_iter()
            .flatten()
            .map(|row| {
                format!(
                    "{}\n{}",
                    row.quoted_text.clone().unwrap_or_default(),
                    row.body_text.clone().unwrap_or_default()
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
            .to_ascii_lowercase();
        let insight_match = haystack.contains(&query_lower);
        let evidence_match = evidence_haystack.contains(&query_lower);
        if !insight_match && !evidence_match {
            continue;
        }
        let has_pass = verifications.iter().any(|verification| {
            verification.episode_id == episode.stable_id
                && verification.status == VerificationStatus::Pass
        });
        results.push(SearchEpisodesResult {
            episode_id: episode.stable_id.clone(),
            workspace_id: episode.workspace_id.clone(),
            source: connector,
            status: episode.status,
            problem: episode_insights
                .iter()
                .find(|insight| insight.kind == InsightKind::Problem)
                .map(|insight| insight.summary.clone())
                .unwrap_or_default(),
            root_cause: episode_insights
                .iter()
                .find(|insight| insight.kind == InsightKind::RootCause)
                .map(|insight| insight.summary.clone()),
            fix: episode_insights
                .iter()
                .find(|insight| insight.kind == InsightKind::Fix)
                .map(|insight| insight.summary.clone()),
            score: evidence_fallback_score(insight_match, evidence_match, has_pass),
        });
    }
    results
}

fn evidence_fallback_score(insight_match: bool, evidence_match: bool, has_pass: bool) -> f64 {
    let mut score = 0.0;
    if insight_match {
        score += 1.0;
    }
    if evidence_match {
        score += 0.75;
    }
    if has_pass {
        score += 0.5;
    }
    score
}

fn first_connector_by_episode(
    connector_rows: &[EpisodeConnectorRow],
) -> BTreeMap<String, Option<String>> {
    let mut rows = connector_rows.to_vec();
    rows.sort_by(|left, right| {
        left.episode_id
            .cmp(&right.episode_id)
            .then(left.turn_index.cmp(&right.turn_index))
    });
    rows.into_iter().fold(BTreeMap::new(), |mut acc, row| {
        acc.entry(row.episode_id).or_insert(row.connector);
        acc
    })
}

fn hydrate_episode_summary(entry: &mut SearchEpisodesResult, insights: &[InsightRow]) {
    let episode_insights = insights
        .iter()
        .filter(|insight| insight.episode_id == entry.episode_id);
    for insight in episode_insights {
        match insight.kind {
            InsightKind::Problem if entry.problem.is_empty() => {
                entry.problem = insight.summary.clone()
            }
            InsightKind::RootCause if entry.root_cause.is_none() => {
                entry.root_cause = Some(insight.summary.clone())
            }
            InsightKind::Fix if entry.fix.is_none() => entry.fix = Some(insight.summary.clone()),
            _ => {}
        }
    }
}

fn sort_episode_search_results(results: &mut [SearchEpisodesResult]) {
    results.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(left.episode_id.cmp(&right.episode_id))
    });
}
