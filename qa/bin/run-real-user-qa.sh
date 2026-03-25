#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cli_bin="$repo_root/target/debug/axiomsync-cli"
fixtures_dir="$repo_root/qa/fixtures"
qa_output_root="${AXIOMSYNC_QA_OUTPUT_ROOT:-$repo_root/target/qa}"
run_id="$(date +%Y%m%d-%H%M%S)"
run_root="$qa_output_root/$run_id"
report_file="$run_root/report.md"
http_port="${AXIOMSYNC_QA_HTTP_PORT:-4410}"
relay_port="${AXIOMSYNC_QA_RELAY_PORT:-4411}"

declare -a server_pids=()
declare -a mcp_pids=()

fail() {
  printf 'QA FAIL: %s\n' "$*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

append_report() {
  printf '%s\n' "$*" >> "$report_file"
}

assert_eq() {
  local actual="$1"
  local expected="$2"
  local message="$3"
  if [[ "$actual" != "$expected" ]]; then
    fail "$message (expected=$expected actual=$actual)"
  fi
}

assert_contains() {
  local value="$1"
  local needle="$2"
  local message="$3"
  if [[ "$value" != *"$needle"* ]]; then
    fail "$message (value=$value needle=$needle)"
  fi
}

cleanup() {
  local pid
  for pid in "${server_pids[@]:-}"; do
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  done
  for pid in "${mcp_pids[@]:-}"; do
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  done
}

trap cleanup EXIT

wait_for_health() {
  local port="$1"
  local url="http://127.0.0.1:$port/health"
  local attempt
  for attempt in $(seq 1 50); do
    if curl -fsS "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.2
  done
  fail "server did not become healthy on port $port"
}

start_server() {
  local root="$1"
  local port="$2"
  local log_file="$3"
  "$cli_bin" --root "$root" serve --addr "127.0.0.1:$port" >"$log_file" 2>&1 &
  server_pids+=("$!")
  wait_for_health "$port"
}

write_json() {
  local file="$1"
  local content="$2"
  printf '%s\n' "$content" > "$file"
}

run_cli_scenario() {
  local scenario_root="$run_root/cli"
  mkdir -p "$scenario_root"

  "$cli_bin" --root "$scenario_root" init > "$scenario_root/init.json"
  "$cli_bin" --root "$scenario_root" sink plan-append-raw-events --file "$fixtures_dir/cli-raw-events.json" > "$scenario_root/ingest-plan.json"
  "$cli_bin" --root "$scenario_root" sink apply-ingest-plan --file "$scenario_root/ingest-plan.json" > "$scenario_root/ingest-apply.json"
  "$cli_bin" --root "$scenario_root" project plan-rebuild > "$scenario_root/replay-plan.json"
  "$cli_bin" --root "$scenario_root" project apply-replay-plan --file "$scenario_root/replay-plan.json" > "$scenario_root/replay-apply.json"
  "$cli_bin" --root "$scenario_root" project doctor > "$scenario_root/doctor.json"
  write_json "$scenario_root/search-cases.json" '{"query":"config drift","filter":{"workspace_root":"/workspace/cli-demo"},"limit":10}'
  "$cli_bin" --root "$scenario_root" query search-cases --file "$scenario_root/search-cases.json" > "$scenario_root/search-result.json"

  assert_eq "$(jq '.receipts | length' "$scenario_root/ingest-plan.json")" "2" "cli ingest receipts mismatch"
  assert_eq "$(jq '.projection.entries' "$scenario_root/replay-apply.json")" "2" "cli replay entries mismatch"
  assert_eq "$(jq '.pending_projection_count' "$scenario_root/doctor.json")" "0" "cli pending projection should be zero"
  assert_eq "$(jq '.pending_derived_count' "$scenario_root/doctor.json")" "0" "cli pending derivation should be zero"
  assert_eq "$(jq '.pending_index_count' "$scenario_root/doctor.json")" "0" "cli pending index should be zero"
  assert_eq "$(jq 'length' "$scenario_root/search-result.json")" "1" "cli search should return one case hit"

  append_report "## cli"
  append_report "- root: \`$scenario_root\`"
  append_report "- receipts planned: $(jq '.receipts | length' "$scenario_root/ingest-plan.json")"
  append_report "- projection entries: $(jq '.projection.entries' "$scenario_root/replay-apply.json")"
  append_report "- doctor: \`$(jq -c . "$scenario_root/doctor.json")\`"
  append_report "- first hit: \`$(jq -r '.[0].title' "$scenario_root/search-result.json")\`"
  append_report
}

run_http_scenario() {
  local scenario_root="$run_root/http"
  local http_dir="$scenario_root/http-checks"
  mkdir -p "$scenario_root" "$http_dir"

  "$cli_bin" --root "$scenario_root" init > "$scenario_root/init.json"
  "$cli_bin" --root "$scenario_root" sink plan-append-raw-events --file "$fixtures_dir/http-workspace-a.json" > "$scenario_root/team-a-ingest-plan.json"
  "$cli_bin" --root "$scenario_root" sink apply-ingest-plan --file "$scenario_root/team-a-ingest-plan.json" > "$scenario_root/team-a-ingest-apply.json"
  "$cli_bin" --root "$scenario_root" sink plan-append-raw-events --file "$fixtures_dir/http-workspace-b.json" > "$scenario_root/team-b-ingest-plan.json"
  "$cli_bin" --root "$scenario_root" sink apply-ingest-plan --file "$scenario_root/team-b-ingest-plan.json" > "$scenario_root/team-b-ingest-apply.json"
  "$cli_bin" --root "$scenario_root" project plan-rebuild > "$scenario_root/replay-plan.json"
  "$cli_bin" --root "$scenario_root" project apply-replay-plan --file "$scenario_root/replay-plan.json" > "$scenario_root/replay-apply.json"

  "$cli_bin" --root "$scenario_root" project plan-auth-grant --workspace-root /workspace/team-a --token team-a-token > "$scenario_root/team-a-auth-plan.json"
  "$cli_bin" --root "$scenario_root" project apply-auth-grant-plan --file "$scenario_root/team-a-auth-plan.json" > "$scenario_root/team-a-auth-apply.json"
  "$cli_bin" --root "$scenario_root" project plan-auth-grant --workspace-root /workspace/team-b --token team-b-token > "$scenario_root/team-b-auth-plan.json"
  "$cli_bin" --root "$scenario_root" project apply-auth-grant-plan --file "$scenario_root/team-b-auth-plan.json" > "$scenario_root/team-b-auth-apply.json"
  "$cli_bin" --root "$scenario_root" project plan-admin-grant --token admin-token > "$scenario_root/admin-auth-plan.json"
  "$cli_bin" --root "$scenario_root" project apply-admin-grant-plan --file "$scenario_root/admin-auth-plan.json" > "$scenario_root/admin-auth-apply.json"

  start_server "$scenario_root" "$http_port" "$scenario_root/server.log"

  write_json "$http_dir/team-a-search.json" '{"query":"queue drift","filter":{"workspace_root":"/workspace/team-a"},"limit":10}'
  write_json "$http_dir/team-b-search.json" '{"query":"auth mismatch","filter":{"workspace_root":"/workspace/team-b"},"limit":10}'
  write_json "$http_dir/team-a-on-b.json" '{"query":"auth mismatch","filter":{"workspace_root":"/workspace/team-b"},"limit":10}'

  local base="http://127.0.0.1:$http_port"
  curl -sS "$base/health" > "$http_dir/health.json"

  local code_a code_b code_missing code_runs_ok code_cross code_admin_case team_a_case
  code_a="$(curl -sS -o "$http_dir/team-a-search.out" -w '%{http_code}' -H 'authorization: Bearer team-a-token' -H 'content-type: application/json' -d @"$http_dir/team-a-search.json" "$base/api/query/search-cases")"
  code_b="$(curl -sS -o "$http_dir/team-b-search.out" -w '%{http_code}' -H 'authorization: Bearer team-b-token' -H 'content-type: application/json' -d @"$http_dir/team-b-search.json" "$base/api/query/search-cases")"
  code_missing="$(curl -sS -o "$http_dir/runs-missing.out" -w '%{http_code}' -H 'authorization: Bearer team-a-token' "$base/api/runs")"
  code_runs_ok="$(curl -sS -o "$http_dir/runs-team-a.out" -w '%{http_code}' -H 'authorization: Bearer team-a-token' "$base/api/runs?workspace_root=/workspace/team-a")"
  code_cross="$(curl -sS -o "$http_dir/team-a-on-b.out" -w '%{http_code}' -H 'authorization: Bearer team-a-token' -H 'content-type: application/json' -d @"$http_dir/team-a-on-b.json" "$base/api/query/search-cases")"
  team_a_case="$(jq -r '.[0].id' "$http_dir/team-a-search.out")"
  code_admin_case="$(curl -sS -o "$http_dir/admin-case.out" -w '%{http_code}' -H 'authorization: Bearer admin-token' "$base/api/cases/$team_a_case")"

  assert_eq "$(jq -r '.status' "$http_dir/health.json")" "ok" "http health should be ok"
  assert_eq "$code_a" "200" "team-a search should succeed"
  assert_eq "$(jq 'length' "$http_dir/team-a-search.out")" "1" "team-a search should return one hit"
  assert_eq "$code_b" "200" "team-b search should succeed"
  assert_eq "$(jq 'length' "$http_dir/team-b-search.out")" "1" "team-b search should return one hit"
  assert_eq "$code_missing" "400" "unscoped runs should fail"
  assert_contains "$(cat "$http_dir/runs-missing.out")" "workspace_root" "unscoped runs should mention workspace_root"
  assert_eq "$code_runs_ok" "200" "scoped runs should succeed"
  assert_eq "$(jq 'length' "$http_dir/runs-team-a.out")" "1" "team-a runs should contain one run"
  assert_eq "$code_cross" "403" "cross-workspace search should be forbidden"
  assert_contains "$(jq -r '.error' "$http_dir/team-a-on-b.out")" "token does not grant access" "cross-workspace error mismatch"
  assert_eq "$code_admin_case" "403" "admin token should not read canonical workspace case route"

  if [[ "$(uname -s)" == "Darwin" ]]; then
    assert_eq "$(stat -f '%A' "$scenario_root/auth.json")" "600" "auth.json mode should be 600 on Darwin"
  elif [[ "$(uname -s)" == "Linux" ]]; then
    assert_eq "$(stat -c '%a' "$scenario_root/auth.json")" "600" "auth.json mode should be 600 on Linux"
  fi

  append_report "## http"
  append_report "- root: \`$scenario_root\`"
  append_report "- replay sessions: $(jq '.projection.sessions' "$scenario_root/replay-apply.json")"
  append_report "- team-a search code: $code_a"
  append_report "- team-b search code: $code_b"
  append_report "- missing selector code: $code_missing"
  append_report "- cross-workspace code: $code_cross"
  append_report "- admin canonical read code: $code_admin_case"
  append_report
}

run_mcp_scenario() {
  local scenario_root="$run_root/http"
  local mcp_dir="$run_root/mcp"
  mkdir -p "$mcp_dir"

  if [[ ! -f "$scenario_root/team-a-auth-plan.json" ]]; then
    run_http_scenario
  fi
  local workspace_id
  workspace_id="$(jq -r '.workspace_id' "$scenario_root/team-a-auth-plan.json")"

  local mcp_log="$mcp_dir/mcp.log"
  local mcp_in_pipe="$mcp_dir/mcp.in"
  local mcp_out_pipe="$mcp_dir/mcp.out"
  mkfifo "$mcp_in_pipe" "$mcp_out_pipe"
  "$cli_bin" --root "$scenario_root" mcp serve --transport stdio --workspace-id "$workspace_id" <"$mcp_in_pipe" >"$mcp_out_pipe" 2>"$mcp_log" &
  mcp_pids+=("$!")
  exec 3>"$mcp_in_pipe"
  exec 4<"$mcp_out_pipe"

  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' >&3
  IFS= read -r tools_line <&4 || fail "missing tools/list response"
  printf '%s\n' "$tools_line" > "$mcp_dir/tools-list.json"

  printf '%s\n' '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search_cases","arguments":{"query":"queue drift","limit":10,"filter":{"workspace_root":"/workspace/team-a"}}}}' >&3
  IFS= read -r search_line <&4 || fail "missing search_cases response"
  printf '%s\n' "$search_line" > "$mcp_dir/search-team-a.json"

  printf '%s\n' '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_runs","arguments":{"workspace_root":"/workspace/team-a"}}}' >&3
  IFS= read -r runs_line <&4 || fail "missing list_runs response"
  printf '%s\n' "$runs_line" > "$mcp_dir/list-runs-team-a.json"

  printf '%s\n' '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"list_runs","arguments":{}}}' >&3
  IFS= read -r missing_line <&4 || fail "missing invalid list_runs response"
  printf '%s\n' "$missing_line" > "$mcp_dir/list-runs-missing.json"

  local case_id
  case_id="$(jq -r '.result[0].id' "$mcp_dir/search-team-a.json")"
  printf '%s\n' "{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"resources/read\",\"params\":{\"uri\":\"axiom://cases/$case_id\"}}" >&3
  IFS= read -r case_line <&4 || fail "missing case resource response"
  printf '%s\n' "$case_line" > "$mcp_dir/read-case.json"

  printf '%s\n' '{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"search_cases","arguments":{"query":"auth mismatch","limit":10,"filter":{"workspace_root":"/workspace/team-b"}}}}' >&3
  IFS= read -r cross_line <&4 || fail "missing cross-workspace error"
  printf '%s\n' "$cross_line" > "$mcp_dir/search-team-b.json"

  assert_eq "$(jq '.result.tools | length' "$mcp_dir/tools-list.json")" "9" "mcp tool count mismatch"
  assert_eq "$(jq '.result | length' "$mcp_dir/search-team-a.json")" "1" "mcp search should return one hit"
  assert_eq "$(jq '.result | length' "$mcp_dir/list-runs-team-a.json")" "1" "mcp list_runs should return one run"
  assert_contains "$(jq -r '.error.message' "$mcp_dir/list-runs-missing.json")" "workspace_root is required" "mcp missing workspace error mismatch"
  assert_eq "$(jq -r '.result.workspace_root' "$mcp_dir/read-case.json")" "/workspace/team-a" "mcp read-case should stay in team-a"
  assert_contains "$(jq -r '.error.message' "$mcp_dir/search-team-b.json")" "outside bound workspace" "mcp cross-workspace error mismatch"

  append_report "## mcp"
  append_report "- root: \`$scenario_root\`"
  append_report "- bound workspace id: \`$workspace_id\`"
  append_report "- canonical tools: $(jq '.result.tools | length' "$mcp_dir/tools-list.json")"
  append_report "- search result case: \`$case_id\`"
  append_report "- missing selector error: \`$(jq -r '.error.message' "$mcp_dir/list-runs-missing.json")\`"
  append_report "- cross-workspace error: \`$(jq -r '.error.message' "$mcp_dir/search-team-b.json")\`"
  append_report

  exec 3>&-
  exec 4<&-
}

run_relay_scenario() {
  local scenario_root="$run_root/relay"
  local relay_dir="$scenario_root/relay-checks"
  mkdir -p "$scenario_root" "$relay_dir"

  "$cli_bin" --root "$scenario_root" init > "$scenario_root/init.json"
  start_server "$scenario_root" "$relay_port" "$scenario_root/server.log"

  local base="http://127.0.0.1:$relay_port"
  curl -sS -H 'content-type: application/json' -d @"$fixtures_dir/relay-raw-events.json" "$base/sink/raw-events/plan" > "$relay_dir/raw-plan-1.json"
  curl -sS -H 'content-type: application/json' -d @"$relay_dir/raw-plan-1.json" "$base/sink/raw-events/apply" > "$relay_dir/raw-apply-1.json"
  curl -sS -H 'content-type: application/json' -d @"$fixtures_dir/relay-cursor.json" "$base/sink/source-cursors/plan" > "$relay_dir/cursor-plan.json"
  curl -sS -H 'content-type: application/json' -d @"$relay_dir/cursor-plan.json" "$base/sink/source-cursors/apply" > "$relay_dir/cursor-apply.json"
  curl -sS -H 'content-type: application/json' -d @"$fixtures_dir/relay-raw-events.json" "$base/sink/raw-events/plan" > "$relay_dir/raw-plan-2.json"
  curl -sS -H 'content-type: application/json' -d @"$relay_dir/raw-plan-2.json" "$base/sink/raw-events/apply" > "$relay_dir/raw-apply-2.json"

  "$cli_bin" --root "$scenario_root" project plan-rebuild > "$relay_dir/replay-plan.json"
  "$cli_bin" --root "$scenario_root" project apply-replay-plan --file "$relay_dir/replay-plan.json" > "$relay_dir/replay-apply.json"
  "$cli_bin" --root "$scenario_root" project doctor > "$relay_dir/doctor.json"
  sqlite3 -header -json "$scenario_root/context.db" 'select connector, cursor_key, cursor_value, updated_at from source_cursor;' > "$relay_dir/source-cursor.json"

  assert_eq "$(jq '.receipts | length' "$relay_dir/raw-plan-1.json")" "1" "relay first plan should accept one receipt"
  assert_eq "$(jq '.accepted' "$relay_dir/raw-apply-1.json")" "1" "relay first apply should accept one receipt"
  assert_eq "$(jq -r '.updated' "$relay_dir/cursor-apply.json")" "true" "relay cursor apply should update"
  assert_eq "$(jq '.receipts | length' "$relay_dir/raw-plan-2.json")" "0" "relay duplicate plan should produce zero receipts"
  assert_eq "$(jq '.skipped_dedupe_keys | length' "$relay_dir/raw-plan-2.json")" "1" "relay duplicate plan should skip one dedupe key"
  assert_eq "$(jq '.accepted' "$relay_dir/raw-apply-2.json")" "0" "relay duplicate apply should accept zero receipts"
  assert_eq "$(jq '.projection.entries' "$relay_dir/replay-apply.json")" "1" "relay replay should project one entry"
  assert_eq "$(jq '.pending_projection_count' "$relay_dir/doctor.json")" "0" "relay pending projection should be zero"
  assert_eq "$(jq '.pending_derived_count' "$relay_dir/doctor.json")" "0" "relay pending derivation should be zero"
  assert_eq "$(jq 'length' "$relay_dir/source-cursor.json")" "1" "relay source cursor row should exist"

  append_report "## relay"
  append_report "- root: \`$scenario_root\`"
  append_report "- first apply accepted: $(jq '.accepted' "$relay_dir/raw-apply-1.json")"
  append_report "- duplicate skipped keys: $(jq '.skipped_dedupe_keys | length' "$relay_dir/raw-plan-2.json")"
  append_report "- cursor row: \`$(jq -c '.[0]' "$relay_dir/source-cursor.json")\`"
  append_report "- doctor: \`$(jq -c . "$relay_dir/doctor.json")\`"
  append_report
}

main() {
  need_cmd cargo
  need_cmd jq
  need_cmd curl
  need_cmd sqlite3

  mkdir -p "$run_root"
  append_report "# AxiomSync Real User QA"
  append_report
  append_report "- repo: \`$repo_root\`"
  append_report "- run_root: \`$run_root\`"
  append_report "- date: \`$(date -u +"%Y-%m-%dT%H:%M:%SZ")\`"
  append_report

  cargo build -q -p axiomsync-cli
  [[ -x "$cli_bin" ]] || fail "cli binary not found at $cli_bin"

  local -a scenarios
  if [[ "$#" -eq 0 ]]; then
    scenarios=(cli http mcp relay)
  else
    scenarios=("$@")
  fi

  local scenario
  for scenario in "${scenarios[@]}"; do
    case "$scenario" in
      cli) run_cli_scenario ;;
      http) run_http_scenario ;;
      mcp) run_mcp_scenario ;;
      relay) run_relay_scenario ;;
      *) fail "unknown scenario: $scenario" ;;
    esac
  done

  append_report "## verdict"
  append_report "- status: pass"
  append_report "- scenarios: \`${scenarios[*]}\`"

  printf 'QA PASS\n'
  printf 'report: %s\n' "$report_file"
}

main "$@"
