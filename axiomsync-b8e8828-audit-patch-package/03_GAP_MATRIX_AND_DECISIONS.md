# 갭 매트릭스와 결정

| 항목 | 현재 상태 | 심각도 | 결정 |
|---|---|---:|---|
| narrow sink CLI 미구현 | README는 sink 기준, 실제 CLI는 connector 기준 | P0 | `connector` 제거, `sink` 추가 |
| narrow sink HTTP 미구현 | 실제 route가 `/ingest/{connector}` / `/project` / `/derive` | P0 | `/sink/*` canonical route로 교체 |
| connector runtime 누수 | sync/watch/repair/serve가 app crate 안에 존재 | P0 | 외부 repo로 추출 |
| cursor contract 미완 | cursor가 ingest batch 옵션에 묶여 있음 | P0 | `SourceCursorUpsertPlan` 1급 시민화 |
| kernel이 connector config 소유 | `AxiomSync`가 connectors port를 보유 | P0 | 제거 |
| workspace split 미완 | `axiomsync-cli`/`http`가 있지만 entrypoint 미이관 | P1 | split 완결 또는 dead code 제거 |
| MCP / UI legacy noun | `episode` / `runbook` 중심 | P1 | canonical `case` alias 추가, legacy alias 유지 |
| overclaimed query noun | 실제 저장하지 않는 `run/task/document`를 먼저 약속 | P1 | 저장 모델이 생길 때까지 public contract에서 과장 금지 |
| search_doc_redacted 미노출 | 저장은 되지만 query surface가 약함 | P2 | 이후 `case` document slice로 노출 |

## 핵심 결정

### 결정 1
지금 필요한 건 **스키마 대확장**이 아니다.  
가장 큰 문제는 **release surface와 ownership 경계**다.

### 결정 2
`source_cursor`는 반드시 ingest batch에서 분리된 독립 계약이 있어야 한다.  
raw event append와 cursor update는 실제 운영에서 다른 cadence를 가진다.

### 결정 3
`axiomsync-cli` / `axiomsync-http` crate split은 “있는데 안 쓰는 상태”가 가장 나쁘다.  
둘 중 하나를 선택해야 한다.

- 끝까지 배선한다
- 아니면 제거한다

### 결정 4
지금 커널이 실제로 잘하는 noun은 아래다.

- thread
- evidence
- case(= episode/runbook alias)
- command

여기에 없는 noun을 먼저 public contract에 올리는 건 오히려 제품 품질을 해친다.
