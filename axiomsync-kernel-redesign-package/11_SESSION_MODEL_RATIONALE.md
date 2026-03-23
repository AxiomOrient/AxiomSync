# Session Model Rationale

## Why not pure `conv_*`
`conv_*` is attractive when the only upstream is conversation capture.

But you explicitly want AxiomSync reusable for:
- AxiomRelay
- axiomRams

That means the kernel must handle both:
- conversations
- runs and their artifacts

## Why not pure `run_*`
That would overfit axiomRams and make AxiomRelay awkward.

## Chosen center
Use:
- `session`
- `entry`
- `artifact`
- `anchor`

Then preserve strong conversation support with:
- `session_kind = conversation`
- `entry_kind = message | selection | tool_call | tool_result`

And support axiomRams naturally with:
- `session_kind = run`
- `entry_kind = run_event | decision_note | artifact_ref`

This is the smallest model that fits both products cleanly.
