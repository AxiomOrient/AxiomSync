# 공통 파서 헬퍼 가이드

## 목적
여러 모듈에서 중복으로 쓰이는 문자열 파싱 규칙을 `crates/axiomsync/src/text.rs`의 공통 유틸로 통합해,
재사용률을 높이고 동작 차이를 줄이는 규칙 가이드입니다.

## 사용 규칙
- 문자열 정규화(공백 제거, 빈 문자열 제외)는 `normalize_token`을 사용한다.
- 문자열 정규화 + 소문자 정규화는 `normalize_token_ascii_lower`를 사용한다.
- 공백/빈 값은 기본값으로 대체해야 할 때 `normalize_token_or_default` 또는
  `normalize_token_ascii_lower_or_default`를 사용한다.
- 모듈별 enum 파싱이 필요하면 `parse_with_default`를 우선 사용한다.
- boolean 플래그 문자열 해석은 `parse_bool_like_flag`로 통일한다.

## 변경된 적용 위치
- `crates/axiomsync/src/session/memory_extractor.rs`
- `crates/axiomsync/src/session/commit/types.rs`
- `crates/axiomsync/src/session/om/scope_binding.rs` (scope binding 파싱)
- `crates/axiomsync/src/session/om.rs`
- `crates/axiomsync/src/client/search/reranker.rs`
- `crates/axiomsync/src/client/search/snapshot.rs`

- `crates/axiomsync/src/text.rs`
  - `normalize_token`
  - `normalize_token_ascii_lower`
  - `normalize_token_or_default`
  - `normalize_token_ascii_lower_or_default`
  - `parse_with_default`
  - `parse_bool_like_flag`

## 메모리 dedup 모드 동작 가이드

- `AXIOMSYNC_MEMORY_DEDUP_MODE`는 다음 값을 허용한다.
  - `auto` (기본값): LLM 시도 실패 시 `auto` 모드에서는 휴리스틱 fallback이 가능합니다.
  - `llm` 또는 `model`: LLM 기반 dedup를 사용한다.
  - `deterministic`: 완전 결정론적 merge 후보 선별만 수행한다.
- `""` 또는 미지정: 기본값 `auto`.
- 그 외 문자열: `auto`로 fallback되며, 세션 커밋 경로에서 1회 `memory_dedup_config` dead letter 이벤트를 남긴다.
  - 이벤트 payload: `mode_requested`, `mode_selected`, `error`(`unsupported memory dedup mode; falling back to auto`)

## 사용 시 리뷰 포인트
1. 동일 정규화 로직을 각 파일에서 또 구현했는지 먼저 검색한다.
2. enum 파싱에서 `unwrap_or(default)` 대신 `parse_with_default`로 분기 처리를 통일했는지 확인한다.
3. 새 파서를 추가할 때는 반드시 `crates/axiomsync/src/text.rs`에 테스트를 추가한다.

## 재사용 강제에 대한 운영 대안 (권장)
- 공통 헬퍼는 `crate::text`로 모듈화해, 새 파싱 로직이 생길 때마다 이 파일을 먼저 확인한다.
- `dead_code`는 기본적으로 사용되지 않는 항목에만 경고를 띄우므로, 문서만으로는 재사용 강제가 어렵습니다.
- 권장 가드(실행 기준):
  - `cargo fmt --all -- --check`
  - `cargo test -p axiomsync`
  - `cargo clippy --all-targets -- -D warnings`
- `cargo clippy --all-targets -- -D warnings`는 `dead_code` 및 중복/비권장 패턴을 실패로 바꿔, 리뷰 전에 바로 차단합니다.
- PR에서 파서 계열 변경이 있으면 위 명령어 3개를 통과해야 병합 체크리스트를 완료합니다.
