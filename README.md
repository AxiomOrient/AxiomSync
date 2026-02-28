# AxiomMe

## 개요
AxiomMe는 로컬 우선(local-first) 컨텍스트 런타임입니다.
핵심 목표는 일관된 URI 모델(`axiom://`)과 결정적 실행 경로를 기반으로, 인제스트/검색/세션 메모리/릴리즈 게이트를 안정적으로 운영하는 것입니다.

## 빠른 시작
```bash
cargo run -p axiomme-cli -- --help
cargo run -p axiomme-cli -- init
cargo run -p axiomme-cli -- add ./README.md --target axiom://resources/repo --wait true
cargo run -p axiomme-cli -- find "context runtime"
```

## 아키텍처
상세 구조는 [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)에서 관리합니다.

상위 경계:
- `axiomme-core`: 데이터 모델, 저장소, 검색/세션/릴리즈 로직
- `axiomme-cli`: 운영자/자동화용 명령 경계
- `axiomme-mobile-ffi`: 모바일 네이티브 연동 경계

## 핵심 모듈
- [crates/axiomme-core](crates/axiomme-core/README.md)
- [crates/axiomme-cli](crates/axiomme-cli/README.md)
- [crates/axiomme-mobile-ffi](crates/axiomme-mobile-ffi/README.md)

## 설치/실행
사전 요구사항:
- Rust toolchain (stable)
- `jq` (일부 스크립트)
- `cargo-audit`

검증:
```bash
bash scripts/quality_gates.sh
bash scripts/release_pack_strict_gate.sh --workspace-dir . --output logs/release_pack_strict_report.json
scripts/release_signoff_status.sh --report-path docs/RELEASE_SIGNOFF_STATUS.md
```

## 사용 예시
문서 편집:
```bash
cargo run -p axiomme-cli -- document load axiom://resources/repo/README.md --mode markdown
cargo run -p axiomme-cli -- document save axiom://resources/repo/README.md --mode markdown --content "# Updated"
```

웹 핸드오프:
```bash
cargo run -p axiomme-cli -- web --host 127.0.0.1 --port 8787
```

컨텍스트셋 벤치마크:
```bash
bash scripts/contextset_random_benchmark.sh \
  --dataset <dataset-path> \
  --report-path logs/benchmarks/contextset_random.md
```

## 운영/품질
- 품질 게이트: `scripts/quality_gates.sh`
- strict 릴리즈 게이트: `scripts/release_pack_strict_gate.sh`
- 릴리즈 사인오프 상태: [docs/RELEASE_SIGNOFF_STATUS.md](docs/RELEASE_SIGNOFF_STATUS.md)
- 문서 인덱스: [docs/README.md](docs/README.md)

## 제약/향후 작업
- 웹 뷰어 구현체는 외부 프로젝트에 위치하며 CLI는 핸드오프만 담당합니다.
- 대형 모듈 분해는 `session/commit`과 `release_gate` 1차 완료 상태이며, `retrieval/expansion.rs`와 `retrieval/planner.rs`는 릴리즈 이후 경계 재분리 후보입니다.
- 운영 산출물은 `docs/archive/` 또는 `logs/`로 분리해 canonical 문서 노이즈를 최소화합니다.
