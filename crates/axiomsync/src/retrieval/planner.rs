use std::collections::HashSet;

use crate::models::{Kind, SearchFilter, SearchOptions};
use crate::uri::{AxiomUri, Scope};

#[derive(Debug, Clone)]
pub(super) struct PlannedQuery {
    pub kind: String,
    pub query: String,
    pub scopes: Vec<Scope>,
    pub priority: u8,
    pub namespace_prefix: Option<String>,
    pub resource_kind: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
}

#[derive(Debug, Clone, Copy, Default)]
struct QueryIntent {
    wants_skill: bool,
    wants_memory: bool,
}

impl PlannedQuery {
    fn new(
        kind: &str,
        query: String,
        scopes: Vec<Scope>,
        priority: u8,
        options: &SearchOptions,
    ) -> Self {
        Self {
            kind: kind.to_string(),
            query,
            scopes: normalize_scopes(scopes),
            priority,
            namespace_prefix: options
                .filter
                .as_ref()
                .and_then(|filter| filter.namespace_prefix.clone()),
            resource_kind: options
                .filter
                .as_ref()
                .and_then(|filter| filter.kind.clone()),
            start_time: options.filter.as_ref().and_then(|filter| filter.start_time),
            end_time: options.filter.as_ref().and_then(|filter| filter.end_time),
        }
    }
}

pub(super) fn plan_queries(options: &SearchOptions) -> Vec<PlannedQuery> {
    let intent = query_intent(&options.query);
    let base_scopes = intent_scopes(intent, options.target_uri.as_ref(), options.filter.as_ref());
    let has_session_context = options.session.is_some();
    let mut planned = vec![PlannedQuery::new(
        "primary",
        options.query.clone(),
        base_scopes.clone(),
        1,
        options,
    )];

    if !options.request_type.starts_with("search") {
        return dedup_and_limit_queries(planned, 1);
    }

    if !options.session_hints.is_empty() {
        let hint_text = merge_non_om_hints(&options.session_hints);
        if !hint_text.is_empty() {
            let kind = if has_session_context {
                "session_recent"
            } else {
                "runtime_hints"
            };
            planned.push(PlannedQuery::new(
                kind,
                format!("{} {}", options.query, hint_text),
                base_scopes.clone(),
                2,
                options,
            ));
        }

        if has_session_context
            && let Some(om_hint) = options
                .session_hints
                .iter()
                .find(|hint| is_om_hint(hint))
                .and_then(|hint| normalize_om_hint(hint))
        {
            let om_scopes = if options.target_uri.is_some() {
                base_scopes
            } else {
                vec![Scope::User, Scope::Agent]
            };
            planned.push(PlannedQuery::new(
                "session_om",
                format!("{} {}", options.query, om_hint),
                om_scopes,
                2,
                options,
            ));
        }
    }

    if options.target_uri.is_none() {
        if has_session_context {
            planned.push(PlannedQuery::new(
                "session_focus",
                options.query.clone(),
                vec![Scope::Session],
                session_focus_priority(intent, &options.query),
                options,
            ));
        }
        if intent.wants_skill {
            planned.push(PlannedQuery::new(
                "skill_focus",
                options.query.clone(),
                vec![Scope::Agent],
                2,
                options,
            ));
        }
        if intent.wants_memory || has_session_context {
            planned.push(PlannedQuery::new(
                "memory_focus",
                options.query.clone(),
                vec![Scope::User, Scope::Agent],
                3,
                options,
            ));
        }
    }

    dedup_and_limit_queries(planned, 5)
}

fn query_intent(query: &str) -> QueryIntent {
    let q = query.to_lowercase();
    QueryIntent {
        wants_skill: q.contains("skill"),
        wants_memory: q.contains("memory") || q.contains("preference") || q.contains("prefer"),
    }
}

