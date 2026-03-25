# Scenario 04: Relay Loopback Sink

## Goal
Validate the write-only sink in a relay-shaped delivery path:
- raw event plan/apply over HTTP
- source cursor upsert over HTTP
- duplicate raw delivery remains idempotent
- replay after sink delivery produces stable rows

## Environment
- one fresh root
- one loopback HTTP server
- no read auth required for sink routes

## Fixtures
- [`../fixtures/relay-raw-events.json`](../fixtures/relay-raw-events.json)
- [`../fixtures/relay-cursor.json`](../fixtures/relay-cursor.json)

## Steps
1. Initialize a fresh root.
2. Start `serve`.
3. POST raw events to `/sink/raw-events/plan`.
4. POST the returned plan to `/sink/raw-events/apply`.
5. POST source cursor request to `/sink/source-cursors/plan`.
6. POST the returned cursor plan to `/sink/source-cursors/apply`.
7. Repeat the same raw events request.
8. Rebuild projection and derivation.
9. Run `project doctor`.

## Expected
- first raw delivery is accepted
- duplicate raw delivery is skipped by dedupe key
- source cursor row is written
- replay completes with no pending work

## Automated Entry
- `qa/bin/run-real-user-qa.sh relay`
