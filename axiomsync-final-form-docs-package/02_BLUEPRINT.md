# 02. 청사진 / 구조도

## 1) 전체 구조

```text
[Relay for ChatGPT] --selected capture--> [AxiomRelay / relayd]
[Codex/Claude adapters] -----------------> [AxiomRelay / relayd]
                                           | spool / retry / approval
                                           v
                                   append_raw_events(batch)
                                           v
                                      [AxiomSync]
                              raw ledger -> canonical -> derived
                                           |
                                           +--> CLI maintenance
                                           +--> HTTP query
                                           +--> MCP read tools/resources

[axiomRams runtime] --run evidence/export--> append_raw_events(batch)
[axiomRams operator] -----------------------> MCP / HTTP query
```

## 2) 세 프로젝트의 경계

### AxiomSync
- `context.db` 소유
- raw truth 소유
- canonical projection 소유
- reusable memory 소유
- query semantics 소유

### AxiomRelay
- capture edge 소유
- local spool 소유
- retry / dead-letter 소유
- approval queue 소유
- kernel forwarding 소유

### axiomRams
- `program/` + `state/` 파일 정본 소유
- runtime loop 소유
- approvals for execution side effects 소유
- deterministic verification state 소유
- AxiomSync import/export adapter 소유

## 3) 연결 방식

### ingest
우선순위:
1. 같은 프로세스면 library call
2. 같은 머신이면 Unix socket
3. 그 외는 local HTTP

### query
우선순위:
1. 에이전트/도구 재사용은 MCP
2. 로컬 디버그/운영은 CLI
3. 대시보드/내부 UI는 HTTP

## 4) repo 구조 제안

### AxiomSync repo

```text
crates/
  axiomsync-domain/
  axiomsync-kernel/
  axiomsync-store-sqlite/
  axiomsync-http/
  axiomsync-mcp/
  axiomsync-cli/
docs/
schema/
```

### AxiomRelay repo

```text
apps/
  relayd/
extensions/
  relay-for-chatgpt/
workers/
  relay-repair-worker/
config/
```

### axiomRams repo

```text
src/
  runtime/
  registry/
  verifier/
  state/
  api/
program/
state/
schemas/
adapters/
  axiomsync/
```

## 5) write ownership

| 시스템 | 정본 | 직접 write 허용 |
|---|---|---|
| AxiomSync | `context.db` | AxiomSync 내부만 |
| AxiomRelay | spool / queue state | AxiomRelay 내부만 |
| axiomRams | `state/` files | axiomRams 내부만 |

절대 금지:
- Relay가 AxiomSync DB 직접 쓰기
- Rams가 AxiomSync DB 직접 쓰기
- AxiomSync가 Rams run state 직접 수정하기

## 6) kernel mode

AxiomSync는 세 가지 mode를 지원하면 충분하다.

### library mode
- Rust process 내부에서 직접 호출
- axiomRams에 가장 자연스럽다

### local service mode
- sibling process로 동작
- AxiomRelay에 가장 자연스럽다

### maintenance mode
- rebuild / repair / inspect 용 CLI

## 7) 핵심 설계 선택

### 선택 1 — `session` 중심
`conv_*`만으로 가지 않는다.  
그렇다고 run-only로도 가지 않는다.

### 선택 2 — raw와 derived를 분리
raw event는 불변.
episode/insight/procedure는 재생성 가능.

### 선택 3 — evidence 없으면 재사용 금지
모든 reusable knowledge는 최소 1개 이상 anchor를 가져야 한다.

### 선택 4 — query는 read-only
ingest path와 query path를 분리한다.

### 선택 5 — retrieval index는 disposable
FTS/embeddings는 파생물이다.
정본은 raw + canonical + derived memory다.
