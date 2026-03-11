# AxiomNexus

**Production-grade Context Management System for Agentic Runtimes**

AxiomNexus는 에이전트 환경을 위한 Rust 기반 로컬 컨텍스트 런타임입니다.
`axiom://` URI를 기준으로 파일, 상태, 검색, 세션 메모리를 일관된 계약으로 다룹니다.

## Overview
- Local-first: `fs + sqlite + in-memory index`
- Deterministic retrieval: `find/search` + trace metadata
- Session/OM memory flow: checkpointed promotion and replay-safe updates
- Release safety: contract/reliability/security gates

## Quick Start
```bash
# 초기화
cargo run -p axiomnexus-cli -- init

# 리소스 추가
cargo run -p axiomnexus-cli -- add ./docs --target axiom://resources/docs

# 검색
cargo run -p axiomnexus-cli -- search "oauth flow" --target axiom://resources/docs
```

## Install / Run
```bash
# CLI 도움말
cargo run -p axiomnexus-cli -- --help

# 로컬 binary 설치(선택)
cargo install --path crates/axiomnexus-cli
```

## 실제 사용 모델 (무엇을 어디에 저장할까?)
| Scope | 저장 대상 | 비고 |
| --- | --- | --- |
| `axiom://resources/...` | 프로젝트 문서, 코드 스냅샷, 정책 문서, 운영 기록 | 기본 지식베이스(검색의 중심) |
| `axiom://user/memories/...` | 사용자 장기 선호/프로필/이벤트 | 세션 종료 후에도 유지 |
| `axiom://agent/memories/...` | 에이전트 패턴, 사례, 재사용 규칙 | 에이전트 동작 기준 |
| `axiom://session/<id>/...` | 현재 작업 타임라인(메시지/도구 이벤트/중간 판단) | 단기 작업 메모리 |
| `axiom://temp`, `axiom://queue` | 시스템 내부 파이프라인 데이터 | `queue`는 비시스템 쓰기 금지 |

## 세션에 대화 외 데이터 저장 가능한가?
가능합니다. `session add`는 `role`을 자유롭게 받아 대화뿐 아니라 도구 실행 결과, 작업 메모, 운영 이벤트를 같은 타임라인으로 저장할 수 있습니다.

```bash
# 1) 세션 생성
SID="$(cargo run -q -p axiomnexus-cli -- session create)"

# 2) 대화 메시지
cargo run -p axiomnexus-cli -- session add --id "$SID" --role user --text "배포 전 검증 시작"

# 3) 대화 외 이벤트(예: 테스트 결과)
cargo run -p axiomnexus-cli -- session add --id "$SID" --role tool --text "integration: 24 passed, 1 flaky"

# 4) 세션 문맥을 검색에 반영
cargo run -p axiomnexus-cli -- search "flaky 원인" --session "$SID"

# 5) 세션 커밋(아카이브 + memory 추출 흐름)
cargo run -p axiomnexus-cli -- session commit --id "$SID"
```

## 프로젝트가 여러 개인 경우 리소스 운영 최선안
권장 원칙은 단순합니다. 교차 검색이 필요하면 루트를 공유하고 URI로 프로젝트를 분리하고, 완전 격리가 필요하면 루트를 물리적으로 분리합니다.

1. 공유 루트 + 프로젝트 네임스페이스(교차 검색 최적)
```bash
ROOT="$HOME/.axiomnexus-workspace"
cargo run -p axiomnexus-cli -- --root "$ROOT" init
cargo run -p axiomnexus-cli -- --root "$ROOT" add ~/work/proj-a/docs --target axiom://resources/projects/proj-a/docs
cargo run -p axiomnexus-cli -- --root "$ROOT" add ~/work/proj-b/docs --target axiom://resources/projects/proj-b/docs
cargo run -p axiomnexus-cli -- --root "$ROOT" search "auth timeout" --target axiom://resources/projects/proj-a
```

2. 프로젝트별 루트 분리(격리/권한/운영 분리 최적)
```bash
cargo run -p axiomnexus-cli -- --root ~/work/proj-a/.axiomnexus init
cargo run -p axiomnexus-cli -- --root ~/work/proj-a/.axiomnexus add ~/work/proj-a/docs --target axiom://resources/docs

cargo run -p axiomnexus-cli -- --root ~/work/proj-b/.axiomnexus init
cargo run -p axiomnexus-cli -- --root ~/work/proj-b/.axiomnexus add ~/work/proj-b/docs --target axiom://resources/docs
```

실무 기준:
- 보안/규정 분리가 중요하면 `프로젝트별 루트 분리`.
- 팀 지식 재사용과 횡단 검색이 중요하면 `공유 루트 + projects/<name>` 구조.
- 세션은 단기 실행 로그로 쓰고, 재사용 가치가 생긴 사실만 `user/agent memories`로 승격.

## Core Modules
- [crates/axiomnexus-core](./crates/axiomnexus-core/README.md): runtime/data engine
- [crates/axiomnexus-cli](./crates/axiomnexus-cli/README.md): CLI surface
- [crates/axiomnexus-mobile-ffi](./crates/axiomnexus-mobile-ffi/README.md): mobile FFI boundary

## Repository Structure
- [crates/README.md](./crates/README.md): crate index
- [docs/README.md](./docs/README.md): canonical docs index

## Guides
- [Architecture](./docs/ARCHITECTURE.md)
- [API Contract](./docs/API_CONTRACT.md)
- [Ontology Evolution Policy](./docs/ONTOLOGY_SCHEMA_EVOLUTION_POLICY.md)
- [Usage Playbook](./docs/USAGE_PLAYBOOK.md)

## Operations/Quality
```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo audit -q
```

## Constraints
- canonical URI protocol: `axiom://`
- runtime code and pure OM contract boundary must remain explicit (`axiomnexus-core` vs `episodic`)
