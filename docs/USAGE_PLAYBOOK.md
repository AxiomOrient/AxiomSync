# Usage Playbook

이 문서는 AxiomNexus를 실제 운영에 투입할 때의 기본 사용 전략을 정리합니다.

## 1) 프로젝트별 루트 분리란?
`--root` 경로를 프로젝트마다 다르게 두는 방식입니다.

- 예: `proj-a -> ~/work/proj-a/.axiomnexus`, `proj-b -> ~/work/proj-b/.axiomnexus`
- 효과:
  - 데이터/인덱스/큐/세션이 프로젝트 단위로 완전히 분리됨
  - 권한, 백업, 폐기(삭제) 작업이 프로젝트 단위로 단순해짐
  - 교차 검색은 자동으로 되지 않음(의도적 격리)

## 2) 언제 분리하고, 언제 공유할까?

### 루트 분리 권장
- 보안/규정/고객 데이터 경계가 중요할 때
- 프로젝트 수명주기가 서로 다를 때
- 팀/계정별 접근 권한을 분리해야 할 때

### 공유 루트 권장
- 여러 프로젝트를 횡단 검색해야 할 때
- 공통 문서/패턴 재사용이 잦을 때

## 3) 운영 패턴 A: 프로젝트별 루트 분리
```bash
# project A
cargo run -p axiomnexus-cli -- --root ~/work/proj-a/.axiomnexus init
cargo run -p axiomnexus-cli -- --root ~/work/proj-a/.axiomnexus add ~/work/proj-a/docs --target axiom://resources/docs
cargo run -p axiomnexus-cli -- --root ~/work/proj-a/.axiomnexus search "release gate"

# project B
cargo run -p axiomnexus-cli -- --root ~/work/proj-b/.axiomnexus init
cargo run -p axiomnexus-cli -- --root ~/work/proj-b/.axiomnexus add ~/work/proj-b/docs --target axiom://resources/docs
cargo run -p axiomnexus-cli -- --root ~/work/proj-b/.axiomnexus search "incident runbook"
```

## 4) 운영 패턴 B: 공유 루트 + 프로젝트 네임스페이스
```bash
ROOT="$HOME/.axiomnexus-workspace"
cargo run -p axiomnexus-cli -- --root "$ROOT" init

cargo run -p axiomnexus-cli -- --root "$ROOT" add ~/work/proj-a/docs --target axiom://resources/projects/proj-a/docs
cargo run -p axiomnexus-cli -- --root "$ROOT" add ~/work/proj-b/docs --target axiom://resources/projects/proj-b/docs

cargo run -p axiomnexus-cli -- --root "$ROOT" search "auth timeout" --target axiom://resources/projects/proj-a
cargo run -p axiomnexus-cli -- --root "$ROOT" search "auth timeout" --target axiom://resources/projects/proj-b
```

## 5) 세션 운용 원칙
세션은 "작업 중 단기 타임라인", memories는 "장기 재사용 지식"으로 분리합니다.

```bash
SID="$(cargo run -q -p axiomnexus-cli -- --root ~/work/proj-a/.axiomnexus session create)"

cargo run -p axiomnexus-cli -- --root ~/work/proj-a/.axiomnexus session add --id "$SID" --role user --text "배포 전 최종 점검"
cargo run -p axiomnexus-cli -- --root ~/work/proj-a/.axiomnexus session add --id "$SID" --role tool --text "e2e: passed 42, flaky 1"

cargo run -p axiomnexus-cli -- --root ~/work/proj-a/.axiomnexus search "flaky" --session "$SID"
cargo run -p axiomnexus-cli -- --root ~/work/proj-a/.axiomnexus session commit --id "$SID"
```

권장:
- 세션: 진행 중 판단/로그/실험 결과
- `axiom://user|agent/memories`: 반복 가치가 확인된 사실/규칙만 승격

## 6) 최소 운영 체크리스트
- `--root` 전략을 먼저 고정한다(분리 or 공유).
- `resources` 경로 규칙을 팀 규약으로 통일한다.
- 세션 커밋을 배포/작업 단위 완료 시점에 수행한다.
- 정기적으로 `cargo run -p axiomnexus-cli -- --help` 및 release gate 명령으로 운용 표면을 재확인한다.
