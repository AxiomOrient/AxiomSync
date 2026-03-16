use crate::config::QUERY_PLAN_BACKEND_POLICY_MEMORY_ONLY;
use crate::error::{AxiomError, Result};
use crate::index::InMemoryIndex;
use crate::models::{ContextHit, FindResult, SearchOptions};
use crate::retrieval::scoring::make_hit;
use crate::uri::AxiomUri;

use super::AxiomSync;
use super::reranker::resolve_reranker_mode;
use super::result::append_query_plan_note;

const FTS_FALLBACK_MIN_CANDIDATES: usize = 32;
const FTS_FALLBACK_LIMIT_MULTIPLIER: usize = 8;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct RetrievalExecutionMetadata {
    pub fts_fallback_used: bool,
}

#[derive(Debug)]
pub(super) struct RetrievalExecution {
    pub result: FindResult,
    pub metadata: RetrievalExecutionMetadata,
}

impl AxiomSync {
    pub(super) fn run_retrieval_memory_only_with_metadata(
        &self,
        options: &SearchOptions,
    ) -> Result<RetrievalExecution> {
        let requested_limit = options.limit.max(1);
        let mut result = self.run_memory_retrieval(options)?;
        append_query_plan_note(&mut result, "backend:memory");
        append_query_plan_note(&mut result, QUERY_PLAN_BACKEND_POLICY_MEMORY_ONLY);
        if let Some(threshold) = options.score_threshold {
            append_query_plan_note(&mut result, &format!("score_threshold:{threshold:.3}"));
        }
        if let Some(min_match_tokens) = options.min_match_tokens.filter(|value| *value > 1) {
            append_query_plan_note(&mut result, &format!("min_match_tokens:{min_match_tokens}"));
        }

        let mut metadata = RetrievalExecutionMetadata::default();
        if should_apply_fts_fallback(options, &result)
            && let Some(fallback_hits) = self.run_fts_fallback(options)?
        {
            result.query_results = fallback_hits;
            result.rebuild_hit_buckets();
            append_query_plan_note(&mut result, "fts_fallback:1");
            metadata.fts_fallback_used = true;
        }

        let reranker_mode = resolve_reranker_mode(self.config.search.reranker.as_deref());
        self.apply_reranker_with_mode(&options.query, &mut result, requested_limit, reranker_mode)?;
        Ok(RetrievalExecution { result, metadata })
    }

    fn run_memory_retrieval(&self, options: &SearchOptions) -> Result<crate::models::FindResult> {
        let mut memory_result = {
            let index = self
                .index
                .read()
                .map_err(|_| AxiomError::lock_poisoned("index"))?;
            self.drr.run(&index, options)
        };
        let embed_profile = crate::embedding::embedding_profile();
        append_query_plan_note(
            &mut memory_result,
            &format!(
                "embedder:{}@{}",
                embed_profile.provider, embed_profile.vector_version
            ),
        );
        Ok(memory_result)
    }

    fn run_fts_fallback(&self, options: &SearchOptions) -> Result<Option<Vec<ContextHit>>> {
        let Some(fts_query) = build_fts_query(&options.query) else {
            return Ok(None);
        };
        let candidate_limit = fts_candidate_limit(options.limit.max(1));
        let ranked_records = self
            .state
            .search_documents_fts_with_records(&fts_query, candidate_limit)?;
        if ranked_records.is_empty() {
            return Ok(None);
        }
        let hits = build_fts_fallback_hits(
            &options.query,
            &ranked_records,
            options.target_uri.as_ref(),
            options.filter.as_ref(),
            options.limit.max(1),
        );
        if hits.is_empty() {
            return Ok(None);
        }
        Ok(Some(hits))
    }
}

fn should_apply_fts_fallback(options: &SearchOptions, result: &FindResult) -> bool {
    result.query_results.is_empty() && options.score_threshold.is_none()
}

fn build_fts_query(raw_query: &str) -> Option<String> {
    let tokens = raw_query
        .split(|c: char| !c.is_alphanumeric())
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    if tokens.is_empty() {
        return None;
    }
    Some(tokens.join(" "))
}

fn fts_candidate_limit(limit: usize) -> usize {
    limit
        .saturating_mul(FTS_FALLBACK_LIMIT_MULTIPLIER)
        .max(FTS_FALLBACK_MIN_CANDIDATES)
}

fn build_fts_fallback_hits(
    query: &str,
    ranked_records: &[crate::models::IndexRecord],
    target_uri: Option<&AxiomUri>,
    filter: Option<&crate::models::SearchFilter>,
    limit: usize,
) -> Vec<ContextHit> {
    ranked_records
        .iter()
        .enumerate()
        .filter_map(|(rank, record)| {
            if !record_matches_fts_fallback(record, target_uri, filter) {
                return None;
            }
            Some(make_hit(record, synthetic_fts_score(rank), query, None))
        })
        .take(limit)
        .collect()
}

fn record_matches_fts_fallback(
    record: &crate::models::IndexRecord,
    target_uri: Option<&AxiomUri>,
    filter: Option<&crate::models::SearchFilter>,
) -> bool {
    record.is_leaf
        && uri_matches_target(record.uri.as_str(), target_uri)
        && InMemoryIndex::leaf_record_matches_filter(record, filter)
}

fn uri_matches_target(uri: &str, target_uri: Option<&AxiomUri>) -> bool {
    let Some(target_uri) = target_uri else {
        return true;
    };
    AxiomUri::parse(uri)
        .map(|parsed| parsed.starts_with(target_uri))
        .unwrap_or(false)
}

fn synthetic_fts_score(rank: usize) -> f32 {
    (1.0 - (rank as f32 * 0.01)).max(0.01)
}
