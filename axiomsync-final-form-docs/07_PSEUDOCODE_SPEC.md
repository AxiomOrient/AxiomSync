# 07. 의사코드 스펙

## 1) append_raw_events

```text
fn append_raw_events(batch):
    validate(batch)
    accepted = []
    rejected = []

    begin tx

    for event in batch.events:
        event_id = event.raw_event_id or make_raw_event_id(event)
        dedupe_key = normalize_dedupe_key(event)

        if exists raw_events where connector_name = batch.source.connector_name
           and dedupe_key = dedupe_key:
            rejected.push({dedupe_key, reason: "duplicate"})
            continue

        normalized = normalize_envelope(batch.source, event)
        content_hash = event.content_hash or sha256(json(normalized.payload))

        insert raw_events(
            raw_event_id = event_id,
            source_kind = batch.source.source_kind,
            connector_name = batch.source.connector_name,
            native_session_id = event.native_session_id,
            native_entry_id = event.native_entry_id,
            event_type = event.event_type,
            captured_at_ms = event.captured_at_ms,
            observed_at_ms = event.observed_at_ms,
            content_hash = content_hash,
            dedupe_key = dedupe_key,
            raw_payload_json = event.payload,
            normalized_json = normalized,
            projection_state = "pending"
        )

        accepted.push(event_id)

    commit tx

    return {accepted, rejected}
```

## 2) project_pending_raw_events

```text
fn project_pending_raw_events(limit):
    rows = select raw_events where projection_state = "pending" order by observed_at_ms limit limit

    for raw in rows:
        begin tx

        session = upsert_session_from_raw(raw)
        actor = upsert_actor_from_raw(raw.normalized_json)
        entry = upsert_entry_from_raw(raw, session, actor)

        for artifact_input in extract_artifacts(raw.normalized_json):
            artifact = upsert_artifact(session, entry, artifact_input)

        for anchor_input in extract_anchors(raw.normalized_json, entry, artifacts):
            upsert_anchor(session, entry, artifact_or_null(anchor_input), anchor_input)

        mark raw_events.projection_state = "projected"

        commit tx
```

## 3) derive_memory_for_session

```text
fn derive_memory_for_session(session_id):
    entries = load_entries(session_id)
    anchors = load_anchors(session_id)

    candidate_groups = segment_into_episode_candidates(entries, anchors)

    for group in candidate_groups:
        episode = upsert_episode(
            kind = classify_episode_kind(group),
            title = summarize_group_title(group),
            summary = summarize_group_body(group),
            status = infer_episode_status(group)
        )
        link_episode_anchors(episode, group.anchor_ids)

        insight_candidates = extract_insights(group)
        for ic in insight_candidates:
            if ic.anchor_ids is empty:
                continue

            insight = upsert_insight(
                kind = classify_insight_kind(ic),
                statement = normalize_statement(ic.statement),
                scope = infer_scope(ic),
                confidence = score_confidence(ic)
            )
            link_insight_anchors(insight, ic.anchor_ids)

            verification = derive_verification(insight, group)
            insert_verification_if_changed(verification)

        procedure_candidate = maybe_extract_procedure(group)
        if procedure_candidate exists and procedure_candidate.anchor_ids not empty:
            procedure = upsert_procedure(
                title = procedure_candidate.title,
                purpose = procedure_candidate.purpose,
                preconditions = procedure_candidate.preconditions,
                expected_outcome = procedure_candidate.expected_outcome
            )
            replace_procedure_steps(procedure, procedure_candidate.steps)
            link_procedure_anchors(procedure, procedure_candidate.anchor_ids)
```

## 4) derive_verification

```text
fn derive_verification(insight, episode_group):
    checks = collect_check_like_entries(episode_group)
    conflicts = find_conflicting_insights(insight.scope, insight.statement)

    if conflicts not empty:
        return verification(status="conflicted", method="heuristic")

    if any deterministic check passed in checks:
        return verification(status="verified", method="deterministic")

    if human confirmation exists in episode_group:
        return verification(status="verified", method="human")

    return verification(status="proposed", method="heuristic")
```

## 5) rebuild_index

```text
fn rebuild_index(scope):
    delete search_docs where matches(scope)

    for entry in load_entries(scope):
        upsert_search_doc(doc_kind="entry", body=entry.text_body, refs=[entry.entry_id])

    for episode in load_episodes(scope):
        upsert_search_doc(doc_kind="episode", body=episode.summary_text, refs=[episode.episode_id])

    for insight in load_insights(scope):
        upsert_search_doc(doc_kind="insight", body=insight.statement, refs=[insight.insight_id])

    for procedure in load_procedures(scope):
        upsert_search_doc(doc_kind="procedure", body=render_procedure_text(procedure), refs=[procedure.procedure_id])

    rebuild_fts()
```

## 6) find_fix

```text
fn find_fix(query, filters):
    hits = search_insights(query, filters + kind="fix")
    verified_hits = prefer_verified(hits)

    if verified_hits not empty:
        return attach_evidence_bundle(best_ranked(verified_hits))

    episode_hits = search_episodes(query, filters + kind="fix")
    if episode_hits not empty:
        return attach_evidence_bundle(best_ranked(episode_hits))

    procedure_hits = search_procedures(query, filters)
    return attach_evidence_bundle(best_ranked(procedure_hits))
```

## 7) Rams export helper

```text
fn export_rams_event(run_state, event):
    if not is_evidence_worthy(event):
        return None

    return raw_event(
        source_kind = "axiomrams",
        connector_name = "rams_runtime",
        native_session_id = "rams:run:" + run_state.run_id,
        native_entry_id = event.event_id,
        event_type = event.type,
        payload = {
            run_id: run_state.run_id,
            task_id: event.task_id,
            step_id: event.step_id,
            summary: event.summary,
            checks: event.checks,
            artifacts: event.artifacts
        },
        hints = {
            session_kind: "run",
            entry_kind: map_rams_event_to_entry_kind(event.type),
            workspace_root: run_state.workspace_root
        }
    )
```

## 8) Relay export helper

```text
fn export_chatgpt_selection(selection):
    return raw_event(
        source_kind = "axiomrelay",
        connector_name = "chatgpt_web_selection",
        native_session_id = "chatgpt:" + selection.conversation_id,
        native_entry_id = selection.message_id,
        event_type = "selection_captured",
        payload = {
            page_url: selection.page_url,
            page_title: selection.page_title,
            source_message: {
                message_id: selection.message_id,
                role: selection.role
            },
            selection: {
                text: selection.text,
                start_hint: selection.start_hint,
                end_hint: selection.end_hint,
                dom_fingerprint: selection.dom_fingerprint
            },
            user_note: selection.user_note,
            tags: selection.tags
        },
        hints = {
            session_kind: "conversation",
            entry_kind: "message"
        }
    )
```
