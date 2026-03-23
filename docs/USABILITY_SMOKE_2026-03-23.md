# Usability Smoke 2026-03-23

## Goal
실사용자처럼 AxiomSync를 처음부터 실행해 보고, 실제 사용 경로에서 생기는 마찰을 기록한 뒤 바로 개선한다.

## Environment
- workspace: `/Users/axient/repository/AxiomSync`
- temp root: `/var/folders/4z/9ns2598n15s5v2t8g3htr5x00000gn/T/tmp.R5hN2kazXx`
- fixture: `axiomsync-final-form-docs-package/examples/raw_event.chatgpt_selection.json`

## Usage Log
1. `cargo run -p axiomsync -- --help`
   - CLI 루트 명령은 정상 출력됐다.
   - 첫 빌드에서 cargo package/build lock 대기가 보였다.
2. `cargo run -p axiomsync -- --root <tmp_root> init`
   - `context.db`, `auth.json`이 생성됐다.
3. `cargo run -p axiomsync -- --root <tmp_root> project doctor`
   - 빈 store 상태가 정상 보고됐다.
4. `cargo run -p axiomsync -- --root <tmp_root> sink plan-append-raw-events --file axiomsync-final-form-docs-package/examples/raw_event.chatgpt_selection.json`
   - ingest plan이 stdout JSON으로 정상 출력됐다.
5. `cargo run -p axiomsync -- --root <tmp_root> sink apply-ingest-plan --file <tmp_root>/ingest-plan.json`
   - receipt 1건이 정상 반영됐다.
6. `cargo run -p axiomsync -- --root <tmp_root> project plan-rebuild > <tmp_root>/replay-plan.json`
   - replay plan이 stdout JSON으로 정상 출력됐다.
7. `cargo run -p axiomsync -- --root <tmp_root> project apply-replay-plan --file <tmp_root>/replay-plan.json`
   - projection 1 session/1 entry/1 anchor, derivation 1 episode/2 insights/1 verification이 생성됐다.
8. `cargo run -p axiomsync -- --root <tmp_root> project doctor`
   - pending counts가 모두 0으로 수렴했다.
9. `cargo run -p axiomsync -- --root <tmp_root> query search-docs --file <tmp_root>/search-docs.json`
   - `"narrow sink contract"` 조회가 evidence preview와 함께 정상 반환됐다.
10. `cargo run -p axiomsync -- --root <tmp_root> serve --addr 127.0.0.1:4410`
   - `curl http://127.0.0.1:4410/health`가 `status=ok`와 zero pending counts를 반환했다.

## Friction Found
- `sink`/`query` 하위 명령 help가 `--file <FILE>`만 보여 줘서 처음 사용하는 사람이 JSON shape를 추측해야 했다.
- `project plan-rebuild`와 `project apply-replay-plan`이 분리되면서, 사용자는 계획 생성과 실제 적용을 명시적으로 구분해야 한다.
- 루트 help에는 canonical quick start가 있었지만 실제 stdout redirection 기반 사용 예시는 없었다.

## Self-Improvement Applied
- CLI root help에 canonical quick start를 추가했다.
- `sink plan-append-raw-events`, `sink apply-ingest-plan`, `sink plan-upsert-source-cursor`, `sink apply-source-cursor-plan` help에 실제 입력 형식과 stdout/staged plan 의미를 추가했다.
- `query` 계열 help에 공통 search request JSON 예시를 추가했다.

## Follow-up Candidates
- `project plan-rebuild > replay-plan.json`와 `project apply-replay-plan --file replay-plan.json` 예시를 README와 smoke 문서에서 같이 유지하는 편이 첫 사용 마찰을 줄인다.
- README quick start에 example JSON 파일 위치를 직접 연결하면 첫 사용 마찰이 더 줄어든다.
