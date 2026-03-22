# ChatGPT Capture Extension

This extension captures **selected text only** from ChatGPT message blocks and forwards it to a local AxiomSync ingest server.

## Purpose
- keep ChatGPT web capture explicit and user-driven
- send only chosen excerpts, not whole conversations
- attach minimal message-bound provenance to each captured selection
- keep browser capture as an optional companion asset, separate from the core CLI/MCP release

## Install
1. Run the local ingest daemon:

```bash
cargo run -p axiomsync -- connector serve chatgpt --addr 127.0.0.1:4402
```

2. Load `extensions/chatgpt` as an unpacked Chrome extension.
3. Open the popup and confirm the endpoint.

## How It Works
- select text inside one ChatGPT message block
- click the floating `Send to Axiom` action
- the extension sends a `selection_captured` event to the local endpoint
- if delivery fails, the packet is queued in local extension storage and retried in the background

## Packet Provenance
Each capture includes:
- message role
- message id or deterministic fallback id
- selection text
- start/end hint
- DOM fingerprint
- page URL and title
- capture timestamp

## Popup Settings
- endpoint
- default tags
- pending queue count
- last success
- last error

## Default Endpoint
- `http://127.0.0.1:4402/`

## Notes
- host permission is intentionally limited to local HTTP/HTTPS loopback endpoints
- if ChatGPT DOM markup changes, the message selector may need to be updated
- this extension is a local companion asset, not a hosted service integration
