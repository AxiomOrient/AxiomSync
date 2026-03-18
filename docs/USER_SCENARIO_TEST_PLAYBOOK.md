# 사용자 시나리오 테스트 플레이북 (AxiomSync)

## 1) 현재 상태 요약

- 시나리오 기반 테스트 자산은 이미 일부 존재합니다.
  - `crates/axiomsync/TEST_INTENT.md`: 의도/시나리오 수준의 테스트 목적 정리
  - `docs/APPLICATION_SERVICE_TEST_STRATEGY.md`: 서비스 커버리지/우선순위
  - `crates/axiomsync/tests/process_contract.rs`: CLI/파이프라인 프로세스 시나리오
  - `crates/axiomsync/tests/repository_markdown_user_flows.rs`: 사용자 흐름(리포지토리 마크다운) 기반 통합 시나리오
  - `crates/axiomsync/src/bin/runtime_baseline.rs`: `small/medium/stress` 시나리오 실행기(성능·복구 측정)
- 본 문서는 실사용자가 바로 실행 가능한 프롬프트형 시나리오와 1줄 점검 명령을 함께 제공합니다.

## 2) 실사용 실행 원칙 (웹 베스트 프랙티스 반영)

- Given/When/Then 형식으로 시나리오를 구성해 실행 조건/행위/검증을 분리합니다. (BDD/Gherkin 계열)
  - 핵심 원칙: Given=사전조건, When=작업, Then=검증
  - 출처: [Cucumber Better Gherkin](https://cucumber.io/docs/bdd/better-gherkin/), [Wikipedia: Given-When-Then](https://en.wikipedia.org/wiki/Given-When-Then)
- 우선순위는 리스크 기반으로 정렬합니다.  
  - 고위험 기능(리소스 손실, 데드레터, 세션 데이터 손실, 보안 관련 경로)을 더 먼저 실행합니다.
  - 출처: [Risk-based testing (definition)](https://en.wikipedia.org/wiki/Risk-based_testing)

## 3) 시나리오 카드 템플릿

```text
Scenario:
  id: SC-xxx
  domain: file|user|session|risk|event
  risk: [H|M|L]
  preconditions:
    - [환경/데이터/권한]
  steps:
    - Given ...
    - When ...
    - Then ...
  assertions:
    - [증명 포인트]
  artifacts:
    - [로그/파일/db/명령 출력]
```

## 4) 즉시 실행 가능한 사용자/QA 프롬프트

아래 텍스트를 그대로 사용하면 됩니다.

### 4.1 File-domain (리소스/파일 추가)

```text
당신은 AxiomSync 테스트 담당자야.

목표: resources 파일 추가/재색인/검색 시나리오를 검증해.

Given:
- 깨끗한 임시 root(`/tmp/axiom-e2e-file`)를 사용해 `axiomsync init` 실행
- `fixture.md`, `notes.md`, `README.md`를 테스트용으로 생성

When:
1. `axiomsync add`로 세 파일을 `axiom://resources/e2e-file`에 wait=true로 추가
2. `axiomsync search`로 `fixture`와 `notes` 쿼리를 실행
3. 동일한 `axiomsync search` 쿼리를 재실행해 검색 일관성 확인

Then:
- 검색 결과가 빈 배열이 아니어야 함
- add 직후/재실행 모두 결과가 일관되어야 함
- 이벤트/큐 메트릭이 `replay`에서 비정상 실패 없이 안정적이어야 함
```

### 4.2 User-domain (세션/메시지/커밋)

```text
당신은 운영 테스트 오퍼레이터야.

목표: 세션 생성부터 메시지/커밋/검색 통합 동작 검증.

Given:
- `axiomsync init`된 root에서 `release-operator` 세션 생성
- 사용자 메시지 3개 추가 (`axiomsync session add` 사용)

When:
1. `axiomsync session create --id release-operator`
2. `axiomsync session add --id release-operator --role user --text "..."` (3회)
3. `axiomsync session commit --id release-operator`
4. `axiomsync search "..." --session release-operator`
5. `axiomsync session list` 및 `axiomsync session delete --id release-operator`

Then:
- 커밋 후 `memories_extracted` 또는 메시지 수집 로그가 갱신되어야 함
- 세션 검색 결과가 메시지/커밋 맥락과 연동되어야 함
- 세션 삭제 후 재호출이 false 또는 idempotent해야 함
```

### 4.3 Risk-domain (리스크/폴백/데드레터)

```text
당신은 신뢰성 검증자야.

목표: 잘못된 설정값에서 시스템 폴백/감시 신호가 보존되는지 확인.

Given:
- 테스트 root에서 `AXIOMSYNC_MEMORY_DEDUP_MODE=invalid` 환경 설정
- 기존 메모리/세션 데이터 최소 1개 준비

When:
1. `AXIOMSYNC_MEMORY_DEDUP_MODE=invalid axiomsync session commit --id release-operator` 실행
2. `axiomsync queue status`로 dead_letter 카운트 확인
3. 동일 실행을 정상 값(`auto`/`llm`/`deterministic`)으로 재실행 비교

Then:
- invalid 값은 `auto` 동작으로 회귀되어야 함
- dead-letter 이벤트 타입(`memory_dedup_config`)가 보존되어야 함
- 정상 값과의 결과 편차가 기대 범위 밖으로 확장되지 않아야 함
```

### 4.4 Event-domain (이벤트 수집/아카이브)

```text
당신은 데이터 수명주기 담당자야.

목표: 이벤트 추가/조회/아카이브 계획 실행/실행 검증.

Given:
- root 초기화 후 baseline 이벤트 아티팩트 준비
- `namespace`와 `kind` 필터 값(`acme/platform`, `incident`) 지정

When:
1. `axiomsync event add --event-id ...`로 최소 2개 이벤트 저장
2. `axiomsync search "..." --target axiom://events --namespace acme/platform --kind incident`로 조회
3. 이벤트 타임범위로 `axiomsync event archive plan` 생성 후 `axiomsync event archive execute` 실행

Then:
- plan 실행 ID와 실행 결과가 일관되어야 함
- 아카이브 대상 이벤트가 예상 수만큼 이동되어야 함
- 기본 조회/아카이브 동작이 이벤트 스코프에서 안정적으로 동작해야 함
```

## 5) 운영 체크리스트 (실행 전/후)

- 실행 전
  - DB/루트 분리(항상 임시 root 사용)
  - fixture는 최소 데이터셋으로 시작하고, 반복 실행 시 clean-up 보장
- 실행 중
  - 성공/실패 출력에 JSON인지 로그인지 명시 수집
  - 실패 시 스택/출력 전체 보존
- 실행 후
  - `queue`, `search`, `session`, `event` 계열 기본 커맨드를 재현
  - 실패 시 `runtime_replay`, `dead_letter` 지표 비교

## 6) 권장 실행 순서

1. `runtime_baseline` 빠른 안정성 확인
   - `cargo run -p axiomsync --bin runtime_baseline -- --scenario small`
2. 파일 시나리오 2개 (정상 + 엣지)
3. 유저 시나리오 1개 (`commit`/`search` 연계)
4. 리스크 시나리오 1개 (invalid env + dead-letter)
5. 이벤트 아카이브 시나리오 1개

## 7) 결과 템플릿 (복붙 보고서)

```text
- scenario_id:
- domain:
- pass_fail: PASS/FAIL
- preconditions:
- executed_steps:
- evidence:
  - command_stdout_snippets:
  - dead_letter_snapshot:
- queue_counts:
- blockers:
- follow_up:
```

## 8) 실사용 1줄 실행 (랜덤 테스트 포함) — 핵심만

아래 명령 하나로 실무용 핵심 점검을 실행합니다.

- 랜덤 시나리오 반복 실행: seed + iterations
- 실패 시 핵심만 출력 (PASS/FAIL, 시나리오, 시드, 경과 시간)
- 실패 시 재현 커맨드 함께 출력

```bash
bash scripts/run_quick_scenario_checks.sh \
  --iterations 5 \
  --seed 20260318 \
  --timeout 90 \
  --scenario random \
  --max-cold-ms 1200 \
  --max-p95-ms 700 \
  --min-queue-eps 50 \
  --summary-out /tmp/axiomsync-quick-run-summary.json \
  --summary-format json
```

옵션:
- `--iterations <n>`: 반복 횟수
- `--seed <n>`: 랜덤 결정값(고정하면 재현 가능)
- `--timeout <seconds>`: 1회 실행 타임아웃
- `--scenario <small|medium|stress|random>`: `random`이면 회차별 랜덤 샘플
- `--fail-fast`: 첫 실패 시 즉시 종료
- `--max-cold-ms / --max-warm-ms / --max-p95-ms`: 임계치 초과 시 해당 반복을 FAIL 처리
- `--min-queue-eps`: queue replay 처리량 하한 미달 시 해당 반복을 FAIL 처리 (단위: eps)
- `--summary-out <path>`: 실행 요약 파일 저장(포맷에 맞는 확장자 사용, 예: `/tmp/axiomsync-quick-run-summary.txt` 또는 `.json`)
- `--summary-format <text|json>`: 요약 포맷 지정(기본값: text)

실행 결과 목표 형식:

```text
IDX SCENARIO STATUS SEED ELAPSED METRICS
001 [medium] PASS seed=... 1234ms cold=...ms ...
002 [stress] FAIL seed=... 90000ms              timeout after 90s
      repro: cargo run -p axiomsync --bin runtime_baseline -- --scenario ...
RESULT pass=... fail=... total=... seed=...
RESULT_WARNING ... # counts_match=false일 때만 출력 (stderr로도 함께 출력)
```

이 결과는 "핵심만" 보기 좋고, 이후 전체 JSON 로그가 필요하면 `--summary-out`에 저장된 집계 JSON이 아니라 각 실행별 `run-###/report.json` 파일에서 상세 로그를 확인하면 됩니다.

`--summary-out` 사용 시 요약 파일은 `fail_reason_code`를 함께 출력해 집계 자동화가 쉽습니다.

```text
failure_reasons:
  timeout: 0
  command_error: 0
  cold_boot_exceeded: 0
  warm_boot_exceeded: 0
  p95_exceeded: 0
  queue_eps_below_min: 0
  unknown: 0
```

`--summary-format json` 사용 시 동일한 요약을 JSON으로 저장해 파이프라인에서 바로 파싱할 수 있습니다.
`counts_match`는 `pass+fail == total` 및 `run_count == total` 확인 플래그입니다. 실패 시 자동 감지 포인트로 사용할 수 있습니다.

```json
{
  "schema_version": "1.0.0",
  "seed": "20260318",
  "iterations": 5,
  "result": {
    "pass": 4,
    "fail": 1,
    "total": 5,
    "counts_match": true,
    "failure_reasons": {
      "timeout": 0,
      "command_error": 1,
      "cold_boot_exceeded": 0,
      "warm_boot_exceeded": 0,
      "p95_exceeded": 0,
      "queue_eps_below_min": 0,
      "unknown": 0,
      "total": 1
    },
    "run_count": 5
  }
}
```
