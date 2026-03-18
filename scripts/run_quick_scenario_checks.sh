#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "$REPO_ROOT"

usage() {
  cat <<'EOF_USAGE'
Usage: scripts/run_quick_scenario_checks.sh [options]

실무용 간단 시나리오 점검을 실행합니다.

Options:
  --iterations <n>      반복 횟수 (기본값: 5)
  --seed <n>            랜덤 시드 (기본값: 현재 epoch)
  --timeout <seconds>   각 실행 타임아웃 (기본값: 90)
  --scenario <name>     고정 시나리오: small | medium | stress | random (기본값: random)
  --root <path>         공용 root 경로 (기본값: 임시 디렉터리)
  --max-cold-ms <ms>    cold boot 임계치 (기본값: 제한 없음)
  --max-warm-ms <ms>    warm boot 임계치 (기본값: 제한 없음)
  --max-p95-ms <ms>     steady p95 임계치 (기본값: 제한 없음)
  --min-queue-eps <eps> queue replay 처리량 하한 임계치 (기본값: 제한 없음)
  --summary-out <path>   실행 요약을 파일로 저장 (text/json)
  --summary-format <text|json>  요약 출력 포맷 (기본값: text)
  --fail-fast           첫 실패 시 즉시 중단
  --help                도움말

예시:
  scripts/run_quick_scenario_checks.sh
  scripts/run_quick_scenario_checks.sh --iterations 10 --seed 20260318
  scripts/run_quick_scenario_checks.sh --scenario small --iterations 3 --max-p95-ms 250
EOF_USAGE
}

iterations=5
seed="$(date +%s)"
timeout_secs=90
scenario_mode="random"
root_base=""
fail_fast=false
max_cold_ms=""
max_warm_ms=""
max_p95_ms=""
min_queue_eps=""
summary_out=""
summary_format="text"
summary_tmp=""
timeout_cmd=""

is_positive_int() {
  [[ "$1" =~ ^[1-9][0-9]*$ ]]
}

is_non_negative_int() {
  [[ "$1" =~ ^[0-9]+$ ]]
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --iterations)
      iterations="${2:-}"
      shift 2
      ;;
    --seed)
      seed="${2:-}"
      shift 2
      ;;
    --timeout)
      timeout_secs="${2:-}"
      shift 2
      ;;
    --scenario)
      scenario_mode="${2:-}"
      shift 2
      ;;
    --root)
      root_base="${2:-}"
      shift 2
      ;;
    --max-cold-ms)
      max_cold_ms="${2:-}"
      shift 2
      ;;
    --max-warm-ms)
      max_warm_ms="${2:-}"
      shift 2
      ;;
    --max-p95-ms)
      max_p95_ms="${2:-}"
      shift 2
      ;;
    --min-queue-eps)
      min_queue_eps="${2:-}"
      shift 2
      ;;
    --summary-out)
      summary_out="${2:-}"
      shift 2
      ;;
    --summary-format)
      summary_format="${2:-}"
      shift 2
      ;;
    --fail-fast)
      fail_fast=true
      shift
      ;;
    --help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if ! is_positive_int "$iterations"; then
  echo "--iterations must be positive integer" >&2
  exit 1
fi

if ! is_non_negative_int "$seed"; then
  echo "--seed must be numeric" >&2
  exit 1
fi

if ! is_positive_int "$timeout_secs"; then
  echo "--timeout must be positive integer" >&2
  exit 1
fi

if [[ -n "$max_cold_ms" && ! is_non_negative_int "$max_cold_ms" ]]; then
  echo "--max-cold-ms must be non-negative integer" >&2
  exit 1
fi

if [[ -n "$max_warm_ms" && ! is_non_negative_int "$max_warm_ms" ]]; then
  echo "--max-warm-ms must be non-negative integer" >&2
  exit 1
fi

if [[ -n "$max_p95_ms" && ! is_non_negative_int "$max_p95_ms" ]]; then
  echo "--max-p95-ms must be non-negative integer" >&2
  exit 1
fi
if [[ -n "$min_queue_eps" && ! is_non_negative_int "$min_queue_eps" ]]; then
  echo "--min-queue-eps must be non-negative integer" >&2
  exit 1
fi
if [[ "$summary_format" != "text" && "$summary_format" != "json" ]]; then
  echo "--summary-format must be text or json" >&2
  exit 1
