# AxiomSync Final Form Master Document

> 이 문서는 패키지 내 핵심 문서를 한 파일로 합친 버전이다.

# 00. Executive Summary

## 최종 판정

가장 단순하고 강한 최종형은 아래다.

- **AxiomSync = local-first conversation-native knowledge kernel**
- **AxiomRelay = capture / spool / forward service**
- **axiomRams = file-first execution runtime**

## AxiomSync가 해야 할 핵심만 남기면

1. raw event ledger 저장
2. canonical `session / entry / artifact / anchor` projection
3. `episode / insight / verification / procedure` 파생
4. evidence-backed retrieval
5. read-only query surface (CLI / HTTP / MCP)
6. replay / rebuild / repair / migration

## AxiomSync가 하면 안 되는 것

- connector polling / watch / sync
- browser extension ownership
- spool / retry / dead-letter
- approval queue
- execution orchestration
- Rams run state canonical ownership
- product UI / service branding

## 왜 이 구조가 맞나

`conv_*`만으로 가면 Rams가 왜곡되고, `run_*`만으로 가면 Relay가 왜곡된다.  
그래서 storage core는 generic하게 두고, query semantics는 conversation/episode 중심으로 유지해야 한다.

## 세 시스템의 연결

```text
AxiomRelay ---- append_raw_events ----> AxiomSync <---- append_raw_events ---- axiomRams
    |                                         |
    |                                         +---- MCP / HTTP / CLI query
    +---- capture / spool / approval
```

## 정본 분리

- AxiomSync → `context.db`
- AxiomRelay → capture/spool state
- axiomRams → `state/` files

direct DB coupling은 금지한다.

## 먼저 읽을 파일

1. `01_FINAL_FORM.md`
2. `03_STORAGE_SCHEMA.md`
3. `04_API_AND_MCP_SPEC.md`
4. `05_INTEGRATION_AXIOMRELAY_AXIOMRAMS.md`

---

# 01. AxiomSync 최종형 정의

## 1) 최종 정의

AxiomSync는 **지식 커널**이다.  
제품 edge, 실행 orchestration, capture runtime, approval queue, connector polling은 하지 않는다.

AxiomSync의 일은 정확히 5가지다.

1. **raw event ledger 저장**
2. **canonical projection 유지**
3. **episode / insight / verification / procedure 파생**
4. **evidence anchor 기반 재사용**
5. **read-only query surface 제공**

## 2) 왜 이것이 최소 핵심인가

AxiomRelay와 axiomRams는 서로 다르다.

- **AxiomRelay**는 capture / spool / forward 시스템이다.
- **axiomRams**는 file-first execution runtime이다.

둘을 하나의 범용 커널 위에 연결하려면, 커널 중심은 `conversation-only`도 `run-only`도 아니어야 한다.  
따라서 canonical 중심은 아래 4개로 잡는다.

- `session`
- `entry`
- `artifact`
- `anchor`

이 4개는 두 시스템 모두를 왜곡 없이 담을 수 있다.

## 3) AxiomSync가 반드시 소유할 것

| 영역 | 소유 내용 |
|---|---|
| Raw truth | immutable raw events, dedupe receipt, source cursor |
| Canonical core | sessions, actors, entries, artifacts, anchors |
| Reusable memory | episodes, insights, verifications, procedures |
| Retrieval | search docs / FTS / optional embeddings |
| Query surface | CLI, HTTP, MCP read tools/resources |
| Maintenance | replay, rebuild, purge, repair, migration |

## 4) AxiomSync가 소유하면 안 되는 것

| 금지 영역 | 이유 |
|---|---|
| connector polling / watch / sync | 제품 edge 책임이다 |
| browser capture / extension runtime | Relay 책임이다 |
| pending/sent/dead-letter spool | Relay 책임이다 |
| approval queue | Relay 또는 Rams 책임이다 |
| operator task board / run state | Rams 책임이다 |
| ChatGPT / Claude / Codex auth refresh | 제품 adapter 책임이다 |
| execution orchestration | Rams 책임이다 |
| service UI / branding | 커널 본질이 아니다 |

## 5) conversation-native 이면서 generic 이어야 하는 이유

AxiomSync는 conversation-native여야 한다.  
왜냐하면 ChatGPT, Claude, Codex, Gemini, agent transcript는 모두 `대화/세션` 질의가 핵심이기 때문이다.

하지만 core schema를 `conv_*`로 고정하면 axiomRams의 run/task/check/evidence를 억지로 conversation으로 넣게 된다.  
그래서 다음처럼 나눈다.

- **storage core**: `session / entry / artifact / anchor`
- **primary public semantics**: conversation / episode / insight / procedure

