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