fn session_focus_priority(intent: QueryIntent, query: &str) -> u8 {
    let q = query.to_lowercase();
    let explicit_session_intent = q.contains("recent")
        || q.contains("conversation")
        || q.contains("chat")
        || q.contains("session");
    let precision_guard = explicit_session_intent && !intent.wants_skill;
    if precision_guard { 1 } else { 2 }
}

fn intent_scopes(
    intent: QueryIntent,
    target: Option<&AxiomUri>,
    filter: Option<&SearchFilter>,
) -> Vec<Scope> {
    if let Some(target) = target {
        return vec![target.scope()];
    }

    if let Some(filter_scope) = filter_scope(filter) {
        return vec![filter_scope];
    }

    if intent.wants_skill {
        return vec![Scope::Agent];
    }
    if intent.wants_memory {
        return vec![Scope::User, Scope::Agent];
    }
    vec![Scope::Resources]
}

fn filter_scope(filter: Option<&SearchFilter>) -> Option<Scope> {
    let filter = filter?;
    if filter.start_time.is_some() || filter.end_time.is_some() {
        return Some(Scope::Events);
    }
    let kind = filter.kind.as_deref()?;
    if is_event_kind(kind) {
        return Some(Scope::Events);
    }
    if is_resource_kind(kind) {
        return Some(Scope::Resources);
    }
    None
}

fn is_event_kind(kind: &str) -> bool {
    matches!(
        Kind::new(kind.to_string()).ok().as_ref().map(Kind::as_str),
        Some("incident" | "run" | "deploy" | "log" | "trace")
    )
}

fn is_resource_kind(kind: &str) -> bool {
    matches!(
        Kind::new(kind.to_string()).ok().as_ref().map(Kind::as_str),
        Some("contract" | "adr" | "runbook" | "repository")
    )
}

fn dedup_and_limit_queries(mut planned: Vec<PlannedQuery>, max_len: usize) -> Vec<PlannedQuery> {
    planned.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then_with(|| a.kind.cmp(&b.kind))
            .then_with(|| a.query.cmp(&b.query))
    });

    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for item in planned {
        let key = (item.query.to_lowercase(), scope_signature(&item.scopes));
        if !seen.insert(key) {
            continue;
        }
        out.push(item);
        if out.len() >= max_len {
            break;
        }
    }

    if out.is_empty() {
        out.push(PlannedQuery::new(
            "primary",
            String::new(),
            vec![Scope::Resources],
            1,
            &SearchOptions {
                query: String::new(),
                target_uri: None,
                session: None,
                session_hints: Vec::new(),
                budget: None,
                limit: 0,
                score_threshold: None,
                min_match_tokens: None,
                filter: None,
                request_type: String::new(),
            },
        ));
    }

    out
}

fn scope_signature(scopes: &[Scope]) -> u8 {
    scopes
        .iter()
        .fold(0u8, |mask, scope| mask | scope_bit(*scope))
}

const fn scope_bit(scope: Scope) -> u8 {
    match scope {
        Scope::Resources => 1 << 0,
        Scope::User => 1 << 1,
        Scope::Agent => 1 << 2,
        Scope::Session => 1 << 3,
        Scope::Events => 1 << 4,
        Scope::Temp => 1 << 5,
        Scope::Queue => 1 << 6,
    }
}

fn normalize_scopes(scopes: Vec<Scope>) -> Vec<Scope> {
    let mut scopes = scopes;
    scopes.sort_by_key(Scope::as_str);
    scopes.dedup();
    scopes
}

pub(super) fn is_om_hint(text: &str) -> bool {
    text.trim_start()
        .get(..3)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("om:"))
}

fn merge_non_om_hints(hints: &[String]) -> String {
    let mut out = String::new();
    for hint in hints {
        if is_om_hint(hint) {
            continue;
        }
        let trimmed = hint.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(trimmed);
    }
    out
}

