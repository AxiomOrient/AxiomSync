use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};

use crate::uri::AxiomUri;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationSummary {
    pub uri: String,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relation_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_object_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_object_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RelationLink {
    pub id: String,
    pub uris: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextHit {
    pub uri: String,
    pub score: f32,
    #[serde(rename = "abstract")]
    pub abstract_text: String,
    pub context_type: String,
    pub relations: Vec<RelationSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_heading: Option<String>,
    #[serde(default)]
    pub score_components: ScoreComponents,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScoreComponents {
    pub exact: f32,
    pub dense: f32,
    pub sparse: f32,
    pub path: f32,
    pub recency: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FindResult {
    pub query_plan: QueryPlan,
    pub query_results: Vec<ContextHit>,
    #[serde(default, skip_serializing_if = "HitBuckets::is_empty")]
    pub hit_buckets: HitBuckets,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<RetrievalTrace>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindResultCompatView {
    pub query_plan: QueryPlan,
    pub query_results: Vec<ContextHit>,
    #[serde(default, skip_serializing_if = "HitBuckets::is_empty")]
    pub hit_buckets: HitBuckets,
    pub memories: Vec<ContextHit>,
    pub resources: Vec<ContextHit>,
    pub skills: Vec<ContextHit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<RetrievalTrace>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct HitBuckets {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub memories: Vec<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<usize>,
}

impl HitBuckets {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.memories.is_empty() && self.resources.is_empty() && self.skills.is_empty()
    }
}

impl FindResult {
    #[must_use]
    pub fn new(
        query_plan: QueryPlan,
        query_results: Vec<ContextHit>,
        trace: Option<RetrievalTrace>,
    ) -> Self {
        let hit_buckets = classify_hit_buckets(&query_results);
        Self {
            query_plan,
            query_results,
            hit_buckets,
            trace,
            trace_uri: None,
        }
    }

    pub fn rebuild_hit_buckets(&mut self) {
        self.hit_buckets = classify_hit_buckets(&self.query_results);
    }

    pub fn memories(&self) -> impl Iterator<Item = &ContextHit> {
        bucket_hits(&self.query_results, &self.hit_buckets.memories)
    }

    pub fn resources(&self) -> impl Iterator<Item = &ContextHit> {
        bucket_hits(&self.query_results, &self.hit_buckets.resources)
    }

    pub fn skills(&self) -> impl Iterator<Item = &ContextHit> {
        bucket_hits(&self.query_results, &self.hit_buckets.skills)
    }

    #[must_use]
    pub fn compat_view(self) -> FindResultCompatView {
        let memories = collect_bucket_hits(&self.query_results, &self.hit_buckets.memories);
        let resources = collect_bucket_hits(&self.query_results, &self.hit_buckets.resources);
        let skills = collect_bucket_hits(&self.query_results, &self.hit_buckets.skills);
        FindResultCompatView {
            query_plan: self.query_plan,
            query_results: self.query_results,
            hit_buckets: self.hit_buckets,
            memories,
            resources,
            skills,
            trace: self.trace,
            trace_uri: self.trace_uri,
        }
    }
}

impl Serialize for FindResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut field_count = 2;
        if !self.hit_buckets.is_empty() {
            field_count += 1;
        }
        if self.trace.is_some() {
            field_count += 1;
        }
        if self.trace_uri.is_some() {
            field_count += 1;
        }

        let mut state = serializer.serialize_struct("FindResult", field_count)?;
        state.serialize_field("query_plan", &self.query_plan)?;
        state.serialize_field("query_results", &self.query_results)?;
        if !self.hit_buckets.is_empty() {
            state.serialize_field("hit_buckets", &self.hit_buckets)?;
        }
        if let Some(trace) = &self.trace {
            state.serialize_field("trace", trace)?;
        }
        if let Some(trace_uri) = &self.trace_uri {
            state.serialize_field("trace_uri", trace_uri)?;
        }
        state.end()
    }
}

pub fn classify_hit_buckets(hits: &[ContextHit]) -> HitBuckets {
    let mut buckets = HitBuckets::default();
    for (index, hit) in hits.iter().enumerate() {
        if hit.uri.starts_with("axiom://user/memories")
            || hit.uri.starts_with("axiom://agent/memories")
        {
            buckets.memories.push(index);
        } else if hit.uri.starts_with("axiom://agent/skills") {
            buckets.skills.push(index);
        } else {
            buckets.resources.push(index);
        }
    }
    buckets
}

fn bucket_hits<'a>(
    hits: &'a [ContextHit],
    bucket: &'a [usize],
) -> impl Iterator<Item = &'a ContextHit> + 'a {
    bucket.iter().filter_map(|&index| hits.get(index))
}

fn collect_bucket_hits(hits: &[ContextHit], bucket: &[usize]) -> Vec<ContextHit> {
    bucket_hits(hits, bucket).cloned().collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchFilter {
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub mime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace_prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_time: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_time: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalStep {
    pub round: u32,
    pub current_uri: String,
    pub children_examined: usize,
    pub children_selected: usize,
    pub queue_size_after: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStats {
    pub latency_ms: u128,
    pub explored_nodes: usize,
    pub convergence_rounds: u32,
    #[serde(default)]
    pub typed_query_count: usize,
    #[serde(default)]
    pub relation_enriched_hits: usize,
    #[serde(default)]
    pub relation_enriched_links: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalTrace {
    pub trace_id: String,
    pub request_type: String,
    pub query: String,
    pub target_uri: Option<String>,
    pub start_points: Vec<TracePoint>,
    pub steps: Vec<RetrievalStep>,
    pub final_topk: Vec<TracePoint>,
    pub stop_reason: String,
    pub metrics: TraceStats,
    #[serde(default)]
    pub scope_decision: ScopeDecisionTrace,
    #[serde(default)]
    pub filter_routing_reason: String,
    #[serde(default = "default_restore_source")]
    pub restore_source: String,
    #[serde(default)]
    pub fts_fallback_used: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScopeDecisionTrace {
    pub selected_scopes: Vec<String>,
    pub primary_scope: String,
    pub reasoning: String,
    #[serde(default)]
    pub mixed_intent: bool,
}

pub const RESTORE_SOURCE_UNKNOWN: &str = "runtime_unknown";

fn default_restore_source() -> String {
    RESTORE_SOURCE_UNKNOWN.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracePoint {
    pub uri: String,
    pub score: f32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexRecord {
    pub id: String,
    pub uri: String,
    pub parent_uri: Option<String>,
    pub is_leaf: bool,
    pub context_type: String,
    pub name: String,
    pub abstract_text: String,
    pub content: String,
    pub tags: Vec<String>,
    pub updated_at: DateTime<Utc>,
    pub depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    pub query: String,
    pub target_uri: Option<AxiomUri>,
    pub session: Option<String>,
    #[serde(default)]
    pub session_hints: Vec<String>,
    #[serde(default)]
    pub budget: Option<SearchBudget>,
    pub limit: usize,
    pub score_threshold: Option<f32>,
    pub min_match_tokens: Option<usize>,
    pub filter: Option<SearchFilter>,
    pub request_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchBudget {
    pub max_ms: Option<u64>,
    pub max_nodes: Option<usize>,
    pub max_depth: Option<usize>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueryPlan {
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub typed_queries: Vec<TypedQueryPlan>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedQueryPlan {
    pub kind: String,
    pub query: String,
    pub scopes: Vec<String>,
    pub priority: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace_prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_time: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_time: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataFilter {
    pub fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_threshold: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_match_tokens: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<MetadataFilter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<SearchBudget>,
    #[serde(default)]
    pub runtime_hints: Vec<RuntimeHint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeHintKind {
    Observation,
    CurrentTask,
    SuggestedResponse,
    External,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeHint {
    pub kind: RuntimeHintKind,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingBackendStatus {
    pub provider: String,
    pub vector_version: String,
    pub dim: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendStatus {
    pub local_records: usize,
    pub retrieval_backend: String,
    pub retrieval_backend_policy: String,
    pub embedding: EmbeddingBackendStatus,
}

#[cfg(test)]
mod tests {
    use super::{
        ContextHit, FindResult, QueryPlan, ScoreComponents, TypedQueryPlan, classify_hit_buckets,
    };

    #[test]
    fn query_plan_serialization_snapshot_is_stable() {
        let plan = QueryPlan {
            scopes: vec!["resources".to_string()],
            keywords: vec!["oauth".to_string()],
            typed_queries: vec![
                TypedQueryPlan {
                    kind: "primary".to_string(),
                    query: "oauth".to_string(),
                    scopes: vec!["resources".to_string()],
                    priority: 1,
                    namespace_prefix: None,
                    resource_kind: None,
                    start_time: None,
                    end_time: None,
                },
                TypedQueryPlan {
                    kind: "session_recent".to_string(),
                    query: "oauth hint".to_string(),
                    scopes: vec!["session".to_string()],
                    priority: 2,
                    namespace_prefix: None,
                    resource_kind: None,
                    start_time: None,
                    end_time: None,
                },
            ],
            notes: vec!["backend:memory".to_string(), "budget_nodes:10".to_string()],
        };

        let encoded = serde_json::to_value(&plan).expect("serialize query plan");
        assert_eq!(
            encoded,
            serde_json::json!({
                "scopes": ["resources"],
                "keywords": ["oauth"],
                "typed_queries": [
                    {
                        "kind": "primary",
                        "query": "oauth",
                        "scopes": ["resources"],
                        "priority": 1
                    },
                    {
                        "kind": "session_recent",
                        "query": "oauth hint",
                        "scopes": ["session"],
                        "priority": 2
                    }
                ],
                "notes": ["backend:memory", "budget_nodes:10"]
            })
        );
    }

    fn hit(uri: &str) -> ContextHit {
        ContextHit {
            uri: uri.to_string(),
            score: 0.5,
            abstract_text: String::new(),
            context_type: "resource".to_string(),
            relations: Vec::new(),
            snippet: None,
            matched_heading: None,
            score_components: ScoreComponents::default(),
        }
    }

    #[test]
    fn classify_hit_buckets_assigns_stable_indices() {
        let hits = vec![
            hit("axiom://resources/docs/a.md"),
            hit("axiom://user/memories/preferences/pref.md"),
            hit("axiom://agent/skills/rust.md"),
            hit("axiom://agent/memories/patterns/pat.md"),
        ];
        let buckets = classify_hit_buckets(&hits);
        assert_eq!(buckets.resources, vec![0]);
        assert_eq!(buckets.memories, vec![1, 3]);
        assert_eq!(buckets.skills, vec![2]);
    }

    #[test]
    fn find_result_bucket_accessors_read_from_query_results() {
        let query_results = vec![
            hit("axiom://resources/docs/a.md"),
            hit("axiom://user/memories/preferences/pref.md"),
            hit("axiom://agent/skills/rust.md"),
        ];
        let result = FindResult::new(QueryPlan::default(), query_results, None);
        let memories = result
            .memories()
            .map(|item| item.uri.clone())
            .collect::<Vec<_>>();
        let resources = result
            .resources()
            .map(|item| item.uri.clone())
            .collect::<Vec<_>>();
        let skills = result
            .skills()
            .map(|item| item.uri.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            memories,
            vec!["axiom://user/memories/preferences/pref.md".to_string()]
        );
        assert_eq!(resources, vec!["axiom://resources/docs/a.md".to_string()]);
        assert_eq!(skills, vec!["axiom://agent/skills/rust.md".to_string()]);
    }

    #[test]
    fn find_result_new_classifies_hit_buckets() {
        let query_results = vec![
            hit("axiom://resources/docs/a.md"),
            hit("axiom://user/memories/preferences/pref.md"),
            hit("axiom://agent/skills/rust.md"),
        ];
        let result = FindResult::new(QueryPlan::default(), query_results, None);
        assert_eq!(result.hit_buckets.memories, vec![1]);
        assert_eq!(result.hit_buckets.resources, vec![0]);
        assert_eq!(result.hit_buckets.skills, vec![2]);
    }

    #[test]
    fn find_result_serialization_defaults_to_canonical_contract() {
        let result = FindResult::new(
            QueryPlan::default(),
            vec![
                hit("axiom://resources/docs/a.md"),
                hit("axiom://user/memories/preferences/pref.md"),
                hit("axiom://agent/skills/rust.md"),
            ],
            None,
        );

        let encoded = serde_json::to_value(&result).expect("serialize find result");
        assert!(encoded.get("memories").is_none());
        assert!(encoded.get("resources").is_none());
        assert!(encoded.get("skills").is_none());
    }

    #[test]
    fn find_result_compat_view_includes_legacy_bucket_arrays() {
        let result = FindResult::new(
            QueryPlan::default(),
            vec![
                hit("axiom://resources/docs/a.md"),
                hit("axiom://user/memories/preferences/pref.md"),
                hit("axiom://agent/skills/rust.md"),
            ],
            None,
        );

        let encoded = serde_json::to_value(result.compat_view()).expect("serialize compat view");
        assert_eq!(encoded["memories"].as_array().map(Vec::len), Some(1));
        assert_eq!(encoded["resources"].as_array().map(Vec::len), Some(1));
        assert_eq!(encoded["skills"].as_array().map(Vec::len), Some(1));
    }
}
