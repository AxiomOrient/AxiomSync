# 01. Final Form

AxiomSync는 immutable raw record를 SQLite에 저장하고, 이를 `session / entry / artifact / anchor`로 투영한 뒤, `episode / insight / verification / procedure`를 evidence-backed memory로 파생해 CLI, HTTP, MCP로 읽게 해 주는 local-first knowledge kernel이다.

소유 범위:
- raw ledger: `ingress_receipts`
- source progress: `source_cursor`
- canonical projection: `sessions`, `actors`, `entries`, `artifacts`, `anchors`
- derived memory: `episodes`, `insights`, `verifications`, `claims`, `procedures`, `search_docs`
- public surface: CLI, HTTP API, MCP

비소유 범위:
- connector polling, watch, sync worker
- spool, retry, approval, delivery orchestration
- browser capture, external auth refresh

원칙:
- write boundary는 raw-only `sink`
- write와 rebuild는 모두 `plan -> apply`
- ID와 경로는 입력 데이터로부터 결정론적으로 계산
- compatibility noun은 남기되 canonical noun으로 승격하지 않음