즉, 저장 구조는 generic, 질의 경험은 conversation-native로 유지한다.

## 6) Derived memory의 최종 형태

### `episode`
재사용 가능한 작업 단위.
예:
- 문제 조사
- 버그 수정
- 설계 결정
- 실험 결과
- 실행 run 요약

### `insight`
evidence-backed distilled statement.
예:
- root cause
- fix summary
- decision
- invariant
- warning
- preference

### `verification`
insight 또는 procedure가 얼마나 믿을 수 있는지 나타내는 검증 기록.
예:
- deterministic check passed
- human confirmed
- stale
- conflicted
- superseded

### `procedure`
재사용 가능한 how-to / runbook.
반드시 evidence 또는 verified insight에 연결된다.

## 7) 최종 non-goals

아래는 최종형에서도 하지 않는다.

- general chat UI
- product-specific ranking policy
- autonomous planning system
- cross-device sync product
- external side-effect execution
- browser automation primary capture
- semantic magic without evidence

## 8) 최종 판정 문장

> **AxiomSync는 “모든 것을 하는 앱”이 아니라, evidence-backed memory를 만드는 local knowledge kernel이다.**

---

# 02. 청사진 / 구조도

## 1) 전체 구조

```text
[Relay for ChatGPT] --selected capture--> [AxiomRelay / relayd]
[Codex/Claude adapters] -----------------> [AxiomRelay / relayd]
                                           | spool / retry / approval
                                           v
                                   append_raw_events(batch)
                                           v
                                      [AxiomSync]
                              raw ledger -> canonical -> derived
                                           |
                                           +--> CLI maintenance
                                           +--> HTTP query
                                           +--> MCP read tools/resources

[axiomRams runtime] --run evidence/export--> append_raw_events(batch)
[axiomRams operator] -----------------------> MCP / HTTP query
```

## 2) 세 프로젝트의 경계

### AxiomSync
- `context.db` 소유
- raw truth 소유
- canonical projection 소유
- reusable memory 소유
- query semantics 소유

### AxiomRelay
- capture edge 소유
- local spool 소유
- retry / dead-letter 소유
- approval queue 소유
- kernel forwarding 소유

### axiomRams
- `program/` + `state/` 파일 정본 소유
- runtime loop 소유
- approvals for execution side effects 소유
- deterministic verification state 소유
- AxiomSync import/export adapter 소유

## 3) 연결 방식

### ingest
우선순위:
1. 같은 프로세스면 library call
2. 같은 머신이면 Unix socket
3. 그 외는 local HTTP

### query
우선순위:
1. 에이전트/도구 재사용은 MCP
2. 로컬 디버그/운영은 CLI
3. 대시보드/내부 UI는 HTTP

## 4) repo 구조 제안

### AxiomSync repo

```text
crates/
  axiomsync-domain/
  axiomsync-kernel/
  axiomsync-store-sqlite/
  axiomsync-http/
  axiomsync-mcp/
  axiomsync-cli/
docs/
schema/
```

### AxiomRelay repo

```text
apps/
  relayd/
extensions/
  relay-for-chatgpt/
workers/
  relay-repair-worker/
config/
```

### axiomRams repo

```text
src/
  runtime/
  registry/
  verifier/
  state/
  api/
program/
state/
schemas/
adapters/
  axiomsync/
```

## 5) write ownership

| 시스템 | 정본 | 직접 write 허용 |
|---|---|---|
| AxiomSync | `context.db` | AxiomSync 내부만 |
| AxiomRelay | spool / queue state | AxiomRelay 내부만 |
| axiomRams | `state/` files | axiomRams 내부만 |

절대 금지:
- Relay가 AxiomSync DB 직접 쓰기
- Rams가 AxiomSync DB 직접 쓰기
- AxiomSync가 Rams run state 직접 수정하기

## 6) kernel mode

AxiomSync는 세 가지 mode를 지원하면 충분하다.

### library mode
- Rust process 내부에서 직접 호출
- axiomRams에 가장 자연스럽다

### local service mode
- sibling process로 동작
- AxiomRelay에 가장 자연스럽다

### maintenance mode
- rebuild / repair / inspect 용 CLI

## 7) 핵심 설계 선택

### 선택 1 — `session` 중심
`conv_*`만으로 가지 않는다.  
그렇다고 run-only로도 가지 않는다.

### 선택 2 — raw와 derived를 분리
raw event는 불변.
episode/insight/procedure는 재생성 가능.

### 선택 3 — evidence 없으면 재사용 금지
모든 reusable knowledge는 최소 1개 이상 anchor를 가져야 한다.

