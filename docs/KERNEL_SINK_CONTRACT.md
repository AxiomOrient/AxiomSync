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

## Request Shapes

### `POST /sink/raw-events/plan`
```json
{
  "request_id": "req-1",
  "events": [
    {
      "source": "chatgpt",
      "native_schema_version": "chatgpt-selection-v1",
      "native_session_id": "/c/abc123",
      "native_event_id": "evt-1",
      "event_type": "selection_captured",
      "ts_ms": 1710000000000,
      "payload": {}
    }
  ]
}
```

### `POST /sink/raw-events/apply`
Request body is a serialized `IngestPlan`.

### `POST /sink/source-cursors/plan`
```json
{
  "source": "codex",
  "cursor": {
    "cursor_key": "events",
    "cursor_value": "cursor-1",
    "updated_at_ms": 1710000000000
  }
}
```

### `POST /sink/source-cursors/apply`
Request body is a serialized `SourceCursorUpsertPlan`.

## Response Semantics
- `/sink/raw-events/plan` returns `IngestPlan`
- `/sink/raw-events/apply` returns the apply transaction result
- `/sink/source-cursors/plan` returns `SourceCursorUpsertPlan`
- `/sink/source-cursors/apply` returns the apply transaction result
- `/health` returns main runtime health metadata with DB path

## Error Semantics
- invalid payload: `400`
- conflict: `409`
- transient/internal failure: `429`, `500`, or `503` depending on adapter policy