fn normalize_om_hint(text: &str) -> Option<String> {
    let trimmed = text.trim();
    let without_prefix = match trimmed.split_once(':') {
        Some((prefix, rest)) if prefix.trim().eq_ignore_ascii_case("om") => rest.trim(),
        _ => trimmed,
    };
    if without_prefix.is_empty() {
        None
    } else {
        Some(without_prefix.to_string())
    }
}

pub(super) fn collect_scope_names(planned_queries: &[PlannedQuery]) -> Vec<String> {
    let mut scopes = planned_queries
        .iter()
        .flat_map(|x| x.scopes.iter().copied())
        .collect::<Vec<_>>();
    scopes.sort_by_key(Scope::as_str);
    scopes.dedup();
    scopes
        .into_iter()
        .map(|scope| scope.as_str().to_string())
        .collect()
}

pub(super) fn uri_in_scopes(uri: &str, scopes: &[Scope]) -> bool {
    if scopes.is_empty() {
        return true;
    }
    let Some(scope_str) = get_scope_str_from_uri(uri) else {
        return false;
    };
    scopes.iter().any(|scope| scope.as_str() == scope_str)
}

fn get_scope_str_from_uri(uri: &str) -> Option<&str> {
    if !uri.starts_with("axiom://") {
        return None;
    }
    let tail = &uri[8..];
    tail.split('/').next()
}

#[cfg(test)]
mod tests {
    use super::{
        PlannedQuery, collect_scope_names, dedup_and_limit_queries, is_om_hint, merge_non_om_hints,
        normalize_scopes, plan_queries, query_intent,
    };
    use crate::models::{SearchFilter, SearchOptions};
    use crate::uri::Scope;

    fn test_options() -> SearchOptions {
        SearchOptions {
            query: String::new(),
            target_uri: None,
            session: None,
            session_hints: Vec::new(),
            budget: None,
            limit: 10,
            score_threshold: None,
            min_match_tokens: None,
            filter: None,
            request_type: "search".to_string(),
        }
    }

    #[test]
    fn normalize_scopes_is_value_based_and_sorted() {
        let scopes = normalize_scopes(vec![
            Scope::Resources,
            Scope::User,
            Scope::Resources,
            Scope::Agent,
        ]);
        assert_eq!(scopes, vec![Scope::Agent, Scope::Resources, Scope::User]);
    }

    #[test]
    fn dedup_queries_ignores_scope_order_after_normalization() {
        let queries = vec![
            PlannedQuery::new(
                "primary",
                "oauth flow".to_string(),
                vec![Scope::User, Scope::Resources],
                1,
                &test_options(),
            ),
            PlannedQuery::new(
                "primary",
                "OAUTH FLOW".to_string(),
                vec![Scope::Resources, Scope::User],
                1,
                &test_options(),
            ),
        ];
        let deduped = dedup_and_limit_queries(queries, 5);
        assert_eq!(deduped.len(), 1);
    }

    #[test]
    fn collect_scope_names_returns_sorted_distinct_names() {
        let planned = vec![
            PlannedQuery::new(
                "primary",
                "q".to_string(),
                vec![Scope::Resources, Scope::User],
                1,
                &test_options(),
            ),
            PlannedQuery::new(
                "secondary",
                "q2".to_string(),
                vec![Scope::Agent, Scope::User],
                2,
                &test_options(),
            ),
        ];
        let names = collect_scope_names(&planned);
        assert_eq!(
            names,
            vec![
                "agent".to_string(),
                "resources".to_string(),
                "user".to_string()
            ]
        );
    }

    #[test]
    fn query_intent_parses_skill_and_memory_flags() {
        let skill = query_intent("show skill docs");
        assert!(skill.wants_skill);
        assert!(!skill.wants_memory);

        let memory = query_intent("memory preference profile");
        assert!(!memory.wants_skill);
        assert!(memory.wants_memory);
    }