fi

if command -v timeout >/dev/null 2>&1; then
  timeout_cmd=timeout
elif command -v gtimeout >/dev/null 2>&1; then
  timeout_cmd=gtimeout
else
  echo "timeout command is required (install coreutils for timeout/gtimeout)" >&2
  exit 1
fi

SCENARIOS=(small medium stress)

case "$scenario_mode" in
  small|medium|stress)
    fixed_scenario="$scenario_mode"
    ;;
  random)
    fixed_scenario=""
    ;;
  *)
    echo "--scenario must be small|medium|stress|random" >&2
    exit 1
    ;;
esac

if [[ -z "$root_base" ]]; then
  root_base="$(mktemp -d /tmp/axiomsync-quick-scenarios-XXXXXX)"
  cleanup_root=true
else
  mkdir -p "$root_base"
  cleanup_root=false
fi

cleanup() {
  if [[ "$cleanup_root" == true && -n "${root_base:-}" && -d "$root_base" ]]; then
    rm -rf "$root_base"
  fi
  if [[ -n "${summary_tmp:-}" && -f "$summary_tmp" ]]; then
    rm -f "$summary_tmp"
  fi
}
trap cleanup EXIT

init_summary() {
  if [[ -z "$summary_out" ]]; then
    return
  fi

  summary_tmp="$(mktemp)"
  if [[ "$summary_format" == "text" ]]; then
    {
      echo "# Quick Scenario Summary"
      echo "seed: $seed"
      echo "iterations: $iterations"
      echo "scenario_mode: $scenario_mode"
      echo "timeout_seconds: $timeout_secs"
      echo "thresholds:"
      if [[ -n "$max_cold_ms" ]]; then
        echo "  max_cold_ms: $max_cold_ms"
      fi
      if [[ -n "$max_warm_ms" ]]; then
        echo "  max_warm_ms: $max_warm_ms"
      fi
      if [[ -n "$max_p95_ms" ]]; then
        echo "  max_p95_ms: $max_p95_ms"
      fi
      if [[ -n "$min_queue_eps" ]]; then
        echo "  min_queue_eps: $min_queue_eps"
      fi
      echo "runs:"
    } >"$summary_tmp"
  else
    json_runs=()
  fi
}

json_escape() {
  local value="$1"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  value="${value//$'\n'/\\n}"
  value="${value//$'\r'/\\r}"
  value="${value//$'\t'/\\t}"
  printf '%s' "$value"
}

json_maybe_number() {
  local value="$1"
  if [[ "$value" == "N/A" ]]; then
    echo "null"
    return
  fi
  if [[ "$value" =~ ^[0-9]+([.][0-9]+)?$ ]]; then
    echo "$value"
    return
  fi
  echo "null"
}

json_maybe_ms_to_number() {
  local value="$1"
  local numeric="${value%ms}"
  if [[ "$value" == "$numeric" ]]; then
    json_maybe_number "$value"
    return
  fi
  json_maybe_number "$numeric"
}

jsonize_value() {
  local value="$1"
  if [[ -z "$value" ]]; then
    echo "null"
  else
    printf '"%s"' "$(json_escape "$value")"
  fi
}

build_summary_json() {
  {
    echo "{"
    echo "  \"schema_version\": \"1.0.0\","
    echo "  \"generated_at\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
    echo "  \"seed\": \"$(json_escape "$seed")\","
    echo "  \"iterations\": $iterations,"
    echo "  \"scenario_mode\": \"$(json_escape "$scenario_mode")\","
    echo "  \"timeout_seconds\": $timeout_secs,"
    echo "  \"thresholds\": {"
    echo "    \"max_cold_ms\": ${max_cold_ms:-null},"
    echo "    \"max_warm_ms\": ${max_warm_ms:-null},"
    echo "    \"max_p95_ms\": ${max_p95_ms:-null},"
    echo "    \"min_queue_eps\": ${min_queue_eps:-null}"
    echo "  },"
    echo "  \"runs\": ["
    local i=0
    for run in "${json_runs[@]}"; do
      if [[ "$i" -gt 0 ]]; then
        echo ","
      fi
      echo "    $run"
      i=$((i + 1))
    done
    echo "  ],"
    echo "  \"result\": {"
    echo "    \"pass\": $pass_count,"
    echo "    \"fail\": $fail_count,"
    echo "    \"total\": $total_count,"
    echo "    \"counts_match\": $counts_match,"
    echo "    \"seed\": \"$(json_escape "$seed")\","
    echo "    \"failure_reasons\": {"
    echo "      \"timeout\": $timeout_count,"
    echo "      \"command_error\": $command_error_count,"
    echo "      \"cold_boot_exceeded\": $cold_boot_exceeded_count,"
    echo "      \"warm_boot_exceeded\": $warm_boot_exceeded_count,"
    echo "      \"p95_exceeded\": $p95_exceeded_count,"
    echo "      \"queue_eps_below_min\": $queue_eps_below_min_count,"
    echo "      \"unknown\": $unknown_fail_reason_count,"
    echo "      \"total\": $failure_reason_sum"
    echo "    },"
    echo "    \"run_count\": $run_count"
    echo "  }"
    echo "}"
  } >"$summary_tmp"
}

