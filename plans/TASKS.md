# Application Service Refactor Tasks

| ID | Goal | Scope | Done When | Evidence / Verification | Depends On |
|---|---|---|---|---|---|
| AS-00 | Freeze external contract | `docs/`, `src/commands`, `src/models` | public CLI/API/JSON invariants가 문서로 고정됨 | `docs/APPLICATION_SERVICE_ROADMAP.md`, `docs/APPLICATION_SERVICE_TEST_STRATEGY.md` | - |
| AS-01 | Extract `ReleaseVerificationService` pure snapshot layer | `src/client/release/verify_service.rs` | inspect/apply/verify read-model 조립이 pure planning/snapshot 구조체로 분리됨 | new pure tests + `cargo test -p axiomsync` | AS-00 |
| AS-02 | Make release verify facade path delegate-only | `src/client/facade.rs`, `src/client/release/*`, `src/commands/handlers.rs` | handler -> facade -> service 흐름만 남고 verify logic가 facade 밖에 있음 | command tests + release fixture tests | AS-01 |
| AS-03 | Extract `RuntimeBootstrapService` decision layer | `src/client.rs`, `src/client/runtime.rs` | bootstrap/restore/reindex/repair 판단이 pure decision struct로 분리됨 | runtime pure tests + lifecycle tests | AS-02 |
| AS-04 | Reduce `client.rs` to composition root | `src/client.rs`, `src/client/facade.rs` | `client.rs`에 wiring/root state 외 정책 로직이 남지 않음 | code review + `cargo clippy --workspace --all-targets -- -D warnings` | AS-03 |
| AS-05 | Extract `ResourceService` intent/plan layer | `src/client/resource.rs` | target resolve, wait strategy, finalize mode가 pure planning 단계로 이동 | resource tests + `cargo test -p axiomsync` | AS-04 |
| AS-06 | Make resource add flow delegate-only | `src/client/facade.rs`, `src/client/resource.rs` | public add path가 facade delegate와 service execute만 가짐 | add/search visibility regression tests | AS-05 |
| AS-07 | Extract `SearchService` request/hint/trace planning | `src/client/search/*`, `src/retrieval/trace.rs` | request normalization, hint layering, trace context가 pure plan struct로 정리됨 | search pure tests + backend tests | AS-06 |
| AS-08 | Close search regression matrix | `src/client/search/*`, `tests/repository_markdown_user_flows.rs`, `tests/process_contract.rs` | mixed intent, FTS fallback, real-doc operator flow가 유지됨 | `cargo test -p axiomsync` | AS-07 |
| AS-09 | Extract `SessionService` orchestration plans | `src/client/runtime.rs`, `src/session/*`와 맞닿는 client orchestration | session scope/promotion/delete/archive-only 판단이 plan struct로 분리됨 | session tests + search/session interaction tests | AS-08 |
| AS-10 | Make session-facing client paths delegate-only | `src/client/facade.rs`, 관련 `src/client/*` | session 관련 public path가 service delegate만 남음 | `cargo clippy --workspace --all-targets -- -D warnings` + `cargo test -p axiomsync` | AS-09 |
| AS-11 | Refresh docs to final structure only | `docs/BLUEPRINT.md`, `docs/IMPLEMENTATION_SPEC.md`, `docs/INDEX.md`, ownership docs | 문서가 최종 explicit service 구조를 현재형으로 설명함 | docs review | AS-10 |
| AS-12 | Final release gate | repo | full refactor 상태에서 최종 gate 전부 통과 | `cargo audit --deny unsound --deny unmaintained --deny yanked`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p axiomsync` | AS-11 |
