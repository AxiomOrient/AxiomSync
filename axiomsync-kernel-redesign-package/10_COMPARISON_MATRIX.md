# Comparison Matrix

| Dimension | AxiomSync (current public) | AxiomSync (target) | AxiomRelay | axiomRams |
|---|---|---|---|---|
| Main role | kernel + edge mix | pure knowledge kernel | capture / forward service | contract-first runtime |
| Source of truth | SQLite db | SQLite knowledge db | spool / service state | file-based run state |
| Ingest | connector ops + HTTP | narrow raw-event sink | capture and forward | export selected evidence |
| Query | CLI / HTTP / MCP | CLI / HTTP / MCP | service UX only | CLI / Tauri for run control |
| Owns approvals | mixed / external ambiguity | no | yes | yes |
| Owns retry / spool | mixed / external ambiguity | no | yes | no |
| Generic across products | limited | high | no | no |
| Best integration with kernel | n/a | yes | append raw events | append selected run evidence |