append_summary() {
  local idx="$1"
  local scenario="$2"
  local status="$3"
  local run_seed="$4"
  local elapsed_ms="$5"
  local cold_ms="$6"
  local warm_ms="$7"
  local p50_ms="$8"
  local p95_ms="$9"
  local queue_eps="${10}"
  local fail_reason="${11:-}"
  local fail_reason_code="${12:-pass}"
  if [[ "$summary_format" == "text" ]]; then
    {
      echo "- index: $idx"
      echo "  scenario: $scenario"
      echo "  status: $status"
      echo "  seed: $run_seed"
      echo "  elapsed_ms: $elapsed_ms"
      echo "  metrics:"
      echo "    cold_boot_ms: $cold_ms"
      echo "    warm_boot_ms: $warm_ms"
      echo "    steady_search_p50_ms: $p50_ms"
      echo "    steady_search_p95_ms: $p95_ms"
      echo "    queue_replay_events_per_sec: $queue_eps"
      echo "  fail_reason_code: $fail_reason_code"
      if [[ -n "$fail_reason" ]]; then
        echo "  fail_reason: ${fail_reason:0:220}"
      fi
    } >>"$summary_tmp"
  else
    local fail_reason_json
    fail_reason_json="$(jsonize_value "${fail_reason:0:220}")"
    json_runs+=("{\"index\":$idx,\"scenario\":\"$(json_escape "$scenario")\",\"status\":\"$(json_escape "$status")\",\"seed\":\"$(json_escape "$run_seed")\",\"elapsed_ms\":$(json_maybe_ms_to_number "$elapsed_ms"),\"metrics\":{\"cold_boot_ms\":$(json_maybe_number "$cold_ms"),\"warm_boot_ms\":$(json_maybe_number "$warm_ms"),\"steady_search_p50_ms\":$(json_maybe_number "$p50_ms"),\"steady_search_p95_ms\":$(json_maybe_number "$p95_ms"),\"queue_replay_events_per_sec\":$(json_maybe_number "$queue_eps")},\"fail_reason_code\":\"$(json_escape "$fail_reason_code")\",\"fail_reason\":$fail_reason_json}")
  fi
}

