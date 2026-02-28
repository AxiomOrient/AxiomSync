# Release Notes

## Summary
- Runtime 경계(`core/cli/mobile-ffi`)를 유지한 상태로 품질 게이트와 strict 릴리즈 게이트를 통과했습니다.
- 문서 체계를 canonical/archived로 분리해 운영 산출물 노이즈를 낮췄습니다.
- 대형 모듈 분해 작업을 단계적으로 진행 중입니다.

## Compatibility
- Canonical URI: `axiom://`
- 검색 응답 계약은 `query_results` 중심으로 유지
- 하위 호환 필드는 `API_CONTRACT.md` 정책에 따름

## Verification Snapshot
- `bash scripts/quality_gates.sh`
- `bash scripts/release_pack_strict_gate.sh --workspace-dir . --output logs/release_pack_strict_report.json`
- `scripts/release_signoff_status.sh --report-path docs/RELEASE_SIGNOFF_STATUS.md`

## Notes
- 날짜/버전 표기 중심의 과거 운영 문서는 `docs/archive/`로 이동했습니다.
