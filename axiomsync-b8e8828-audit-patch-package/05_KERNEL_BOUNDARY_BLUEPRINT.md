# 최종 커널 경계 청사진

## AxiomSync가 소유하는 것

### ingest truth
- `raw_event`
- `source_cursor`
- `import_journal`

### canonical projection
- `workspace`
- `conv_session`
- `conv_turn`
- `conv_item`
- `artifact`
- `evidence_anchor`

### derived knowledge
- `episode`
- `episode_member`
- `insight`
- `verification`
- `insight_anchor`
- `search_doc_redacted`

### query / delivery
- `get_thread`
- `get_evidence`
- `search_cases`
- `get_case`
- `search_commands`
- MCP resources/tools/roots
- replay / purge / repair / doctor

## AxiomSync가 소유하면 안 되는 것

- connector polling
- connector watch loop
- connector-specific serve endpoint
- connectors.toml
- retry / spool / approval
- browser integration
- delivery orchestration

## canonical public surface

### CLI
```text
axiomsync init
axiomsync sink plan-append-raw-events --file <raw-events.json>
axiomsync sink apply-ingest-plan --file <ingest-plan.json>
axiomsync sink plan-upsert-source-cursor --file <source-cursor.json>
axiomsync sink apply-source-cursor-plan --file <cursor-plan.json>
axiomsync project plan-rebuild
axiomsync project apply-replay-plan --file <replay-plan.json>
axiomsync derive plan
axiomsync derive apply-plan --file <derive-plan.json>
axiomsync search <query>
axiomsync mcp serve
axiomsync web
```

### HTTP
```text
GET  /health
POST /sink/raw-events/plan
POST /sink/raw-events/apply
POST /sink/source-cursors/plan
POST /sink/source-cursors/apply
GET  /api/cases
GET  /api/cases/{id}
GET  /api/threads/{id}
GET  /api/evidence/{id}
POST /mcp
```

### MCP
canonical:
- `search_cases`
- `get_case`
- `get_thread`
- `get_evidence`
- `search_commands`

compat:
- `search_episodes`
- `get_runbook`

## 핵심 설계 원칙

### 원칙 1 — raw append와 cursor update는 분리
이 둘을 한 요청으로 묶지 않는다.

### 원칙 2 — 저장하는 noun만 canonical noun로 올린다
지금 강한 중심은 `thread / case / evidence / command`다.

### 원칙 3 — edge policy는 외부 repo
AxiomSync는 knowledge kernel이지 collector가 아니다.

### 원칙 4 — plan/apply를 끝까지 유지
side effect는 항상 serialized plan을 거친다.

## Rust pseudocode

```rust
pub struct SourceCursorInput {
    pub connector: String,
    pub cursor_key: String,
    pub cursor_value: String,
    pub updated_at_ms: i64,
}

pub struct SourceCursorUpsertPlan {
    pub row: SourceCursorRow,
}

impl AxiomSync {
    pub fn plan_append_raw_events(&self, input: &ConnectorBatchInput) -> Result<IngestPlan>;
    pub fn apply_ingest_plan(&self, plan: &IngestPlan) -> Result<Value>;

    pub fn plan_upsert_source_cursor(
        &self,
        input: &SourceCursorInput,
    ) -> Result<SourceCursorUpsertPlan>;

    pub fn apply_source_cursor_plan(
        &self,
        plan: &SourceCursorUpsertPlan,
    ) -> Result<Value>;

    pub fn plan_rebuild(&self) -> Result<ReplayPlan>;
    pub fn apply_replay_plan(&self, plan: &ReplayPlan) -> Result<Value>;

    pub fn plan_derivation(&self) -> Result<DerivePlan>;
    pub fn apply_derive_plan(&self, plan: &DerivePlan) -> Result<Value>;
}
```
