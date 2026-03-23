# 구현 파일별 감사

## 1. 루트 `README.md`

### 좋은 점
- repository boundary를 분명히 적었다.
- edge capture / spool / retry / approval / browser integration이 외부 repo 책임이라고 선언한다.
- `sink` surface를 canonical write boundary로 선언한다.

### 문제
- 선언된 CLI/HTTP surface와 실제 구현이 아직 다르다.
- 선언은 최신인데 shipping code는 아직 old connector runtime surface를 쓴다.

### 판정
문서 방향은 맞다. **실구현이 아직 못 따라왔다.**

---

## 2. 루트 `Cargo.toml`

### 좋은 점
- workspace split은 실제로 존재한다.
- `axiomsync-domain`, `store-sqlite`, `kernel`, `http`, `mcp`, `cli`, `axiomsync`로 분해돼 있다.

### 문제
- split이 존재하지만 실제 entrypoint가 그 split을 끝까지 사용하고 있지 않다.

### 판정
구조는 좋아졌다. **배선이 덜 끝났다.**

---

## 3. `crates/README.md`

### 좋은 점
- package map이 분명하다.
- `axiomsync-http`, `axiomsync-cli`의 존재를 명시한다.

### 문제
- 실제 shipping binary는 아직 `crates/axiomsync/src/main.rs -> axiomsync::command_line`로 간다.
- 따라서 package map과 runtime entrypoint가 아직 일치하지 않는다.

### 판정
설명은 맞는 방향. **entrypoint migration 미완료.**

---

## 4. `crates/axiomsync-domain/src/domain.rs`

### 좋은 점
- `RawEventInput`, `RawEventRow`, `IngestPlan`, `ConvSessionRow`, `ConvTurnRow`, `ConvItemRow`, `EpisodeRow`, `InsightRow`, `VerificationRow`, `RunbookRecord`가 잘 정리돼 있다.
- deterministic stable ID와 validation model이 있다.
- `CursorInput`이 있어 ingest batch에 cursor를 동반시킬 수 있다.

### 문제
- `source_cursor`는 ingest batch 내부 옵션일 뿐, 독립적인 plan/apply contract가 아니다.
- public surface 기준으로는 `SourceCursorUpsertPlan`이 1급 시민이어야 한다.
- 저장 모델은 conversation-native로 충분히 좋지만, public noun은 아직 `episode`/`runbook` 중심이다.

### 판정
domain core는 강하다. **cursor contract 분리만 추가하면 된다.**

---

## 5. `crates/axiomsync-store-sqlite/src/schema.sql`

### 좋은 점
- 모델이 작고 선명하다.
- `raw_event -> conv_session/turn/item -> artifact/evidence_anchor -> episode/insight/verification -> search_doc_redacted` 흐름이 명확하다.
- evidence anchor와 episode derivation이 잘 분리돼 있다.

### 문제
- schema 자체보다 public contract가 더 문제다.
- `source_cursor`는 있는데, source cursor만 독립적으로 upsert하는 transaction contract가 드러나지 않는다.
- `search_doc_redacted`는 있는데 document retrieval contract는 없다.

### 판정
스키마는 유지해도 된다. **확장보다 surface 정리가 우선이다.**

---

## 6. `crates/axiomsync-store-sqlite/src/context_db.rs`

### 좋은 점
- transactional apply 구조가 좋다.
- `apply_ingest_tx`, `apply_projection_tx`, `apply_derivation_tx`, `apply_replay_tx`, `apply_purge_tx`, `apply_repair_tx`가 명확하다.
- doctor report가 drift/FTS mismatch를 잡는다.

### 문제
- `source_cursor` upsert는 `apply_ingest_tx` 내부 side effect다.
- cursor만 별도로 plan/apply할 수 없다.
- `search_doc_redacted`는 저장되지만 query contract에서 거의 쓰이지 않는다.

### 판정
store adapter는 좋다. **cursor tx를 1개 더 분리하면 완성도가 크게 오른다.**

---

## 7. `crates/axiomsync-kernel/src/logic.rs`

### 좋은 점
- `normalize_raw_event`, dedupe, projection, derivation, runbook synthesis의 흐름이 좋다.
- ingest 계획 생성이 deterministic하다.

### 문제
- cursor update가 `input.events.first()`에 의존한다.
- 즉 cursor upsert가 독립 operation이 아니라 “ingest batch가 있을 때 덤으로” 처리된다.
- 이는 narrow sink contract 기준으로 좋지 않다.

### 판정
logic은 좋다. **cursor를 batch ingestion에서 분리해야 한다.**

---

## 8. `crates/axiomsync-kernel/src/kernel.rs`

### 좋은 점
- AxiomSync application service가 plan/apply 중심으로 정리돼 있다.
- query surface(`search_episodes`, `get_runbook`, `get_thread`, `get_evidence`, `search_commands`)는 실제 저장 모델과 연결돼 있다.
- doctor / purge / repair / replay가 있다.

