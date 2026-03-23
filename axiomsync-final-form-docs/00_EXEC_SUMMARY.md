# 00. Executive Summary

## 최종 판정

가장 단순하고 강한 최종형은 아래다.

- **AxiomSync = local-first conversation-native knowledge kernel**
- **AxiomRelay = capture / spool / forward service**
- **axiomRams = file-first execution runtime**

## AxiomSync가 해야 할 핵심만 남기면

1. raw event ledger 저장
2. canonical `session / entry / artifact / anchor` projection
3. `episode / insight / verification / procedure` 파생
4. evidence-backed retrieval
5. read-only query surface (CLI / HTTP / MCP)
6. replay / rebuild / repair / migration

## AxiomSync가 하면 안 되는 것

- connector polling / watch / sync
- browser extension ownership
- spool / retry / dead-letter
- approval queue
- execution orchestration
- Rams run state canonical ownership
- product UI / service branding

## 왜 이 구조가 맞나

`conv_*`만으로 가면 Rams가 왜곡되고, `run_*`만으로 가면 Relay가 왜곡된다.  
그래서 storage core는 generic하게 두고, query semantics는 conversation/episode 중심으로 유지해야 한다.

## 세 시스템의 연결

```text
AxiomRelay ---- append_raw_events ----> AxiomSync <---- append_raw_events ---- axiomRams
    |                                         |
    |                                         +---- MCP / HTTP / CLI query
    +---- capture / spool / approval
```

## 정본 분리

- AxiomSync → `context.db`
- AxiomRelay → capture/spool state
- axiomRams → `state/` files

direct DB coupling은 금지한다.

## 먼저 읽을 파일

1. `01_FINAL_FORM.md`
2. `03_STORAGE_SCHEMA.md`
3. `04_API_AND_MCP_SPEC.md`
4. `05_INTEGRATION_AXIOMRELAY_AXIOMRAMS.md`
