# Documentation Index

릴리스 기준 활성 문서는 아래 목록이면 충분합니다.

- [README.md](../README.md): 제품 요약과 빠른 시작
- [API_CONTRACT.md](./API_CONTRACT.md): 현재 public surface
- [KERNEL_SINK_CONTRACT.md](./KERNEL_SINK_CONTRACT.md): canonical raw-only write boundary
- [RUNTIME_ARCHITECTURE.md](./RUNTIME_ARCHITECTURE.md): `record -> view -> knowledge` 구조와 crate 경계
- [TESTING.md](./TESTING.md): 검증 명령과 회귀 테스트
- [RELEASE_RUNBOOK.md](./RELEASE_RUNBOOK.md): 출시 전 최종 체크

`docs/`에는 현재 구현과 직접 맞는 문서만 유지한다.
루트에 있는 review package나 설계 참고 자료는 현재 릴리스 계약이 아니라 배경 자료로만 취급한다.

## Stale Code Audit
- 현재 빌드 경로가 아닌 후보:
  - `crates/axiomsync-store-sqlite/src/context_db/`
  - `crates/axiomsync-domain/src/domain/`
- 이 경로들은 참고용 구버전 구현 흔적일 수 있으므로, 삭제 전에는 import graph와 테스트 참조를 다시 확인한다.
