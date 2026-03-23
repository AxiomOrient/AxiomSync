# API Contract

## Design center

AxiomSync is a **knowledge kernel**, not a connector daemon.

## Transport priority

1. local CLI / Unix socket
2. same-machine HTTP
3. MCP for query, not ingest

## Ingest sink

### `POST /sink/raw-events`
Appends immutable raw envelopes.

Request:
```json
{
  "batch_id": "ingest-2026-03-23-001",
  "events": [ ... ]
}
```

Response:
```json
{
  "accepted": 12,
  "rejected": 0,
  "receipt_ids": ["..."],
  "projection_required": true
}
```

### `GET /health`
Returns:
- db status
- schema version
- index freshness

## Operator maintenance

### `POST /admin/rebuild/projection`
### `POST /admin/rebuild/derivations`
### `POST /admin/rebuild/index`

These are operator-only.

## Query API

### `GET /sessions/:id`
### `GET /entries/:id`
### `GET /artifacts/:id`
### `GET /anchors/:id`

### `POST /query/search-entries`
### `POST /query/search-episodes`
### `POST /query/search-claims`
### `POST /query/search-procedures`

## Non-goals

This API does not expose:
- connector sync
- connector watch
- connector repair
- connector serve
- approval queues
- spool state