### 선택 4 — query는 read-only
ingest path와 query path를 분리한다.

### 선택 5 — retrieval index는 disposable
FTS/embeddings는 파생물이다.
정본은 raw + canonical + derived memory다.

---

# 03. 저장 모델 / 스키마

## 1) 레이어

```text
L1 raw ledger
  raw_events
  source_cursors

L2 canonical projection
  sessions
  actors
  entries
  artifacts
  anchors

L3 derived memory
  episodes
  insights
  verifications
  procedures

L4 retrieval index
  search_docs
  search_docs_fts
  optional embeddings
```

## 2) raw ledger

### `raw_events`
역할:
- 들어온 packet을 불변으로 보관
- dedupe의 기준점
- replay의 출발점

필수 필드:
- `raw_event_id`
- `source_kind`
- `connector_name`
- `native_session_id`
- `native_entry_id`
- `event_type`
- `captured_at_ms`
- `observed_at_ms`
- `content_hash`
- `dedupe_key`
- `raw_payload_json`
- `normalized_json`
- `projection_state`

규칙:
- update 금지
- delete는 purge 정책으로만
- 동일 `(connector_name, dedupe_key)`는 1회만 accept

### `source_cursors`
역할:
- connector import 진행 위치
- poll/watch 재시작 위치

필수 필드:
- `connector_name`
- `cursor_key`
- `cursor_value`
- `observed_at_ms`
- `metadata_json`

## 3) canonical projection

### `sessions`
가장 작은 상위 container.

예:
- ChatGPT conversation
- Claude thread
- Codex session
- axiomRams run
- manual import batch

핵심 필드:
- `session_id`
- `session_kind` (`conversation`, `run`, `task`, `import`)
- `stable_key`
- `title`
- `workspace_root`
- `started_at_ms`
- `ended_at_ms`
- `metadata_json`

### `actors`
발화/행동 주체.
예:
- user
- assistant
- tool
- verifier
- operator
- runtime

### `entries`
session 안의 ordered unit.

예:
- message
- tool_call
- tool_result
- run_step
- check_result
- approval_note
- manual_note

핵심 필드:
- `entry_id`
- `session_id`
- `seq_no`
- `entry_kind`
- `actor_id`
- `parent_entry_id`
- `stable_key`
- `text_body`
- `started_at_ms`
- `ended_at_ms`
- `metadata_json`

### `artifacts`
entry에 매달린 결과물.
예:
- file
- diff
- screenshot
- log
- URL
- code block

### `anchors`
정확한 evidence pointer.
예:
- text span
- line range
- DOM selection
- timestamp range
- JSON pointer

핵심 규칙:
- anchor는 `entry` 또는 `artifact` 중 하나 이상을 가리켜야 한다
- retrieval answer는 가능하면 anchor를 같이 반환해야 한다

## 4) derived memory

### `episodes`
여러 entry/anchor를 묶은 reusable unit.

권장 `episode_kind`:
- `problem`
- `investigation`
- `fix`
- `decision`
- `procedure_run`
- `summary`

### `insights`
반복 재사용 가능한 distilled statement.

권장 `insight_kind`:
- `root_cause`
- `fix`
- `decision`
- `invariant`
- `warning`
- `preference`

필수 규칙:
- `statement`는 짧고 단정적으로 쓴다
- 최소 1개 anchor 필요
- confidence는 evidence 개수와 verification 상태를 반영한다

### `verifications`
subject는 `episode`, `insight`, `procedure` 중 하나.

권장 `status`:
- `proposed`
- `verified`
- `conflicted`
- `stale`
- `superseded`

권장 `method`:
- `deterministic`
- `human`
- `heuristic`

### `procedures`
재사용 가능한 runbook.

구성:
- 제목
- 목적
- 전제조건
- step list
- 예상 결과
- evidence anchors
- optional verification status

## 5) retrieval index

### `search_docs`
검색 단위.
`entry`, `episode`, `insight`, `procedure`를 각각 문서화해 저장.

### `search_docs_fts`
SQLite FTS index.
정본이 아니므로 drop/rebuild 가능해야 한다.

### optional `embedding_index`
있어도 좋지만 core가 아니다.
항상 `search_docs`에서 재구성 가능해야 한다.

## 6) 핵심 불변식

1. raw event는 immutable
2. AxiomSync 외부는 DB 직접 write 금지
3. derived record는 evidence anchor를 최소 1개 가져야 함
4. verification은 subject를 덮어쓰지 않고 별도 기록으로 남음
5. retrieval index는 전부 rebuild 가능해야 함
6. `session -> entry -> anchor` 경로 없이 reusable knowledge를 만들지 않음

