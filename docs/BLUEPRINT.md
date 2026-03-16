# Product Blueprint

## 이상적인 목표 구조

이 프로젝트의 이상적인 구조는 **“단일 제품, 단일 정본(SQLite), 다중 파생 계층(runtime index / FTS / release evidence)”** 이다.

### 핵심 철학
- **로컬 퍼스트**
- **axiom://** 중심의 명시적 자원 주소 체계
- **context.db** 단일 정본
- **runtime retrieval**은 메모리 우선
- **세션/OM 상태** 명시화
- **외부 companion**은 저장소 밖
- **vendored OM 경계** 명시적 격리

## 바꿔야 할 구조 원칙

- “한 크레이트에 다 넣기”는 유지 가능하되, **public contract**와 **internal ops contract**를 분리해야 한다.
- **domain service**와 **CLI handler**를 직접 연결하지 말고, **application service**를 거치게 해야 한다.
- **compatibility serialization**은 core model이 아니라 presentation layer의 책임으로 내려야 한다.
- **migration/release verification**은 문서가 아니라 실행 가능한 정책 객체가 되어야 한다.

## 권장 레이어 구조

### Interface Layer
- CLI
- future FFI / JSON adapter

### Application Layer
- RuntimeBootstrapService
- ResourceService
- EventService
- LinkService
- RepoMountService
- SearchService
- SessionService
- ReleaseVerificationService

주석:
- Phase 1 완료 서비스: Event/Link/Repo/Archive
- Phase 2 대상: Resource/Search/Session/Runtime/ReleaseVerification

### Domain Layer
- URI / Scope
- Namespace / Kind
- Resource / Event / Link / Session / Search contracts

### Persistence Layer
- StateStore
- SearchProjectionStore
- QueueStore
- TraceStore
- ReleaseEvidenceStore

### Infrastructure Layer
- SQLite
- Rooted FS
- InMemoryIndex
- FTS Projection
- Host tools

## 권장 데이터 흐름
1. **Input(CLI/API)**
2. -> **validate**
3. -> **bootstrap mode resolve**
4. -> **domain/application service**
5. -> **SQLite write** (source of truth)
6. -> **search projection update**
7. -> **runtime index sync or restore**
8. -> **query / trace / evidence output**

## 권장 모듈 경계
- `client.rs`는 composition root만 담당
- `client/facade.rs`는 orchestration only
- `state/*`는 “저장”과 “projection”을 분리
- `models/*`는 pure data contract만 유지
- `commands/*`는 입출력 포맷과 validation만 담당
- `release_gate/*`는 “실행 가능한 정책”만 담당

## 권장 계약/API 원칙
- **canonical public API**는 하나만 둔다.
- **compatibility API**는 명시적 버전/플래그로만 유지
- **destructive operation**(export+compact)은 plan/confirm/execute 2단계로 노출
- **release gate**와 **migration 진단**은 항상 JSON 출력 지원
- **planner**는 scope decision evidence를 trace에 반드시 포함
