# 패치 세트

## 0001 — narrow public surface로 축소

### 목적
shipping binary에서 connector runtime command와 connector HTTP ingest surface를 제거한다.

### 바뀌는 것
- CLI top-level `Connector` 제거
- 새 top-level `Sink` 추가
- canonical sink subcommand 추가
  - `plan-append-raw-events`
  - `apply-ingest-plan`
  - `plan-upsert-source-cursor`
  - `apply-source-cursor-plan`
- HTTP route 교체
  - `/sink/raw-events/plan`
  - `/sink/raw-events/apply`
  - `/sink/source-cursors/plan`
  - `/sink/source-cursors/apply`
- `/connectors`, `/ingest/{connector}`, `connector_ingest_router` 제거

### 기대 효과
AxiomSync가 다시 kernel답게 보인다.

---

## 0002 — explicit source cursor contract

### 목적
cursor를 ingest 부속물이 아니라 1급 operation으로 만든다.

### 새 타입
- `SourceCursorInput`
- `SourceCursorUpsertPlan`

### 새 kernel method
- `plan_upsert_source_cursor()`
- `apply_source_cursor_plan()`

### 새 tx port
- `apply_source_cursor_tx()`

### 기대 효과
external collector는 raw event append 없이도 cursor만 안전하게 갱신 가능하다.

---

## 0003 — connector runtime leakage 제거

### 목적
kernel/application service에서 connector config ownership을 제거한다.

### 바뀌는 것
- `SharedConnectorConfigPort` 제거
- `FileConnectorConfigStore` 제거 또는 external repo로 이동
- `AxiomSync::connectors_path()`, `load_connectors_config()`, `connector_status()` 제거
- `connectors.rs` 전체 externalization

### 기대 효과
AxiomSync 내부에 edge concern이 남지 않는다.

---

## 0004 — split 완결

### 목적
workspace split을 실제 진입점까지 끝낸다.

### 바뀌는 것
- `crates/axiomsync`는 composition root + tiny binary만 유지
- CLI 구현은 `axiomsync-cli`
- HTTP 구현은 `axiomsync-http`

### 기대 효과
crate boundary가 문서가 아니라 실제 제품 구조가 된다.

---

## 0005 — query noun 정리

### 목적
실제 저장 모델과 public query surface를 맞춘다.

### 권장 canonical noun
- `case`
- `thread`
- `evidence`
- `command`

### compatibility alias
- `episode`
- `runbook`

### 보류
- `run`
- `task`
- `document`

이 셋은 실제 storage와 query semantics가 생길 때 추가한다.