## 7) stable key 전략

| 엔터티 | stable key 예시 |
|---|---|
| session | `chatgpt:<conversation_id>` / `rams:run:<run_id>` |
| entry | `<session_key>:<native_entry_id>` 또는 deterministic hash |
| artifact | `<session_key>:<uri>:<sha256>` |
| anchor | `<entry_or_artifact_key>:<fingerprint>` |
| episode | deterministic hash over anchor set + kind |
| insight | deterministic hash over normalized statement + scope |
| procedure | deterministic hash over title + normalized steps |

## 8) 권장 SQL skeleton

구체 테이블 초안은 `schema/axiomsync_kernel_vnext.sql`에 포함했다.

---

# 04. API / MCP 스펙

## 1) ingest contract

AxiomSync의 write API는 아래 3개면 충분하다.

1. `append_raw_events(batch)`
2. `upsert_source_cursor(cursor)`
3. `health()`

선택 operator API:
- `rebuild_projection(scope)`
- `rebuild_derived(scope)`
- `rebuild_index(scope)`

## 2) `append_raw_events(batch)`

### request

```json
{
  "batch_id": "relay-2026-03-23T12:00:00Z-001",
  "source": {
    "source_kind": "axiomrelay",
    "connector_name": "chatgpt_web_selection"
  },
  "events": [
    {
      "raw_event_id": "optional-client-id",
      "native_session_id": "chatgpt:abc123",
      "native_entry_id": "msg_42",
      "event_type": "selection_captured",
      "captured_at_ms": 1710000000000,
      "observed_at_ms": 1710000000123,
      "dedupe_key": "chatgpt:abc123:msg_42:fp_001",
      "content_hash": "sha256:...",
      "payload": {},
      "hints": {}
    }
  ]
}
```

### behavior

- schema validate
- idempotency check
- raw ledger append
- projection queue mark
- `accepted[]` / `rejected[]` 반환

### response

```json
{
  "batch_id": "relay-2026-03-23T12:00:00Z-001",
  "accepted": ["rawevt_1"],
  "rejected": [
    {
      "dedupe_key": "chatgpt:abc123:msg_42:fp_001",
      "reason": "duplicate"
    }
  ],
  "stats": {
    "accepted_count": 1,
    "rejected_count": 1
  }
}
```

## 3) `upsert_source_cursor(cursor)`

```json
{
  "connector_name": "claude_hook",
  "cursor_key": "latest_event_id",
  "cursor_value": "evt_918273",
  "observed_at_ms": 1710000000456,
  "metadata": {
    "workspace_root": "/repo"
  }
}
```

의미:
- connector 재시작 위치 갱신
- raw event와 분리된 운영용 진행 위치

## 4) `health()`

최소 반환값:
- `db_ready`
- `schema_version`
- `pending_projection_count`
- `pending_derived_count`
- `pending_index_count`

## 5) projection/derived trigger 정책

권장 기본:
- `append_raw_events`는 **accept만 보장**
- projection/derived/index는 background worker 또는 operator command가 수행

이유:
- ingest latency를 짧게 유지
- repair/rebuild를 단순화
- Rams/Relay에서 kernel 내부 구현에 의존하지 않게 함

## 6) query contract

### canonical reads
- `get_session(session_id)`
- `get_entry(entry_id)`
- `get_artifact(artifact_id)`
- `get_anchor(anchor_id)`

### search
- `search_entries(query, filters)`
- `search_episodes(query, filters)`
- `search_insights(query, filters)`
- `search_procedures(query, filters)`

### reuse-focused helpers
- `find_fix(query, filters)`
- `find_decision(query, filters)`
- `find_runbook(query, filters)`
- `get_evidence_bundle(subject_kind, subject_id)`

## 7) MCP surface

MCP는 read-only가 원칙이다.

### tools
- `axiomsync.search_insights`
- `axiomsync.find_fix`
- `axiomsync.find_decision`
- `axiomsync.find_runbook`
- `axiomsync.get_session`
- `axiomsync.get_evidence_bundle`

### resources
- `session://<session_id>`
- `episode://<episode_id>`
- `insight://<insight_id>`
- `procedure://<procedure_id>`

### roots
- optional `context.db`
- optional derived export root

금지:
- MCP write tool
- MCP direct raw ingest
- MCP direct rebuild mutation

## 8) transport 정책

| 용도 | 권장 transport |
|---|---|
| local ingest | library / Unix socket |
| sibling service ingest | HTTP |
| agent query | MCP |
| operator maintenance | CLI |

## 9) error model

