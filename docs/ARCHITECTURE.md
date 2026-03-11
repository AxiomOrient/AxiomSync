# AxiomNexus Architecture

## 1. Intent
AxiomNexus는 로컬 우선 컨텍스트 런타임입니다.  
핵심 목표는 `axiom://` URI 기반의 일관된 데이터 모델, 예측 가능한 검색, 명시적인 상태 전이입니다.

## 2. Core Data Model
- `AxiomUri`: `axiom://{scope}/{path}` 형식의 정규 식별자
- `Scope`: `resources|user|agent|session|temp|queue`
- `OmRecord`/`OmObservationEntry`: OM 런타임 상태와 entry 기반 메모리 단위
- `FindResult`: 검색 결과 + 추적(Trace) 메타데이터

## 3. Runtime Layers
1. Interface
- `axiomnexus-cli`
- `axiomnexus-mobile-ffi`

2. Coordinator
- `AxiomNexus` facade가 FS/State/Index/Session 경계를 오케스트레이션

3. Storage/Logic
- `LocalContextFs`: 파일 I/O 경계
- `SqliteStateStore`: 큐/체크포인트/OM 영속 상태
- `InMemoryIndex`: 검색용 메모리 인덱스
- Retrieval pipeline: 정해진 정책으로 find/search 실행

## 4. Data Flow
1. Ingest: 리소스 등록 -> outbox enqueue
2. Replay: queue consume -> index/state 동기화
3. Query: memory index 검색 + trace 생성
4. Session/OM: 관찰/반영 -> continuation/entry 상태 갱신

## 5. Boundary Rules
- 부수효과는 `Fs/State` 레이어에만 둡니다.
- 검색/선택/계약 검증은 가능한 순수 변환으로 유지합니다.
- `episodic`은 pure OM contract/transform 계층, AxiomNexus는 런타임/영속 계층입니다.
