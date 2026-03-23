# axiomsync Test Intent

이 문서는 현재 `axiomsync` crate가 실제로 보호하는 회귀 의도를 간단히 고정한다.
이 crate의 초점은 app shell 경계, unified HTTP/MCP 표면, canonical sink surface다.

## 1) `renewal_kernel.rs`
- raw ingest -> projection -> derivation -> retrieval 흐름이 결정론적으로 유지된다.
- purge/replay/repair가 derived state를 일관되게 재계산한다.
- canonical sink 경로에서도 같은 dedupe/cursor 규칙이 유지된다.

## 2) `sink_contract.rs`
- sink HTTP router는 raw-only append contract를 지킨다.
- duplicate append는 성공으로 처리하되 duplicate count로 집계된다.
- `plan-*` route는 무변이여야 하고 `apply-*` route만 mutation을 수행해야 한다.
- source cursor는 event batch 없이 독립 저장 가능해야 한다.
- malformed sink request는 `400`으로 거절된다.
- sink route는 loopback source address가 아니면 거절되어야 한다.

## 3) `http_and_mcp.rs`
- main query/admin router는 bearer auth와 workspace binding을 강제한다.
- global admin route는 workspace token이 아니라 admin token을 요구한다.
- MCP HTTP/stdio surface는 workspace scope를 벗어나지 않는다.
- web UI와 MCP query surface는 현재 kernel projection을 렌더링한다.

## 4) `process_contract.rs`
- operator CLI help는 현재 public command set을 정확히 노출한다.
- `sink`는 canonical write surface로 드러나고, 제거된 legacy command는 다시 나타나지 않아야 한다.
- 실제 operator 흐름인 `init -> sink plan/apply -> cursor plan/apply -> project plan/apply -> search`가 mixed producer record에서도 유지되어야 한다.
- conversation, execution, document record가 함께 들어와도 CLI 실사용 경로에서 deterministic replay/search 결과가 깨지지 않아야 한다.
