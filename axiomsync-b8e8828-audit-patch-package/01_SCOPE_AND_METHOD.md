# 범위와 방법

## 범위

이번 검토는 사용자가 준 commit URL을 직접 fetch해 byte-for-byte 비교한 감사는 아니다.  
이 환경에서는 commit page 직접 fetch가 되지 않았고, 따라서 **현재 공개 tree에서 fetch 가능한 실제 구현 파일**을 기준으로 감사를 수행했다.

## 실제로 본 파일

### 루트
- `README.md`
- `Cargo.toml`
- `crates/README.md`

### domain / store / kernel / mcp
- `crates/axiomsync-domain/src/domain.rs`
- `crates/axiomsync-domain/src/error.rs`
- `crates/axiomsync-store-sqlite/src/schema.sql`
- `crates/axiomsync-store-sqlite/src/context_db.rs`
- `crates/axiomsync-kernel/src/kernel.rs`
- `crates/axiomsync-kernel/src/logic.rs`
- `crates/axiomsync-kernel/src/ports.rs`
- `crates/axiomsync-mcp/src/mcp.rs`

### app shell
- `crates/axiomsync/src/lib.rs`
- `crates/axiomsync/src/main.rs`
- `crates/axiomsync/src/command_line.rs`
- `crates/axiomsync/src/connector_config.rs`
- `crates/axiomsync/src/connectors.rs`
- `crates/axiomsync/src/http_api.rs`
- `crates/axiomsync/src/web_ui.rs`
- `crates/axiomsync/Cargo.toml`

## 하지 않은 것

- 로컬 빌드 / 테스트 / clippy
- exact commit tree checkout 검증
- fetch 불가 파일에 대한 추정

## 평가 기준

### 최종형 정의
AxiomSync는 다음만 정확히 해야 한다.

- immutable raw event 저장
- canonical conversation projection
- episode / insight / verification derivation
- evidence anchor
- runbook / thread / evidence retrieval
- MCP and thin query surface
- replay / purge / repair / doctor

### AxiomSync가 하면 안 되는 것
- connector polling
- connector watch daemon
- connector-specific HTTP ingest server
- spool / retry / approval
- browser integration
- edge delivery policy
- connector config ownership

## 핵심 질문

1. 현재 커널 코어는 충분한가?
2. release surface가 실제로 좁은 sink contract인가?
3. implementation이 previous boundary decision을 실제로 반영했는가?
4. 지금 필요한 변경이 스키마 확장인지, 경계 축소인지?
