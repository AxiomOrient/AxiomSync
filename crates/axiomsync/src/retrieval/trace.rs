use crate::models::{RetrievalStep, RetrievalTrace, TracePoint, TraceStats};

use super::planner::PlannerTraceEvidence;

pub(crate) use crate::models::RESTORE_SOURCE_UNKNOWN as TRACE_RESTORE_SOURCE_UNKNOWN;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TraceExecutionContext {
    pub restore_source: String,
    pub fts_fallback_used: bool,
}

impl Default for TraceExecutionContext {
    fn default() -> Self {
        Self {
            restore_source: TRACE_RESTORE_SOURCE_UNKNOWN.to_string(),
            fts_fallback_used: false,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TraceBuildInput {
    pub trace_id: String,
    pub request_type: String,
    pub query: String,
    pub target_uri: Option<String>,
    pub start_points: Vec<TracePoint>,
    pub steps: Vec<RetrievalStep>,
    pub final_topk: Vec<TracePoint>,
    pub stop_reason: String,
    pub metrics: TraceStats,
    pub planner_evidence: PlannerTraceEvidence,
    pub execution_context: TraceExecutionContext,
}

pub(crate) fn build_trace_execution_context(
    restore_source: impl Into<String>,
    fts_fallback_used: bool,
) -> TraceExecutionContext {
    TraceExecutionContext {
        restore_source: restore_source.into(),
        fts_fallback_used,
    }
}

pub(crate) fn apply_trace_execution_context(
    trace: &mut RetrievalTrace,
    context: &TraceExecutionContext,
) {
    trace.restore_source = context.restore_source.clone();
    trace.fts_fallback_used = context.fts_fallback_used;
}

pub(crate) fn build_retrieval_trace(input: TraceBuildInput) -> RetrievalTrace {
    RetrievalTrace {
        trace_id: input.trace_id,
        request_type: input.request_type,
        query: input.query,
        target_uri: input.target_uri,
        start_points: input.start_points,
        steps: input.steps,
        final_topk: input.final_topk,
        stop_reason: input.stop_reason,
        metrics: input.metrics,
        scope_decision: input.planner_evidence.scope_decision,
        filter_routing_reason: input.planner_evidence.filter_routing_reason,
        restore_source: input.execution_context.restore_source,
        fts_fallback_used: input.execution_context.fts_fallback_used,
    }
}

pub(crate) fn build_trace_evidence(
    selected_scopes: Vec<String>,
    primary_scope: String,
    reasoning: impl Into<String>,
    filter_routing_reason: impl Into<String>,
    mixed_intent: bool,
) -> PlannerTraceEvidence {
    PlannerTraceEvidence {
        scope_decision: crate::models::ScopeDecisionTrace {
            selected_scopes,
            primary_scope,
            reasoning: reasoning.into(),
            mixed_intent,
        },
        filter_routing_reason: filter_routing_reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        TraceBuildInput, TraceExecutionContext, apply_trace_execution_context,
        build_retrieval_trace, build_trace_evidence, build_trace_execution_context,
    };
    use crate::models::{RetrievalTrace, TracePoint, TraceStats};

    fn sample_trace() -> RetrievalTrace {
        build_retrieval_trace(TraceBuildInput {
            trace_id: "trace-1".to_string(),
            request_type: "search".to_string(),
            query: "oauth".to_string(),
            target_uri: Some("axiom://resources".to_string()),
            start_points: vec![TracePoint {
                uri: "axiom://resources".to_string(),
                score: 0.0,
            }],
            steps: Vec::new(),
            final_topk: Vec::new(),
            stop_reason: "queue_empty".to_string(),
            metrics: TraceStats {
                latency_ms: 0,
                explored_nodes: 1,
                convergence_rounds: 0,
                typed_query_count: 1,
                relation_enriched_hits: 0,
                relation_enriched_links: 0,
            },
            planner_evidence: build_trace_evidence(
                vec!["resources".to_string()],
                "resources".to_string(),
                "target_uri",
                "target_uri",
                false,
            ),
            execution_context: TraceExecutionContext::default(),
        })
    }

    #[test]
    fn build_retrieval_trace_uses_execution_context() {
        let trace = build_retrieval_trace(TraceBuildInput {
            execution_context: build_trace_execution_context("state_restore", true),
            ..TraceBuildInput {
                trace_id: "trace-ctx".to_string(),
                request_type: "find".to_string(),
                query: "trace".to_string(),
                target_uri: None,
                start_points: Vec::new(),
                steps: Vec::new(),
                final_topk: Vec::new(),
                stop_reason: "queue_empty".to_string(),
                metrics: TraceStats {
                    latency_ms: 1,
                    explored_nodes: 1,
                    convergence_rounds: 0,
                    typed_query_count: 1,
                    relation_enriched_hits: 0,
                    relation_enriched_links: 0,
                },
                planner_evidence: build_trace_evidence(
                    vec!["resources".to_string()],
                    "resources".to_string(),
                    "query_intent",
                    "query_intent",
                    false,
                ),
                execution_context: TraceExecutionContext::default(),
            }
        });

        assert_eq!(trace.restore_source, "state_restore");
        assert!(trace.fts_fallback_used);
    }

    #[test]
    fn apply_trace_execution_context_updates_existing_trace() {
        let mut trace = sample_trace();
        let context = build_trace_execution_context("full_reindex", true);
        apply_trace_execution_context(&mut trace, &context);

        assert_eq!(trace.restore_source, "full_reindex");
        assert!(trace.fts_fallback_used);
    }
}