    #[test]
    fn is_om_hint_is_case_insensitive_and_trim_aware() {
        assert!(is_om_hint("om: hint"));
        assert!(is_om_hint("  Om: hint"));
        assert!(is_om_hint("\tOM: hint"));
        assert!(!is_om_hint("hint om: value"));
        assert!(!is_om_hint("memo: hint"));
    }

    #[test]
    fn merge_non_om_hints_skips_om_prefixed_entries() {
        let hints = vec![
            "recent one".to_string(),
            "  om: long memory".to_string(),
            " recent two ".to_string(),
            " ".to_string(),
        ];
        assert_eq!(merge_non_om_hints(&hints), "recent one recent two");
    }

    #[test]
    fn session_search_adds_session_focus_scope() {
        let options = SearchOptions {
            query: "oauth".to_string(),
            target_uri: None,
            session: Some("s-1".to_string()),
            session_hints: Vec::new(),
            budget: None,
            limit: 5,
            score_threshold: None,
            min_match_tokens: None,
            filter: None,
            request_type: "search".to_string(),
        };
        let planned = plan_queries(&options);
        assert!(planned.iter().any(|item| {
            item.kind == "session_focus"
                && item.scopes == vec![Scope::Session]
                && item.priority == 2
        }));
    }

    #[test]
    fn recent_chat_query_prefers_session_scope() {
        let options = SearchOptions {
            query: "recent chat summary".to_string(),
            target_uri: None,
            session: Some("s-2".to_string()),
            session_hints: Vec::new(),
            budget: None,
            limit: 5,
            score_threshold: None,
            min_match_tokens: None,
            filter: None,
            request_type: "search".to_string(),
        };
        let planned = plan_queries(&options);
        assert!(planned.iter().any(|item| {
            item.kind == "session_focus"
                && item.scopes == vec![Scope::Session]
                && item.priority == 1
        }));
    }

    #[test]
    fn session_priority_boost_keeps_precision_guardrails() {
        let options = SearchOptions {
            query: "session skill onboarding".to_string(),
            target_uri: None,
            session: Some("s-3".to_string()),
            session_hints: Vec::new(),
            budget: None,
            limit: 5,
            score_threshold: None,
            min_match_tokens: None,
            filter: None,
            request_type: "search".to_string(),
        };
        let planned = plan_queries(&options);
        assert!(planned.iter().any(|item| {
            item.kind == "session_focus"
                && item.scopes == vec![Scope::Session]
                && item.priority == 2
        }));
    }

    #[test]
    fn event_filter_without_target_switches_primary_scope_to_events() {
        let options = SearchOptions {
            query: "stale jwks refresh failures".to_string(),
            target_uri: None,
            session: Some("s-1".to_string()),
            session_hints: Vec::new(),
            budget: None,
            limit: 10,
            score_threshold: None,
            min_match_tokens: None,
            filter: Some(SearchFilter {
                tags: Vec::new(),
                mime: None,
                namespace_prefix: Some("acme/identity/prod".to_string()),
                kind: Some("incident".to_string()),
                start_time: Some(1_710_499_000),
                end_time: Some(1_710_501_000),
            }),
            request_type: "search".to_string(),
        };

        let planned = plan_queries(&options);
        assert_eq!(planned[0].kind, "primary");
        assert_eq!(planned[0].scopes, vec![Scope::Events]);
    }

    #[test]
    fn resource_filter_without_target_keeps_primary_scope_on_resources() {
        let options = SearchOptions {
            query: "oauth runbook".to_string(),
            target_uri: None,
            session: None,
            session_hints: Vec::new(),
            budget: None,
            limit: 10,
            score_threshold: None,
            min_match_tokens: None,
            filter: Some(SearchFilter {
                tags: Vec::new(),
                mime: None,
                namespace_prefix: Some("acme/identity".to_string()),
                kind: Some("runbook".to_string()),
                start_time: None,
                end_time: None,
            }),
            request_type: "search".to_string(),
        };

        let planned = plan_queries(&options);
        assert_eq!(planned[0].scopes, vec![Scope::Resources]);
    }
}
