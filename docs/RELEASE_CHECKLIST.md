# Release Checklist

이 문서는 릴리스 오너가 지금 저장소에서 실제로 따라야 하는 최소 절차만 적습니다.

## Preflight
- 작업 디렉터리와 `--workspace-dir` 가 같은 저장소를 가리키는지 확인한다.
- `jq` 가 설치되어 있는지 확인한다.
- `scripts/quality_gates.sh` 를 실행할 계획이면 `cargo-audit` 가 설치되어 있는지 확인한다.
- 릴리스 게이트는 `<root>/context.db` 기준으로 새 루트를 만들어 검증하므로, 기존 로컬 상태를 재사용하지 않는다.

## Required Gates
```bash
bash scripts/quality_gates.sh
bash scripts/release_pack_strict_gate.sh --workspace-dir "$(pwd)"
```

## Release Decision Rules
- `quality_gates.sh` 는 포맷, clippy, workspace tests, dependency audit, mirror notice smoke 를 통과해야 한다.
- `release_pack_strict_gate.sh` 는 `release pack --enforce` 를 실행하고 JSON 보고서의 `.passed == true` 여야 한다.
- 둘 중 하나라도 실패하면 출시하지 않는다.

## Evidence To Keep
- `quality_gates.sh` 표준 출력 로그
- `release_pack_strict_gate.sh` JSON 출력 또는 `--output <path>` 로 저장한 보고서
- 필요 시 `axiom://queue/release/packs/...` 문서 URI

## Retrieval-Specific Checks In This Release Line
- `FindResult.query_results + hit_buckets` 가 canonical result shape 인지 확인한다.
- JSON 응답에 `memories`, `resources`, `skills` 호환 배열이 계속 직렬화되는지 확인한다.
- repo mount 이후 파일 인덱싱 결과가 namespace 필터에서 보이는지 확인한다.
- `repo mount` 기본 동작이 인덱싱 완료까지 기다리는지 확인한다.

## Do Not Ignore
- legacy DB 파일명 탐색이나 자동 마이그레이션은 지원하지 않는다.
- benchmark gate 가 `no_benchmark_reports` 를 반환하면 release pack 정책을 먼저 확인하고, 필요하면 benchmark artifact를 생성한 뒤 다시 실행한다.
