# 08. Consistency Review

결정:
- shipping truth를 문서 truth로 쓴다
- sink는 단일 mutate API가 아니라 `plan -> apply`
- rebuild도 same rule을 따른다
- `claims`는 숨기지 않고 current derived helper로 문서화한다
- `case`, `thread`, `runbook`, `task`, `document`, `evidence`는 compatibility adapter다

구분:
- `axiomsync-b8e8828-audit-patch-package`는 historical audit artifact
- `axiomsync-final-form-docs-package`는 current release artifact
