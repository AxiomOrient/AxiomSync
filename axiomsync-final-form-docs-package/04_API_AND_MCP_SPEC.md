# 04. API And MCP Spec

## Write Contract

### CLI sink
- `axiomsync sink plan-append-raw-events --file batch.json`
- `axiomsync sink apply-ingest-plan --file ingest-plan.json`
- `axiomsync sink plan-upsert-source-cursor --file cursor.json`
- `axiomsync sink apply-source-cursor-plan --file cursor-plan.json`

### CLI operator rebuild
- `axiomsync project plan-projection`
- `axiomsync project apply-projection-plan --file projection-plan.json`
- `axiomsync project plan-derivations`
- `axiomsync project apply-derivation-plan --file derivation-plan.json`
- `axiomsync project plan-rebuild`
- `axiomsync project apply-replay-plan --file replay-plan.json`

### HTTP sink
- `GET /health`
- `POST /sink/raw-events/plan`
- `POST /sink/raw-events/apply`
- `POST /sink/source-cursors/plan`
- `POST /sink/source-cursors/apply`

### HTTP operator rebuild
- `POST /admin/projection/plan`
- `POST /admin/projection/apply`
- `POST /admin/derivations/plan`
- `POST /admin/derivations/apply`
- `POST /admin/replay/plan`
- `POST /admin/replay/apply`

## Sink Semantics

- `plan-*`은 request를 검증하고 serialized plan을 반환한다
- `apply-*`는 plan payload만 받는다
- sink route는 loopback source address만 허용한다
- source cursor upsert는 raw append와 독립 operation이다

## Query Contract

canonical read:
- `get_session`
- `get_entry`
- `get_artifact`
- `get_anchor`
- `get_episode`
- `get_procedure`
- `search_entries`
- `search_episodes`
- `search_docs`
- `search_insights`
- `search_claims`
- `search_procedures`
- `find_fix`
- `find_decision`
- `find_runbook`
- `get_evidence_bundle`

compatibility read:
- `get_case`
- `get_thread`
- `get_runbook`
- `get_task`

## MCP

tools:
- `search_entries`
- `search_episodes`
- `search_docs`
- `search_insights`
- `search_claims`
- `search_procedures`
- `find_fix`
- `find_decision`
- `find_runbook`
- `get_evidence_bundle`
- `get_session`
- `get_entry`
- `get_artifact`
- `get_anchor`
- `get_case`
- `get_thread`
- `get_runbook`
- `get_task`
