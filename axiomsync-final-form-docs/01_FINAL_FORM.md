# 01. AxiomSync 최종형 정의

## 1) 최종 정의

AxiomSync는 **지식 커널**이다.  
제품 edge, 실행 orchestration, capture runtime, approval queue, connector polling은 하지 않는다.

AxiomSync의 일은 정확히 5가지다.

1. **raw event ledger 저장**
2. **canonical projection 유지**
3. **episode / insight / verification / procedure 파생**
4. **evidence anchor 기반 재사용**
5. **read-only query surface 제공**

## 2) 왜 이것이 최소 핵심인가

AxiomRelay와 axiomRams는 서로 다르다.

- **AxiomRelay**는 capture / spool / forward 시스템이다.
- **axiomRams**는 file-first execution runtime이다.

둘을 하나의 범용 커널 위에 연결하려면, 커널 중심은 `conversation-only`도 `run-only`도 아니어야 한다.  
따라서 canonical 중심은 아래 4개로 잡는다.

- `session`
- `entry`
- `artifact`
- `anchor`

이 4개는 두 시스템 모두를 왜곡 없이 담을 수 있다.

## 3) AxiomSync가 반드시 소유할 것

| 영역 | 소유 내용 |
|---|---|
| Raw truth | immutable raw events, dedupe receipt, source cursor |
| Canonical core | sessions, actors, entries, artifacts, anchors |
| Reusable memory | episodes, insights, verifications, procedures |
| Retrieval | search docs / FTS / optional embeddings |
| Query surface | CLI, HTTP, MCP read tools/resources |
| Maintenance | replay, rebuild, purge, repair, migration |

## 4) AxiomSync가 소유하면 안 되는 것

| 금지 영역 | 이유 |
|---|---|
| connector polling / watch / sync | 제품 edge 책임이다 |
| browser capture / extension runtime | Relay 책임이다 |
| pending/sent/dead-letter spool | Relay 책임이다 |
| approval queue | Relay 또는 Rams 책임이다 |
| operator task board / run state | Rams 책임이다 |
| ChatGPT / Claude / Codex auth refresh | 제품 adapter 책임이다 |
| execution orchestration | Rams 책임이다 |
| service UI / branding | 커널 본질이 아니다 |

## 5) conversation-native 이면서 generic 이어야 하는 이유

AxiomSync는 conversation-native여야 한다.  
왜냐하면 ChatGPT, Claude, Codex, Gemini, agent transcript는 모두 `대화/세션` 질의가 핵심이기 때문이다.

하지만 core schema를 `conv_*`로 고정하면 axiomRams의 run/task/check/evidence를 억지로 conversation으로 넣게 된다.  
그래서 다음처럼 나눈다.

- **storage core**: `session / entry / artifact / anchor`
- **primary public semantics**: conversation / episode / insight / procedure

즉, 저장 구조는 generic, 질의 경험은 conversation-native로 유지한다.

## 6) Derived memory의 최종 형태

### `episode`
재사용 가능한 작업 단위.
예:
- 문제 조사
- 버그 수정
- 설계 결정
- 실험 결과
- 실행 run 요약

### `insight`
evidence-backed distilled statement.
예:
- root cause
- fix summary
- decision
- invariant
- warning
- preference

### `verification`
insight 또는 procedure가 얼마나 믿을 수 있는지 나타내는 검증 기록.
예:
- deterministic check passed
- human confirmed
- stale
- conflicted
- superseded

### `procedure`
재사용 가능한 how-to / runbook.
반드시 evidence 또는 verified insight에 연결된다.

## 7) 최종 non-goals

아래는 최종형에서도 하지 않는다.

- general chat UI
- product-specific ranking policy
- autonomous planning system
- cross-device sync product
- external side-effect execution
- browser automation primary capture
- semantic magic without evidence

## 8) 최종 판정 문장

> **AxiomSync는 “모든 것을 하는 앱”이 아니라, evidence-backed memory를 만드는 local knowledge kernel이다.**
