# AxiomSync Final Form Docs Package

## 목적

이 패키지는 **AxiomSync의 최종형 문서 세트**를 정리한다. 범위는 다음 3가지다.

1. **AxiomSync 자체의 최종형 청사진**
2. **AxiomRelay / axiomRams와의 연결 방식**
3. **구현 순서, 계약, 의사코드, 정합성 검토**

이 문서는 **정책 문서가 아니라 구현 지침 문서**다.  
현재 공개 코드 커밋 감사 결과를 주장하지 않는다. 대신 업로드된 컨텍스트와 이전 설계 산출물에서 일치하는 핵심만 남겨 **정규 설계**로 재구성했다.

## 한 문장 정의

> **AxiomSync는 local-first conversation-native knowledge kernel이며, immutable raw event ledger를 저장하고, session/entry/artifact/anchor를 canonical projection으로 유지하며, episode/insight/verification/procedure를 evidence-backed memory로 파생하고, read-only query surface를 CLI/HTTP/MCP로 제공한다.**

## 읽는 순서

1. `01_FINAL_FORM.md`
2. `02_BLUEPRINT.md`
3. `03_STORAGE_SCHEMA.md`
4. `04_API_AND_MCP_SPEC.md`
5. `05_INTEGRATION_AXIOMRELAY_AXIOMRAMS.md`
6. `06_ROADMAP.md`
7. `07_PSEUDOCODE_SPEC.md`
8. `08_CONSISTENCY_REVIEW.md`

## 포함 파일

- `01_FINAL_FORM.md` — 최종형 정의
- `02_BLUEPRINT.md` — 구조도 / 경계 / repo layout
- `03_STORAGE_SCHEMA.md` — 저장 책임과 핵심 스키마
- `04_API_AND_MCP_SPEC.md` — ingest/query/MCP 계약
- `05_INTEGRATION_AXIOMRELAY_AXIOMRAMS.md` — 연결 방식과 사용 흐름
- `06_ROADMAP.md` — 구현 로드맵
- `07_PSEUDOCODE_SPEC.md` — 핵심 알고리즘 의사코드
- `08_CONSISTENCY_REVIEW.md` — 셀프 피드백과 충돌 검토
- `schema/axiomsync_kernel_vnext.sql` — 최소 SQL skeleton
- `schema/kernel_sink_contract.json` — ingest contract JSON skeleton
- `examples/*.json` — 연동 예시 payload