pick_random_scenario() {
  # xorshift-like deterministic LCG for reproducible pseudo-random sampling.
  seed=$(( (seed * 1103515245 + 12345) & 2147483647 ))
  idx=$(( seed % ${#SCENARIOS[@]} ))
  printf '%s' "${SCENARIOS[$idx]}"
}

run_one() {
  local i="$1"
  local scenario="$2"
  local run_seed="$3"
  local run_root="${root_base}/run-$(printf '%03d' "$i")"
  local json_out="${run_root}/report.json"
  local cmd=(
    cargo
    run
    --quiet
    -p
    axiomsync
    --bin
    runtime_baseline
    --
    --scenario
    "$scenario"
    --root
    "$run_root"
    --json-out
    "$json_out"
  )

  local start_ms
  local elapsed_ms
  local status="PASS"
  local fail_reason=""
  local fail_reason_code="pass"
  local metrics=""
  local cold_ms="N/A"
  local warm_ms="N/A"
  local p50_ms="N/A"
  local p95_ms="N/A"
  local queue_eps="N/A"

  mkdir -p "$run_root"
  start_ms=$(date +%s%3N)

  if output="$("$timeout_cmd" "${timeout_secs}s" "${cmd[@]}" 2>&1)"; then
    if [[ -f "$json_out" ]] && command -v jq >/dev/null 2>&1; then
      read -r cold_ms warm_ms p50_ms p95_ms queue_eps <<<"$(jq -r '.reports[0] | \"\\(.cold_boot_ms) \\(.warm_boot_ms) \\(.steady_search_p50_ms) \\(.steady_search_p95_ms) \\(.queue_replay_events_per_sec)\"' \"$json_out\" 2>/dev/null || echo 'N/A N/A N/A N/A N/A')"
      metrics=$(jq -r '.reports[0] | "cold=\(.cold_boot_ms)ms warm=\(.warm_boot_ms)ms p50=\(.steady_search_p50_ms)ms p95=\(.steady_search_p95_ms)ms"' "$json_out" 2>/dev/null || true)
    fi
  else
    status="FAIL"
    rc=$?
    fail_reason_code="command_error"
    if [[ $rc -eq 124 ]]; then
      fail_reason_code="timeout"
      fail_reason="timeout after ${timeout_secs}s"
    else
      fail_reason="command failed (exit $rc): $(printf '%s' "$output" | tr '\n' ' ' | cut -c1-220)"
    fi
  fi
  elapsed_ms=$(( $(date +%s%3N) - start_ms ))

  if [[ "$status" == PASS && -n "$max_cold_ms" && "$cold_ms" != N/A && "$cold_ms" =~ ^[0-9]+$ && "$cold_ms" -gt "$max_cold_ms" ]]; then
    status="FAIL"
    fail_reason_code="cold_boot_exceeded"
    fail_reason="cold_boot_ms ${cold_ms} > ${max_cold_ms}"
  fi
  if [[ "$status" == PASS && -n "$max_warm_ms" && "$warm_ms" != N/A && "$warm_ms" =~ ^[0-9]+$ && "$warm_ms" -gt "$max_warm_ms" ]]; then
    status="FAIL"
    fail_reason_code="warm_boot_exceeded"
    fail_reason="warm_boot_ms ${warm_ms} > ${max_warm_ms}"
  fi
  if [[ "$status" == PASS && -n "$max_p95_ms" && "$p95_ms" != N/A && "$p95_ms" =~ ^[0-9]+$ && "$p95_ms" -gt "$max_p95_ms" ]]; then
    status="FAIL"
    fail_reason_code="p95_exceeded"
    fail_reason="steady_search_p95_ms ${p95_ms} > ${max_p95_ms}"
  fi
  if [[ "$status" == PASS && -n "$min_queue_eps" && "$queue_eps" != N/A ]]; then
    if awk -v q="$queue_eps" -v m="$min_queue_eps" \
      'BEGIN { exit (q + 0 < m + 0 ? 0 : 1) }'; then
      status="FAIL"
      fail_reason_code="queue_eps_below_min"
      fail_reason="queue_replay_events_per_sec ${queue_eps} < ${min_queue_eps}"
    fi
  fi

  if [[ "$status" == PASS ]]; then
    printf '%3s | %-8s | %-6s | %-8s | %-9s | %s\n' \
      "$i" "[$scenario]" "$status" "seed=$run_seed" "${elapsed_ms}ms" "$metrics"
    if [[ -n "$summary_tmp" ]]; then
      append_summary "$i" "$scenario" "$status" "$run_seed" "${elapsed_ms}ms" "$cold_ms" "$warm_ms" "$p50_ms" "$p95_ms" "$queue_eps" "" "$fail_reason_code"
    fi
  else
    case "$fail_reason_code" in
      timeout)
        timeout_count=$((timeout_count + 1))
        ;;
      command_error)
        command_error_count=$((command_error_count + 1))
        ;;
      cold_boot_exceeded)
        cold_boot_exceeded_count=$((cold_boot_exceeded_count + 1))
        ;;
      warm_boot_exceeded)
        warm_boot_exceeded_count=$((warm_boot_exceeded_count + 1))
        ;;
      p95_exceeded)
        p95_exceeded_count=$((p95_exceeded_count + 1))
        ;;
      queue_eps_below_min)
        queue_eps_below_min_count=$((queue_eps_below_min_count + 1))
        ;;
      *)
        unknown_fail_reason_count=$((unknown_fail_reason_count + 1))
        ;;
    esac

    printf '%3s | %-8s | %-6s | %-8s | %-9s | %s\n' \
      "$i" "[$scenario]" "$status" "seed=$run_seed" "${elapsed_ms}ms" "$fail_reason"
    echo "      repro: cargo run -p axiomsync --bin runtime_baseline -- --scenario $scenario --root $run_root --json-out $json_out"
    if [[ -n "$summary_tmp" ]]; then
      append_summary "$i" "$scenario" "$status" "$run_seed" "${elapsed_ms}ms" "$cold_ms" "$warm_ms" "$p50_ms" "$p95_ms" "$queue_eps" "$fail_reason" "$fail_reason_code"
    fi
  fi

  if [[ "$status" == FAIL ]]; then
    return 1
  fi

  return 0
}

