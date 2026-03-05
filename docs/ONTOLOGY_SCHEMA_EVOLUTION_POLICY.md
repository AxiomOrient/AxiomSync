# Ontology Schema Evolution Policy

## 1. Objective
온톨로지 스키마 진화는 명시적 버전과 결정적 검증을 기본으로 합니다.

## 2. Current Baseline
- Canonical URI: `axiom://agent/ontology/schema.v1.json`
- Current schema version: `1`
- Parser mode: strict (`deny_unknown_fields`)

## 3. Evolution Rule
현재 단계에서는 단일 활성 major만 유지합니다.
- `v1 -> v2` 전환은 한 변경셋에서 명시적으로 cutover
- hidden fallback/dual path는 두지 않음

## 4. Release Gate Requirement
Contract integrity gate(`G0`)는 다음을 만족해야 합니다.
- ontology probe test pass
- schema parse/compile pass
- required schema version match
- invariant failure count = 0

## 5. Operational Rule
- 스키마 변경 시 `API_CONTRACT.md`와 함께 동기화
- 변경 근거는 테스트/게이트 결과로 남김
