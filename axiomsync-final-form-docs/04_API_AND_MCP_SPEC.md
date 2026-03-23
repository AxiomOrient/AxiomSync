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
