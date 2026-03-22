Return a JSON array of verification objects.

Each object must use this exact shape:

{
  "kind": "test | command_exit | diff_applied | human_confirm",
  "status": "pass | fail | partial | unknown",
  "summary": "optional short summary or null",
  "evidence": "optional evidence text or null",
  "pass_condition": "optional pass condition or null",
  "exit_code": 0,
  "human_confirmed": false
}

Rules:
- Respond with JSON only.
- Return an empty array when no verification is present.
- Only use the listed enum values.
- Use `null` for missing optional fields.
- Use `human_confirmed=true` only when the transcript explicitly contains human confirmation.
