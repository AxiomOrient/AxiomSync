# Retrieval Planner Rules

이 문서는 retrieval planner 가 어떤 신호로 primary scope 를 정하는지와, 최근 수정한 event-filter routing 버그의 원인을 고정한다.

## Scope Priority
planner 는 아래 순서로 primary scope 를 정한다.

1. `target_uri`
2. `SearchFilter`
3. query intent
4. default fallback

의미:
- `target_uri` 가 있으면 해당 URI scope 가 최우선이다.
- `target_uri` 가 없으면 `SearchFilter` 가 가장 강한 검색 의도 신호다.
- filter 도 없을 때만 query text 기반 intent 를 사용한다.
- 어떤 신호도 없으면 기본 scope 는 `resources` 다.

## Filter Routing Rules
현재 planner 는 아래 규칙으로 filter 를 scope 로 바꾼다.

- `start_time` 또는 `end_time` 이 있으면 `events`
- `kind=incident|run|deploy|log|trace` 이면 `events`
- `kind=contract|adr|runbook|repository` 이면 `resources`
- 그 외 kind 는 planner 가 scope 를 강제하지 않고 intent fallback 으로 내려간다

이 규칙은 `target_uri` 가 없을 때만 적용된다.

## Root Cause Of The Failed Case
문제였던 실패 케이스는 아래 조건이었다.

- `target_uri = None`
- query 는 일반 자연어
- `SearchFilter.kind = incident`
- `SearchFilter.start_time/end_time` 존재

이 경우 정상 동작은 `events` 검색이다. 하지만 이전 planner 는 `SearchFilter` 를 scope 결정에 사용하지 않았고, query text 만 보고 `resources` 를 primary scope 로 선택했다. 결과적으로 event projection 이 검색 후보에서 빠졌고, 데이터 적재는 정상이지만 retrieval 이 실패했다.

즉 원인은 테스트 데이터가 아니라 planner 의 scope inference 누락이었다.

## Correct Behavior
정상 케이스 예시는 아래와 같다.

| 입력 | 기대 primary scope |
|---|---|
| `target_uri=axiom://events/...` | `events` |
| `target_uri=None`, `kind=incident` | `events` |
| `target_uri=None`, `start_time/end_time` 있음 | `events` |
| `target_uri=None`, `kind=runbook` | `resources` |
| `target_uri=None`, query contains `skill` | `agent` |
| `target_uri=None`, query contains `memory/preference` | `user + agent` |
| 아무 신호 없음 | `resources` |

## Current Limits
- `namespace_prefix` 만으로는 아직 scope 를 강제하지 않는다.
- kind 분류는 현재 알려진 kind 집합에 의존한다.
- event/resource 혼합 의도 질의는 아직 weighted multi-scope planning 으로 풀지 않고 단일 primary scope 를 사용한다.

## Evidence
- planner regression: `cargo test -q -p axiomsync retrieval::planner::tests::event_filter_without_target_switches_primary_scope_to_events`
- realistic event flow: `cargo test -q -p axiomsync realistic_event_timeline_covers_full_fields_filters_search_and_archive_flow`