| code | 의미 |
|---|---|
| `invalid_request` | schema 또는 required field 오류 |
| `duplicate` | dedupe_key 충돌 |
| `unsupported_event` | event_type 지원 안 함 |
| `projection_error` | accept 후 background projection 실패 |
| `storage_unavailable` | DB 사용 불가 |
| `internal_error` | 예상 못한 오류 |

원칙:
- raw accept와 projection failure를 구분
- accepted raw event는 projection 실패해도 ledger에 남음
- replay/rebuild로 복구 가능해야 함

## 10) examples

- `examples/raw_event.chatgpt_selection.json`
- `examples/raw_event.axiomrams_run_summary.json`
- `schema/kernel_sink_contract.json`

---

# 05. AxiomRelay / axiomRams 연동

## 1) AxiomRelay와의 연결

### AxiomRelay가 하는 일
- ChatGPT selection capture
- Codex / Claude import
- spool / retry / dead-letter
- approval queue
- append_raw_events 호출

### AxiomSync에 넘기는 것
- **raw packet만**
- provenance 포함 selection
- tool/workspace/session identity
- connector cursor

### 넘기면 안 되는 것
- episode segmentation
- insight text
- procedure modeling
- verification synthesis
- search ranking policy

### 권장 흐름

```text
extension -> relayd pending spool
          -> approval(optional)
          -> append_raw_events(batch)
          -> upsert_source_cursor(optional)
```

### ChatGPT selection packet 최소 요건

- `native_session_id`
- `source_message.message_id`
- `source_message.role`
- `selection.text`
- `selection.start_hint`
- `selection.end_hint`
- `selection.dom_fingerprint`
- `page_url`
- `captured_at_ms`

예시는 `examples/raw_event.chatgpt_selection.json` 참조.

## 2) axiomRams와의 연결

### axiomRams가 하는 일
- `program/` + `state/`를 정본으로 실행
- plan / do / verify loop 실행
- approvals 처리
- evidence files 생성
- run summary / check result를 export

### AxiomSync에 넘기는 것
- run session summary
- task / step / check / artifact evidence
- completed or meaningful intermediate facts
- deterministic verification results
- reusable decision/fix context의 원자료

### 넘기면 안 되는 것
- Rams 내부 resume token을 kernel truth로 넘기기
- hidden scratchpad
- 승인 대기 상태를 kernel canonical state로 만들기
- Rams 전체 UI state

### 권장 export 방식

#### option A — library mode
Rust crate dependency로 직접 `append_raw_events` 호출.

장점:
- 가장 단순
- serialization hop 감소

#### option B — local socket/HTTP
Rams가 별도 프로세스로 AxiomSync에 전송.

장점:
- 배포 분리 쉬움
- local service 구조 명확

### Rams event mapping 권장

| Rams event | AxiomSync `entry_kind` |
|---|---|
| `run_started` | `run_step` |
| `task_selected` | `run_step` |
| `skill_loaded` | `tool_call` or `run_step` |
| `verification_passed` | `check_result` |
| `verification_failed` | `check_result` |
| `approval_requested` | `approval_note` |
| `approval_resolved` | `approval_note` |
| `run_completed` | `run_step` |
| `artifact_written` | `artifact_ref` |

예시는 `examples/raw_event.axiomrams_run_summary.json` 참조.

## 3) query 사용 방식

### AxiomRelay side
목적:
- 사용자가 저장한 selection / conversation evidence를 다시 찾기
- “지난번 fix / decision / runbook” 검색
- evidence bundle 열람

권장 surface:
- MCP read tools
- local HTTP for service UI

### axiomRams side
목적:
- 새 run 시작 전 과거 fix/decision/runbook 조회
- verify 단계에서 관련 evidence cross-check
- operator가 이전 사례 검색

권장 surface:
- library mode query or MCP
- deterministic verifier가 사용할 read-only helper

## 4) 세 시스템의 source of truth 정리

| 항목 | 정본 |
|---|---|
| conversation/raw capture | AxiomRelay spool 이전에는 edge source, accept 이후에는 AxiomSync raw ledger |
| reusable knowledge | AxiomSync |
| run execution state | axiomRams `state/` |
| approval queue for capture | AxiomRelay |
| approval queue for execution | axiomRams |

## 5) 가장 중요한 통합 원칙

1. **single writer**
2. **raw first**
3. **evidence first**
4. **query/read and ingest/write 분리**
5. **AxiomSync는 generic, 제품 edge는 외부**

## 6) 가장 자연스러운 사용 방식

### 패턴 A — Relay 중심
- 사용자는 ChatGPT/Codex/Claude에서 내용을 캡처
- Relay가 raw를 커널로 전달
- 커널은 episode/insight/procedure로 축적
- agent/tool은 MCP로 재사용

