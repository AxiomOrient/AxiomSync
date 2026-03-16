# Documentation Index

읽을 문서는 아래면 충분합니다.

## Start Here
- [README.md](../README.md): 저장소 요약
- [BLUEPRINT.md](./BLUEPRINT.md): 제품 목표 구조와 방향
- [IMPLEMENTATION_SPEC.md](./IMPLEMENTATION_SPEC.md): 구현 요구사항과 완료 조건
- [API_CONTRACT.md](./API_CONTRACT.md): 안정 계약
- [RUNTIME_ARCHITECTURE.md](./RUNTIME_ARCHITECTURE.md): 구조와 경계
- [RETRIEVAL_ARCHITECTURE.md](./RETRIEVAL_ARCHITECTURE.md): 검색 경로
- [APPLICATION_SERVICE_ROADMAP.md](./APPLICATION_SERVICE_ROADMAP.md): explicit service 승격 실행 스펙과 로드맵
- [APPLICATION_SERVICE_TEST_STRATEGY.md](./APPLICATION_SERVICE_TEST_STRATEGY.md): 테스트 범위, 의도, 커버 전략
- [RELEASE_RUNBOOK.md](./RELEASE_RUNBOOK.md): 출시 전 체크리스트
- [CODE_OWNERSHIP.md](./CODE_OWNERSHIP.md): 변경 시작점

## Read By Need
- Operator: Repository README
- Runtime developer: API Contract, Runtime Architecture, Retrieval Architecture, Code Ownership
- Release owner: Repository README, API Contract, Release Runbook, `scripts/quality_gates.sh`, `scripts/release_pack_strict_gate.sh`

## Operator Commands
- 진단:
  `axiomsync doctor storage --json`
  `axiomsync doctor retrieval --json`
- 마이그레이션:
  `axiomsync migrate inspect --json`
  `axiomsync migrate apply --backup-dir <dir> --json`
- 릴리스 사전 확인:
  `axiomsync release verify --json`

## Repository Boundary
- This repository owns the runtime library and CLI only.
- Web and mobile companion projects are external.
- 활성 문서는 `docs/` 에 둔다.

## Documentation Rules
- 계약은 `API_CONTRACT.md`
- 구조는 `RUNTIME_ARCHITECTURE.md`
- 검색은 `RETRIEVAL_ARCHITECTURE.md`
- 소유 경계는 `CODE_OWNERSHIP.md`
- 운영 중인 설계 문서는 `docs/`

## Execution Artifacts
- 실행 계획은 `plans/IMPLEMENTATION-PLAN.md`
- 실행 태스크 ledger는 `plans/TASKS.md`
- explicit service 경계는 `src/client/{runtime,resource,search,release/verify_service}.rs` 와 `src/client/facade.rs` 에서 확인할 수 있다.
