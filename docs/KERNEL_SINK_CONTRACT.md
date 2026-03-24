# Kernel Sink Contract

`sink`는 AxiomSync의 canonical raw-only write surface다.

## Principles
- Parse -> Normalize -> Plan -> Apply
- raw only
- deterministic ids
- projection/derivation meaning stays inside the kernel
- `plan-*` returns only plan payload
- `apply-*` accepts only plan payload

## CLI Surface
- `axiomsync sink plan-append-raw-events --file batch.json`
- `axiomsync sink apply-ingest-plan --file ingest-plan.json`
- `axiomsync sink plan-upsert-source-cursor --file cursor.json`
- `axiomsync sink apply-source-cursor-plan --file cursor-plan.json`

## HTTP Surface
- `GET /health`
- `POST /sink/raw-events/plan`
- `POST /sink/raw-events/apply`
- `POST /sink/source-cursors/plan`
- `POST /sink/source-cursors/apply`

Sink routes live on the main `web` server. Default base URL is `http://127.0.0.1:4400`.
These routes are intentionally unauthenticated but are enforced as loopback-only by source address.
Canonical server entrypoint is `axiomsync serve`.

## Request Shapes

### `POST /sink/raw-events/plan`
AxiomSync accepts one canonical append envelope:
```json
{
  "batch_id": "relay-2026-03-23T12:00:00Z-001",
  "producer": "axiomrelay",
  "received_at_ms": 1710000000123,
  "events": [
    {
      "connector": "chatgpt_web_selection",
      "native_schema_version": "1",
      "native_session_id": "chatgpt:abc123",
      "native_event_id": "evt_42",
      "event_type": "selection_captured",
      "ts_ms": 1710000000123,
      "payload": {
        "session_kind": "thread",
        "workspace_root": "/workspace/demo",
        "page_url": "https://chatgpt.com/c/abc123",
        "page_title": "ChatGPT - Architecture Review",
        "source_message": {
          "message_id": "msg_42",
          "role": "assistant"
        },
        "selection": {
          "text": "Use a narrow sink contract between relayd and AxiomSync.",
          "start_hint": "Use a narrow sink contract",
          "end_hint": "between relayd and AxiomSync.",
          "dom_fingerprint": "sha1:dom:fp_001"
        }
      }
    }
  ]
}
```

Supported `event_type` values:
- `message_captured`
- `selection_captured`
- `command_started`
- `command_finished`
- `artifact_emitted`
- `verification_recorded`
- `task_state_imported`
- `approval_requested`
- `approval_resolved`
- `note_recorded`

### `POST /sink/raw-events/apply`
Request body is a serialized `IngestPlan`.

### `POST /sink/source-cursors/plan`
```json
{
  "connector": "codex",
  "cursor_key": "events",
  "cursor_value": "cursor-1",
  "updated_at_ms": 1710000000000
}
```

### `POST /sink/source-cursors/apply`
Request body is a serialized `SourceCursorUpsertPlan`.

## Response Semantics
- `/sink/raw-events/plan` returns `IngestPlan`
- `/sink/raw-events/apply` returns the apply transaction result
- `/sink/source-cursors/plan` returns `SourceCursorUpsertPlan`
- `/sink/source-cursors/apply` returns the apply transaction result
- `/health` returns main runtime health metadata with DB path plus pending projection/derivation/index counts

## Error Semantics
- invalid payload: `400`
- conflict: `409`
- transient/internal failure: `429`, `500`, or `503` depending on adapter policy