### 문제
- `AxiomSync`가 `connectors` port를 직접 가진다.
- `load_connectors_config`, `connectors_path`, `connector_status`가 kernel에 있다.
- 이것은 external edge/runtime concern이다.
- public final form 기준으로 kernel은 raw sink + query만 소유해야 한다.

### 판정
kernel service는 강하다. **connector ownership만 제거하면 훨씬 좋아진다.**

---

## 9. `crates/axiomsync-kernel/src/ports.rs`

### 좋은 점
- repository/read/write/transaction/MCP/LLM/clock/hash port 분리가 깔끔하다.

### 문제
- `ConnectorConfigPort`와 `AuthStorePort`가 같은 레벨에 놓여 있다.
- auth는 query authorization과 직접 연결되므로 내부에 남길 수 있어도, connector config는 남기면 안 된다.
- `TransactionManager`에 `apply_source_cursor_tx`가 없다.

### 판정
ports는 거의 좋다. **ConnectorConfigPort 제거 + source cursor tx 추가가 핵심.**

---

## 10. `crates/axiomsync-mcp/src/mcp.rs`

### 좋은 점
- thin adapter다.
- MCP transport 구현을 kernel 밖에 두지 않고 adapter로 유지한 점은 좋다.

### 문제
- 실제 tool set 정의는 kernel 구현에 걸려 있다.
- canonical alias 정리(`case` vs `episode`)가 아직 덜 됐다.

### 판정
adapter 자체는 문제 없다. **tool naming 정리만 필요하다.**

---

## 11. `crates/axiomsync/src/lib.rs`

### 좋은 점
- composition root 역할을 한다.
- domain/kernel/store/mcp re-export가 있다.

### 문제
- `command_line`, `connectors`, `http_api`, `web_ui`, `connector_config`를 여전히 같은 crate 안에 직접 들고 있다.
- `open_with_llm()`가 `FileConnectorConfigStore`를 kernel에 주입한다.
- 즉 external edge concern이 open path에 침투한다.

### 판정
여기가 가장 큰 경계 누수 지점이다.

---

## 12. `crates/axiomsync/src/main.rs`

### 좋은 점
- 매우 작다.

### 문제
- 여전히 `axiomsync::command_line::{Cli, run}`에 직접 의존한다.
- workspace에 별도 `axiomsync-cli`가 있어도 실제 entrypoint migration이 완료되지 않았다.

### 판정
얇지만 old path를 유지한다.

---

## 13. `crates/axiomsync/src/command_line.rs`

### 좋은 점
- 기존 기능은 다 노출한다.
- init / project / derive / search / runbook / mcp / web surface가 존재한다.

### 문제
- 여전히 top-level `Connector` command와 `Ingest/Sync/Repair/Watch/Serve`를 노출한다.
- 이는 separate external runtime이 해야 할 일이다.
- root README의 `sink plan-append-raw-events` / `sink apply-ingest-plan` / `sink plan-upsert-source-cursor` / `sink apply-source-cursor-plan`와 불일치한다.

### 판정
**가장 먼저 바꿔야 한다.**

---

## 14. `crates/axiomsync/src/connector_config.rs`

### 좋은 점
- 파일 하나로 단순하다.

### 문제
- 이 파일 자체가 현재 repo boundary와 충돌한다.
- `connectors.toml`는 external collector/runtime의 concern이다.

### 판정
외부 repo로 이동해야 한다.

---

## 15. `crates/axiomsync/src/connectors.rs`

### 좋은 점
- 현재로서는 여러 connector ingestion을 일관된 batch model로 파싱한다.

### 문제
- `sync`, `watch`, `repair`, `serve`가 여기에 들어 있다.
- codex remote poll, gemini watch, chatgpt/claude serve가 전부 들어 있다.
- 이것은 AxiomSync의 본질이 아니다.

### 판정
**전체를 외부 runtime으로 추출해야 한다.**

---

## 16. `crates/axiomsync/src/http_api.rs`

### 좋은 점
- web/query/MCP/ingest를 하나의 server로 묶는 구조 자체는 단순하다.

### 문제
- route가 아직 `/ingest/{connector}`, `/project`, `/derive`, `/connectors` 중심이다.
- `connector_ingest_router`도 여전히 존재한다.
- root README가 선언한 `/sink/raw-events/plan`, `/sink/raw-events/apply`, `/sink/source-cursors/plan`, `/sink/source-cursors/apply`와 다르다.

### 판정
**public HTTP surface도 최종형이 아니다.**

---

## 17. `crates/axiomsync/src/web_ui.rs`

### 좋은 점
- 단순하고 작다.
- runbook view 자체는 유용하다.

### 문제
- 제목이 아직 `AxiomSync Renewal Kernel`이다.
- `/connectors` 페이지를 가진다.
- UI도 release boundary 축소를 따라가지 못했다.

### 판정
UI는 단순화가 필요하다.
