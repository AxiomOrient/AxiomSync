# Build Artifact Control

이 문서는 Rust workspace에서 `target/`이 비정상적으로 커질 때, 원인을 빨리 좁히고 같은 문제를 다른 저장소에서도 반복해서 줄일 수 있게 하는 운영 절차를 정리합니다.

## When To Use This

아래 증상이 보이면 이 문서를 그대로 적용합니다.

- `target/debug/deps`에 큰 해시 파일이 수백 개 생김
- `target/debug/incremental`이 수백 MB 이상 커짐
- `cargo test --workspace` 한 번으로 `target/debug`가 1GB 이상 증가
- `*.rcgu.o`, `*.rlib`, 해시가 붙은 test/bin 실행 파일이 반복 생성됨

## Measure First

느낌으로 보지 말고 먼저 같은 명령으로 baseline을 잡습니다.

```bash
cargo clean
cargo test --workspace --quiet >/dev/null

du -sh target/debug target/debug/deps target/debug/incremental target/debug/build
find target/debug/deps -maxdepth 1 -type f -size +20M -print0 | xargs -0 ls -lh | sort -k5 -h | tail -40
find target/debug/deps -maxdepth 1 -type f -name '*.rcgu.o' | wc -l
```

## Read The Artifacts

- `*.rcgu.o`
  - Rust CodeGen Unit object file
  - 큰 crate가 codegen unit으로 쪼개져 만든 중간 산출물
- `lib*.rlib`
  - crate archive
- `*.rmeta`
  - metadata artifact
- 해시가 붙은 실행 파일
  - bin/test target 링크 결과
- `target/debug/incremental`
  - incremental compilation cache

`rcgu.o`가 많다는 건 보통 crate가 크고, 같은 crate가 여러 빌드 문맥에서 반복 컴파일된다는 뜻입니다.

## What Worked Here

이번 저장소에서는 두 단계를 먼저 적용했습니다.

### 1. Remove duplicated cargo work

[scripts/quality_gates.sh](../scripts/quality_gates.sh) 에서 중복 빌드를 줄였습니다.

- workspace test에 이미 포함된 단일 테스트 재실행 제거
- 정보성 notice 확인 때문에 strict release pack 전체를 다시 돌리던 경로 제거

중복 경로를 먼저 없애는 이유는 가장 안전하고 ROI가 높기 때문입니다.

### 2. Reduce dev/test artifact size directly

`Cargo.toml`에 아래 profile 설정을 추가했습니다.

```toml
[profile.dev]
debug = 0
incremental = false
codegen-units = 64

[profile.test]
debug = 0
incremental = false
codegen-units = 64
```

의미:

- `debug = 0`
  - debug 심볼 제거
  - `rlib`, binary, object 파일 크기 감소
- `incremental = false`
  - incremental cache 제거
  - `target/debug/incremental` 폭증 방지
- `codegen-units = 64`
  - giant crate의 codegen shard churn 완화

## Before / After From This Repository

동일 기준:

```bash
cargo clean
cargo test --workspace --quiet >/dev/null
```

결과:

- `target/debug`: `1.5G` -> `606M`
- `target/debug/deps`: `991M` -> `510M`
- `target/debug/incremental`: `397M` -> `0B`
- `target/debug/build`: `66M` -> `56M`
- largest `libaxiomsync-*.rlib`: `70M` -> `29M`
- largest `axiomsync-*` binaries: `44M` -> `36~37M`
- visible `axiomsync-*.rcgu.o`: `923` -> `0`

## Reusable Rollout Order

다른 Rust 프로젝트에 적용할 때도 순서는 같습니다.

1. `cargo clean` 후 대표 명령 한 개로 baseline을 잡는다.
2. `deps`와 `incremental` 중 어디가 큰지 먼저 본다.
3. 스크립트가 같은 `cargo build/test/clippy/run`을 중복 호출하는지 제거한다.
4. 그래도 크면 `profile.dev`, `profile.test`에서 `debug = 0`, `incremental = false`를 검토한다.
5. 그래도 크면 crate 분해로 넘어간다.

## Tradeoffs

- `incremental = false`
  - 장점: 디스크 사용량 크게 감소
  - 단점: 반복 로컬 재빌드가 느려질 수 있음
- `debug = 0`
  - 장점: artifact 크기 감소
  - 단점: 로컬 디버깅 정보 감소

저장 공간이 우선이면 위 설정을 유지하고, 디버깅이 우선이면 CI 전용 profile이나 환경 변수로만 적용하는 것이 낫습니다.

## Verification Contract

용량이 줄었더라도 아래 검증을 다시 통과해야 합니다.

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
bash scripts/quality_gates.sh
du -sh target/debug target/debug/deps target/debug/incremental target/debug/build
```
