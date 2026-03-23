# 05. AxiomRelay / axiomRams 연동

## 1) AxiomRelay와의 연결

### AxiomRelay가 하는 일
- ChatGPT selection capture
- Codex / Claude import
- spool / retry / dead-letter
- approval queue
- append_raw_events 호출

### AxiomSync에 넘기는 것
- **raw packet만**
- provenance 포함 selection
- tool/workspace/session identity
- connector cursor

### 넘기면 안 되는 것
- episode segmentation
- insight text
- procedure modeling
- verification synthesis
- search ranking policy

### 권장 흐름

```text
extension -> relayd pending spool
          -> approval(optional)
          -> append_raw_events(batch)
          -> upsert_source_cursor(optional)
```

### ChatGPT selection packet 최소 요건

- `native_session_id`
- `source_message.message_id`
- `source_message.role`
- `selection.text`
- `selection.start_hint`
- `selection.end_hint`
- `selection.dom_fingerprint`
- `page_url`
- `captured_at_ms`

예시는 `examples/raw_event.chatgpt_selection.json` 참조.

## 2) axiomRams와의 연결

### axiomRams가 하는 일
- `program/` + `state/`를 정본으로 실행
- plan / do / verify loop 실행
- approvals 처리
- evidence files 생성
- run summary / check result를 export

### AxiomSync에 넘기는 것
- run session summary
- task / step / check / artifact evidence
- completed or meaningful intermediate facts
- deterministic verification results
- reusable decision/fix context의 원자료

### 넘기면 안 되는 것
- Rams 내부 resume token을 kernel truth로 넘기기
- hidden scratchpad
- 승인 대기 상태를 kernel canonical state로 만들기
- Rams 전체 UI state

### 권장 export 방식

#### option A — library mode
Rust crate dependency로 직접 `append_raw_events` 호출.

장점:
- 가장 단순
- serialization hop 감소

#### option B — local socket/HTTP
Rams가 별도 프로세스로 AxiomSync에 전송.

장점:
- 배포 분리 쉬움
- local service 구조 명확

### Rams event mapping 권장

| Rams event | AxiomSync `entry_kind` |
|---|---|
| `run_started` | `run_step` |
| `task_selected` | `run_step` |
| `skill_loaded` | `tool_call` or `run_step` |
| `verification_passed` | `check_result` |
| `verification_failed` | `check_result` |
| `approval_requested` | `approval_note` |
| `approval_resolved` | `approval_note` |
| `run_completed` | `run_step` |
| `artifact_written` | `artifact_ref` |

예시는 `examples/raw_event.axiomrams_run_summary.json` 참조.

## 3) query 사용 방식

### AxiomRelay side
목적:
- 사용자가 저장한 selection / conversation evidence를 다시 찾기
- “지난번 fix / decision / runbook” 검색
- evidence bundle 열람

권장 surface:
- MCP read tools
- local HTTP for service UI

### axiomRams side
목적:
- 새 run 시작 전 과거 fix/decision/runbook 조회
- verify 단계에서 관련 evidence cross-check
- operator가 이전 사례 검색

권장 surface:
- library mode query or MCP
- deterministic verifier가 사용할 read-only helper

## 4) 세 시스템의 source of truth 정리

| 항목 | 정본 |
|---|---|
| conversation/raw capture | AxiomRelay spool 이전에는 edge source, accept 이후에는 AxiomSync raw ledger |
| reusable knowledge | AxiomSync |
| run execution state | axiomRams `state/` |
| approval queue for capture | AxiomRelay |
| approval queue for execution | axiomRams |

## 5) 가장 중요한 통합 원칙

1. **single writer**
2. **raw first**
3. **evidence first**
4. **query/read and ingest/write 분리**
5. **AxiomSync는 generic, 제품 edge는 외부**

## 6) 가장 자연스러운 사용 방식

### 패턴 A — Relay 중심
- 사용자는 ChatGPT/Codex/Claude에서 내용을 캡처
- Relay가 raw를 커널로 전달
- 커널은 episode/insight/procedure로 축적
- agent/tool은 MCP로 재사용

### 패턴 B — Rams 중심
- Rams run이 evidence를 생성
- 의미 있는 run events와 artifact를 커널로 보냄
- 다음 run이 커널의 insight/procedure를 참고
- 실행 시스템과 지식 시스템이 느슨하게 결합

### 패턴 C — Combined operator workflow
- Relay가 conversation evidence를 모음
- Rams가 implementation/verification evidence를 모음
- AxiomSync가 둘을 같은 episode/insight graph로 묶음

이 패턴이 최종적으로 가장 강하다.
