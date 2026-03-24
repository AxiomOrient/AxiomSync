# Relay Interop

이 문서는 same-host 배치에서 `AxiomRelay -> AxiomSync` 연동을 현재 repo public contract에 맞춰 고정한다.

## Boundary
- AxiomSync는 raw-only `sink` write surface와 knowledge read model을 소유한다.
- AxiomRelay는 capture, normalization, spool, retry, sent ledger, durable cursor truth를 소유한다.
- canonical topology는 `AxiomRelay -> http://127.0.0.1:4400/sink/* -> AxiomSync` 다.
- sink route는 bearer token 없이 loopback source address만 허용한다.
- duplicate raw append와 source cursor upsert는 둘 다 idempotent success semantics로 고정한다.

## Non-Goals
- remote Relay -> remote AxiomSync direct write
- public network exposed sink
- Relay가 AxiomSync 내부 crate를 직접 링크하는 구조
- 이 repo 내부에 relay durable state store나 sent ledger를 구현하는 일

## Relay Batch Contract
테스트 전용 relay fixture는 아래 shape를 사용한다.

```json
{
  "batch_id": "relay-batch-001",
  "producer": "axiomrelay",
  "received_at_ms": 1711000000000,
  "packets": [
    {
      "packet_id": "pkt-1",
      "source_kind": "chatgpt_web",
      "native_schema_version": "relay-v1",
      "source_identity": {
        "connector": "chatgpt_web_selection",
        "native_session_id": "chatgpt:abc123",
        "native_event_id": "evt-42"
      },
      "event_type": "selection_captured",
      "observed_at_ms": 1711000000100,
      "workspace_hint": "/workspace/demo",
      "labels": ["assistant", "selection"],
      "artifact_refs": [
        {
          "uri": "file:///workspace/demo/docs/spec.md",
          "mime": "text/markdown"
        }
      ],
      "payload": {
        "session_kind": "thread"
      }
    }
  ],
  "cursor_updates": [
    {
      "connector": "chatgpt_web_selection",
      "cursor_key": "thread",
      "cursor_value": "cursor-42",
      "updated_at_ms": 1711000000300
    }
  ]
}
```

## Mapping
Relay adapter는 internal batch를 repo sink contract로 분해한다.

| Relay field | AxiomSync target | Rule |
|---|---|---|
| `batch_id` | `AppendRawEventsRequest.batch_id` | 그대로 사용 |
| `producer` | `AppendRawEventsRequest.producer` | 그대로 사용 |
| `received_at_ms` | `AppendRawEventsRequest.received_at_ms` | 그대로 사용 |
| `source_identity.connector` | `RawEvent.connector` | canonical connector name 사용 |
| `source_identity.native_session_id` | `native_session_id` | canonical sink key로 emit |
| `source_identity.native_event_id` | `native_event_id` | 있으면 사용 |
| `source_identity.native_entry_id` | `native_event_id` | compatibility alias로 수용 가능 |
| `event_type` | `event_type` | repo taxonomy만 허용 |
| `observed_at_ms` | `ts_ms` | epoch millis 사용 |
| `packet_id` | `payload.relay.packet_id` | raw payload 안에 보존 |
| `source_kind` | `payload.relay.source_kind` | raw payload 안에 보존 |
| `workspace_hint` | `payload.workspace_root` | payload 안에 보존 |
| `labels[]` | `payload.labels[]` | payload 안에 보존 |
| `artifact_refs[]` | `payload.artifact_refs[]` | payload 안에 보존 |
| relay payload | `RawEvent.payload` | semantic derivation 없이 구조 보존 |
| cursor update | `UpsertSourceCursorRequest` | `connector/cursor_key/cursor_value/updated_at_ms` 그대로 사용 |

현재 구현은 canonical request 외에도 `artifacts`, `hints`, `native_entry_id` 같은 helper field를 받아들인다. 다만 relay adapter가 emit 하는 wire example은 canonical sink key를 우선 사용한다.

## Delivery Sequence
canonical delivery sequence는 아래로 고정한다.

```text
1. Relay selects an unsent batch
2. Relay translates the batch into one AppendRawEventsRequest and N UpsertSourceCursorRequest values
3. POST /sink/raw-events/plan
4. POST /sink/raw-events/apply
5. POST /sink/source-cursors/plan
6. POST /sink/source-cursors/apply
7. Only after both apply phases succeed, Relay may commit sent ledger state
```

## Failure Expectations
- raw apply 이전에 cursor plan/apply 를 시작하면 안 된다.
- raw apply 성공 후 cursor plan/apply 가 실패하면 Relay는 sent commit 을 하면 안 된다.
- apply 응답이 유실되거나 결과가 불명확한 경우 최종 `ambiguous_delivery` 상태 저장은 Relay 책임이다.
- AxiomSync는 Relay sent ledger의 정본이 아니다.

## Duplicate Semantics
- duplicate raw append는 `409`가 아니라 idempotent success다.
- duplicate batch 재전송 시 second ingest plan은 `receipts = []` 와 `skipped_dedupe_keys = [...]` 로 노출된다.
- source cursor upsert는 동일 plan 재적용 시에도 idempotent upsert다.
- Relay retry policy는 AxiomSync sink가 duplicate append/cursor upsert를 no-op success로 처리한다는 전제 위에 맞춘다.

## Verification
- fixture translation parity는 `crates/axiomsync-cli/tests/relay_interop.rs` 에서 검증한다.
- HTTP smoke는 실제 `/sink/raw-events/*` 와 `/sink/source-cursors/*` route를 loopback source address로 호출한다.
- release gate는 workspace regression 전체와 focused relay HTTP smoke를 함께 실행한다.

## Closure
- relay-compatible scope는 문서, fixture, 테스트 harness, release gate까지 포함해 이 레포 안에서 닫힌 상태로 본다.
- 외부 실제 Relay 런타임, sent ledger 저장소, ambiguous delivery 자동해결 정책은 이 레포 범위 밖이다.
