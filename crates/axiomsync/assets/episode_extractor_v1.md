Return one JSON object with this exact shape:

{
  "problem": "short problem statement",
  "root_cause": "optional root cause or null",
  "fix": "optional fix or null",
  "commands": ["command strings only"],
  "decisions": ["important decisions only"],
  "snippets": ["short code or diff snippets only"]
}

Rules:
- Respond with JSON only.
- Use concise plain text.
- Do not invent fields.
- If a field is unknown, use null or an empty array.
- `problem` is required and must summarize the main issue in one sentence.
