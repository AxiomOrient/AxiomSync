# AxiomSync 구현 감사 + 패치 패키지

이 문서는 감사 시점 판단을 담은 historical artifact다.
현재 release-facing final form은 `../axiomsync-final-form-docs-package`에 정리한다.

## 한 줄 결론

현재 공개 구현은 **커널 코어는 좋다. 하지만 최종형은 아직 아니다.**

좋은 점:
- raw event ledger
- canonical conversation projection
- episode / insight / verification
- evidence anchor
- runbook retrieval
- MCP
- multi-crate workspace

미완 점:
- shipping binary가 아직 connector runtime(`sync/watch/serve/repair`)을 소유한다
- README가 선언한 좁은 `sink` surface가 실제 CLI/HTTP에 반영되지 않았다
- `source_cursor`는 `IngestPlan` 내부 부속물일 뿐, 1급 plan/apply contract가 아니다
- kernel이 아직 `connectors.toml`과 connector status를 안쪽으로 끌어들인다
- UI/MCP/public query noun이 아직 runbook/episode 중심이고, surface 정리가 덜 끝났다
- `axiomsync-cli`, `axiomsync-http` crate split이 workspace에는 있으나 실제 진입점은 아직 old in-crate module을 사용한다

## 이번 패키지의 목적

1. 실제 구현 파일 기준 감사 결과를 남긴다  
2. 최종형에 못 미치는 부분만 정확히 추린다  
3. 과한 재설계 대신 **핵심 경계만 닫는 패치**를 제안한다  
4. hand-authored patch-style diff와 Rust skeleton을 같이 준다  

## 우선순위

### P0
- public write boundary를 `sink`로 고정
- connector runtime을 shipping surface에서 제거
- explicit `SourceCursorUpsertPlan` 추가

### P1
- `axiomsync-cli` / `axiomsync-http` split을 실제 entrypoint에 연결
- web UI와 MCP를 실제 저장 모델에 맞춰 단순화

### P2
- `case` canonical alias를 추가하되, 실제 저장하지 않는 `run/task/document`는 억지로 1급 시민화하지 않는다

## 포함물

- `01_SCOPE_AND_METHOD.md`
- `02_IMPLEMENTATION_AUDIT.md`
- `03_GAP_MATRIX_AND_DECISIONS.md`
- `04_PATCHSET.md`
- `05_KERNEL_BOUNDARY_BLUEPRINT.md`
- `06_SELF_REVIEW.md`
- `patches/*.patch`
- `schema/*.json`
- `proposed/rust/*.rs`

## 적용 원칙

- 단순함 > 과장된 범용성
- 커널 내부 지식 모델링 유지
- edge capture / sync / retry / approval 제거
- 실제 저장/조회되는 noun만 public contract에 올린다
- docs를 맞추기 위해 스키마를 부풀리지 않는다