### 패턴 B — Rams 중심
- Rams run이 evidence를 생성
- 의미 있는 run events와 artifact를 커널로 보냄
- 다음 run이 커널의 insight/procedure를 참고
- 실행 시스템과 지식 시스템이 느슨하게 결합

### 패턴 C — Combined operator workflow
- Relay가 conversation evidence를 모음
- Rams가 implementation/verification evidence를 모음
- AxiomSync가 둘을 같은 episode/insight graph로 묶음

이 패턴이 최종적으로 가장 강하다.

---

# 06. 구현 로드맵

## Phase 0 — 계약 고정

목표:
- raw ingest / query / MCP boundary 확정
- entity naming 확정
- SQL skeleton 확정

완료 기준:
- `kernel_sink_contract.json` frozen
- core table set frozen
- write ownership 표 frozen

## Phase 1 — raw ledger

구현:
- `raw_events`
- `source_cursors`
- dedupe
- accept/reject response
- health endpoint

완료 기준:
- duplicate replay가 safe
- append_raw_events는 raw accept만 보장
- projection failure와 raw accept가 분리됨

## Phase 2 — canonical projection

구현:
- sessions
- actors
- entries
- artifacts
- anchors
- replay from raw

완료 기준:
- ChatGPT selection packet이 session/entry/anchor로 투영됨
- Rams run summary가 session/entry/artifact로 투영됨
- rebuild projection이 deterministic

## Phase 3 — derived memory

구현:
- episodes
- insights
- verifications
- procedures
- evidence links

완료 기준:
- 최소 3개 질의가 evidence-backed로 답변됨
  - `find_fix`
  - `find_decision`
  - `find_runbook`
- evidence 없는 reusable record 생성 차단

## Phase 4 — retrieval surface

구현:
- `search_docs`
- `search_docs_fts`
- query API
- MCP read tools/resources

완료 기준:
- entry / episode / insight / procedure 검색 가능
- retrieval answer에 anchor preview 포함
- MCP에서 read-only 사용 가능

## Phase 5 — integration

구현:
- AxiomRelay adapter
- axiomRams adapter
- example payload fixtures
- import contract tests

완료 기준:
- Relay packet fixture 통과
- Rams packet fixture 통과
- cursor update fixture 통과

## Phase 6 — maintenance / rebuild

구현:
- replay
- rebuild projection
- rebuild derived
- rebuild index
- purge policy
- repair tooling

완료 기준:
- DB 삭제 후 raw backup으로 복구 가능
- index drop/rebuild 안전
- corrupted derived layer 재생성 가능

## Phase 7 — hardening

구현:
- migration tests
- state invariant tests
- idempotency tests
- crash/restart tests
- performance baseline

완료 기준:
- duplicate ingest property test 통과
- rebuild determinism test 통과
- 1인 로컬 사용 규모에서 latency 허용 범위 확보

## 우선순위 원칙

1. raw accept correctness
2. canonical projection correctness
3. evidence anchoring correctness
4. query usefulness
5. convenience features

## 미루어도 되는 것

- embeddings
- graph expansion
- cross-device sync
- semantic reranker
- UI polish

이건 core가 아니다.

---

# 07. 의사코드 스펙

## 1) append_raw_events

```text
fn append_raw_events(batch):
    validate(batch)
    accepted = []
    rejected = []

    begin tx

    for event in batch.events:
        event_id = event.raw_event_id or make_raw_event_id(event)
        dedupe_key = normalize_dedupe_key(event)

        if exists raw_events where connector_name = batch.source.connector_name
           and dedupe_key = dedupe_key:
            rejected.push({dedupe_key, reason: "duplicate"})
            continue

        normalized = normalize_envelope(batch.source, event)
        content_hash = event.content_hash or sha256(json(normalized.payload))

        insert raw_events(
            raw_event_id = event_id,
            source_kind = batch.source.source_kind,
            connector_name = batch.source.connector_name,
            native_session_id = event.native_session_id,
            native_entry_id = event.native_entry_id,
            event_type = event.event_type,
            captured_at_ms = event.captured_at_ms,
            observed_at_ms = event.observed_at_ms,
            content_hash = content_hash,
            dedupe_key = dedupe_key,
            raw_payload_json = event.payload,
            normalized_json = normalized,
            projection_state = "pending"
        )

        accepted.push(event_id)

    commit tx

    return {accepted, rejected}
```

## 2) project_pending_raw_events

