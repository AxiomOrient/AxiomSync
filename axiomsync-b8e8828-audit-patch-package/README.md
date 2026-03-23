# 패키지 안내

이 패키지는 **실구현 감사 + patch-style 개선안** 세트다.

현재 릴리스 계약은 `../axiomsync-final-form-docs-package`를 기준으로 본다.
이 폴더는 historical audit artifact다.

## 바로 볼 순서

1. `00_EXEC_SUMMARY.md`
2. `02_IMPLEMENTATION_AUDIT.md`
3. `03_GAP_MATRIX_AND_DECISIONS.md`
4. `04_PATCHSET.md`
5. `patches/0001_narrow_public_surface.patch`
6. `patches/0002_explicit_source_cursor_plan.patch`
7. `patches/0003_remove_connector_runtime_leakage.patch`

## 해석 방법

- `patches/` 는 hand-authored unified diff다.
- `proposed/rust/` 는 compile-oriented skeleton이다.
- 이 환경에서는 cargo verification을 하지 못했으므로, **패치 적용 전 로컬 compile/test가 필수**다.