init_summary
echo "seed=$seed, iterations=$iterations, scenario=$scenario_mode, timeout=${timeout_secs}s"
if [[ -n "$max_cold_ms" || -n "$max_warm_ms" || -n "$max_p95_ms" || -n "$min_queue_eps" ]]; then
  echo "thresholds: ${max_cold_ms:+cold<${max_cold_ms} }${max_warm_ms:+warm<${max_warm_ms} }${max_p95_ms:+p95<${max_p95_ms} }${min_queue_eps:+queue_eps>=${min_queue_eps}}"
fi
printf '%3s | %-8s | %-6s | %-18s | %-9s | %s\n' \
  "IDX" "SCENARIO" "STATUS" "SEED" "ELAPSED" "METRICS/FN"

pass_count=0
fail_count=0
timeout_count=0
command_error_count=0
cold_boot_exceeded_count=0
warm_boot_exceeded_count=0
p95_exceeded_count=0
queue_eps_below_min_count=0
unknown_fail_reason_count=0

for i in $(seq 1 "$iterations"); do
  if [[ -n "$fixed_scenario" ]]; then
    current_scenario="$fixed_scenario"
  else
    current_scenario="$(pick_random_scenario)"
  fi

  if run_one "$i" "$current_scenario" "$seed"; then
    pass_count=$((pass_count + 1))
  else
    fail_count=$((fail_count + 1))
    if [[ "$fail_fast" == true ]]; then
      echo "fail-fast triggered"
      break
    fi
  fi
done

total_count=$((pass_count + fail_count))
failure_reason_sum=$((timeout_count + command_error_count + cold_boot_exceeded_count + warm_boot_exceeded_count + p95_exceeded_count + queue_eps_below_min_count + unknown_fail_reason_count))
run_count="$total_count"
if [[ "$summary_format" == "json" ]]; then
  run_count="${#json_runs[@]}"
fi
counts_match=false
if [[ "$failure_reason_sum" -eq "$fail_count" && "$run_count" -eq "$total_count" ]]; then
  counts_match=true
fi
echo "RESULT pass=$pass_count fail=$fail_count total=$total_count seed=$seed"
if [[ "$counts_match" == false ]]; then
  result_warning="RESULT_WARNING counts_match=false (fail_reason_sum=$failure_reason_sum, run_count=$run_count, total=$total_count)"
  echo "$result_warning"
  echo "$result_warning" >&2
fi

if [[ -n "$summary_tmp" ]]; then
  if [[ "$summary_format" == "json" ]]; then
    build_summary_json
  else
    {
      echo "result:"
      echo "  pass: $pass_count"
      echo "  fail: $fail_count"
      echo "  total: $total_count"
      echo "  seed: $seed"
      echo "  failure_reasons:"
      echo "    timeout: $timeout_count"
      echo "    command_error: $command_error_count"
      echo "    cold_boot_exceeded: $cold_boot_exceeded_count"
      echo "    warm_boot_exceeded: $warm_boot_exceeded_count"
      echo "    p95_exceeded: $p95_exceeded_count"
      echo "    queue_eps_below_min: $queue_eps_below_min_count"
      echo "    unknown: $unknown_fail_reason_count"
    } >>"$summary_tmp"
  fi
  mkdir -p "$(dirname "$summary_out")"
  mv "$summary_tmp" "$summary_out"
fi

if [[ "$fail_count" -ne 0 ]]; then
  exit 1
fi