```text
fn project_pending_raw_events(limit):
    rows = select raw_events where projection_state = "pending" order by observed_at_ms limit limit

    for raw in rows:
        begin tx

        session = upsert_session_from_raw(raw)
        actor = upsert_actor_from_raw(raw.normalized_json)
        entry = upsert_entry_from_raw(raw, session, actor)

        for artifact_input in extract_artifacts(raw.normalized_json):
            artifact = upsert_artifact(session, entry, artifact_input)

        for anchor_input in extract_anchors(raw.normalized_json, entry, artifacts):
            upsert_anchor(session, entry, artifact_or_null(anchor_input), anchor_input)

        mark raw_events.projection_state = "projected"

        commit tx
```

## 3) derive_memory_for_session

```text
fn derive_memory_for_session(session_id):
    entries = load_entries(session_id)
    anchors = load_anchors(session_id)

    candidate_groups = segment_into_episode_candidates(entries, anchors)

    for group in candidate_groups:
        episode = upsert_episode(
            kind = classify_episode_kind(group),
            title = summarize_group_title(group),
            summary = summarize_group_body(group),
            status = infer_episode_status(group)
        )
        link_episode_anchors(episode, group.anchor_ids)

        insight_candidates = extract_insights(group)
        for ic in insight_candidates:
            if ic.anchor_ids is empty:
                continue

            insight = upsert_insight(
                kind = classify_insight_kind(ic),
                statement = normalize_statement(ic.statement),
                scope = infer_scope(ic),
                confidence = score_confidence(ic)
            )
            link_insight_anchors(insight, ic.anchor_ids)

            verification = derive_verification(insight, group)
            insert_verification_if_changed(verification)

        procedure_candidate = maybe_extract_procedure(group)
        if procedure_candidate exists and procedure_candidate.anchor_ids not empty:
            procedure = upsert_procedure(
                title = procedure_candidate.title,
                purpose = procedure_candidate.purpose,
                preconditions = procedure_candidate.preconditions,
                expected_outcome = procedure_candidate.expected_outcome
            )
            replace_procedure_steps(procedure, procedure_candidate.steps)
            link_procedure_anchors(procedure, procedure_candidate.anchor_ids)
```

## 4) derive_verification

```text
fn derive_verification(insight, episode_group):
    checks = collect_check_like_entries(episode_group)
    conflicts = find_conflicting_insights(insight.scope, insight.statement)

    if conflicts not empty:
        return verification(status="conflicted", method="heuristic")

    if any deterministic check passed in checks:
        return verification(status="verified", method="deterministic")

    if human confirmation exists in episode_group:
        return verification(status="verified", method="human")

    return verification(status="proposed", method="heuristic")
```

## 5) rebuild_index

```text
fn rebuild_index(scope):
    delete search_docs where matches(scope)

    for entry in load_entries(scope):
        upsert_search_doc(doc_kind="entry", body=entry.text_body, refs=[entry.entry_id])

    for episode in load_episodes(scope):
        upsert_search_doc(doc_kind="episode", body=episode.summary_text, refs=[episode.episode_id])

    for insight in load_insights(scope):
        upsert_search_doc(doc_kind="insight", body=insight.statement, refs=[insight.insight_id])

    for procedure in load_procedures(scope):
        upsert_search_doc(doc_kind="procedure", body=render_procedure_text(procedure), refs=[procedure.procedure_id])

    rebuild_fts()
```

## 6) find_fix

```text
fn find_fix(query, filters):
    hits = search_insights(query, filters + kind="fix")
    verified_hits = prefer_verified(hits)

    if verified_hits not empty:
        return attach_evidence_bundle(best_ranked(verified_hits))

    episode_hits = search_episodes(query, filters + kind="fix")
    if episode_hits not empty:
        return attach_evidence_bundle(best_ranked(episode_hits))

    procedure_hits = search_procedures(query, filters)
    return attach_evidence_bundle(best_ranked(procedure_hits))
```

## 7) Rams export helper

```text
fn export_rams_event(run_state, event):
    if not is_evidence_worthy(event):
        return None

    return raw_event(
        source_kind = "axiomrams",
        connector_name = "rams_runtime",
        native_session_id = "rams:run:" + run_state.run_id,
        native_entry_id = event.event_id,
        event_type = event.type,
        payload = {
            run_id: run_state.run_id,
            task_id: event.task_id,
            step_id: event.step_id,
            summary: event.summary,
            checks: event.checks,
            artifacts: event.artifacts
        },
        hints = {
            session_kind: "run",
            entry_kind: map_rams_event_to_entry_kind(event.type),
            workspace_root: run_state.workspace_root
        }
    )
```

## 8) Relay export helper

