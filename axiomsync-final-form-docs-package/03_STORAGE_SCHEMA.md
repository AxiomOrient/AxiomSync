# 03. Storage Schema

이 패키지는 제안용 이름이 아니라 현재 shipping schema 이름을 기준으로 쓴다.

Raw truth:
- `ingress_receipts`
- `source_cursor`

Canonical projection:
- `sessions`
- `actors`
- `entries`
- `artifacts`
- `anchors`

Derived memory:
- `episodes`
- `insights`
- `insight_anchors`
- `verifications`
- `claims`
- `claim_evidence`
- `procedures`
- `procedure_evidence`
- `search_docs`

핵심 판단:
- 지금 중요한 것은 rename이 아니라 replayability와 evidence linkage다
- `ingress_receipts`와 `source_cursor`는 이미 raw truth 역할을 수행한다