```text
fn export_chatgpt_selection(selection):
    return raw_event(
        source_kind = "axiomrelay",
        connector_name = "chatgpt_web_selection",
        native_session_id = "chatgpt:" + selection.conversation_id,
        native_entry_id = selection.message_id,
        event_type = "selection_captured",
        payload = {
            page_url: selection.page_url,
            page_title: selection.page_title,
            source_message: {
                message_id: selection.message_id,
                role: selection.role
            },
            selection: {
                text: selection.text,
                start_hint: selection.start_hint,
                end_hint: selection.end_hint,
                dom_fingerprint: selection.dom_fingerprint
            },
            user_note: selection.user_note,
            tags: selection.tags
        },
        hints = {
            session_kind: "conversation",
            entry_kind: "message"
        }
    )
```

---

# 08. 정합성 / 이해충돌 검토

이 문서는 셀프 피드백 결과를 기록한다.

## Pass 1 — 이름 충돌 검토

### 문제 후보
- `conversation-native`를 강조하면 Rams를 억지로 conversation으로 넣을 위험
- `run_*`를 중심에 두면 Relay가 왜곡됨

### 수정
- storage core를 `session / entry / artifact / anchor`로 고정
- public semantics는 conversation/episode/insight/procedure 중심 유지

### 판정
- genericity와 conversation usability가 동시에 유지된다

---

## Pass 2 — write ownership 충돌 검토

### 문제 후보
- Relay나 Rams가 kernel DB를 직접 만지면 coupling 증가
- kernel이 Rams state를 canonical로 삼으면 source-of-truth 충돌

### 수정
- single writer discipline을 명시
- Relay/Rams는 raw packet export만 허용

### 판정
- 정본 충돌 없음

---

## Pass 3 — derived memory 과잉 검토

### 문제 후보
- `insight`, `claim`, `procedure`, `verification`를 모두 separate entity로 두면 과해질 수 있음

### 수정
- `claim`을 제거하고 `insight`로 통합
- `verification`은 최소 상태 레코드만 유지

### 판정
- 구조는 충분히 강하면서도 과하지 않다

---

## Pass 4 — query와 ingest 경계 검토

### 문제 후보
- MCP에 write까지 넣으면 boundary가 흐려짐
- HTTP가 query/ingest를 동시에 넓게 담당하면 운영 복잡도 상승

### 수정
- ingest는 library/socket/HTTP
- query는 MCP 중심
- MCP는 read-only

### 판정
- 경계가 선명하다

---

## Pass 5 — evidence 요구 검토

### 문제 후보
- episode/insight/procedure가 evidence 없이 생성되면 품질 하락
- Rams의 check result와 Relay의 selection이 같은 신뢰도로 섞일 수 있음

### 수정
- reusable knowledge에 최소 1개 anchor 요구
- verification을 별도 레코드로 두어 신뢰 수준 분리

### 판정
- evidence-first 원칙 유지

---

## Pass 6 — retrieval 과잉 검토

### 문제 후보
- embeddings / graph / reranker를 core에 넣으면 단순성 저하
- 없으면 장기 검색 품질이 아쉬울 수 있음

### 수정
- FTS를 core, embeddings는 optional
- retrieval index는 disposable로 정의

### 판정
- 본질 유지, 확장 여지 확보

---

## Pass 7 — AxiomRelay / axiomRams 이해충돌 검토

### 문제 후보
- Relay approval과 Rams approval이 같은 시스템으로 합쳐질 위험
- Relay capture state가 Rams run state처럼 보일 위험

### 수정
- capture approval은 Relay
- execution approval은 Rams
- kernel은 approval state의 정본이 아님

### 판정
- 제품 경계 충돌 없음

---

## Pass 8 — 최종 체크리스트

아래 질문에 모두 `예`로 답할 수 있어야 한다.

1. AxiomSync가 단독으로 knowledge kernel 역할을 설명할 수 있는가? — 예
2. AxiomRelay가 kernel modeling 없이도 존재할 수 있는가? — 예
3. axiomRams가 kernel 없이도 run state를 유지할 수 있는가? — 예
4. 세 시스템이 direct DB coupling 없이 연결되는가? — 예
5. reusable memory가 항상 evidence로 거슬러 올라갈 수 있는가? — 예
6. search index를 버리고도 정본을 유지할 수 있는가? — 예
7. conversation use-case와 run use-case를 동시에 수용하는가? — 예

## 최종 판정

이 설계는 다음 세 조건을 동시에 만족한다.

- **단순함**
- **genericity**
- **evidence-backed reuse**

따라서 현재 문맥에서 가장 강한 최종형 문서 세트로 판단한다.

---

