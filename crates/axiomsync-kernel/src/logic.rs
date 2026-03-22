use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde_json::{Value, json};

use crate::domain::{
    ArtifactRow, AuthGrantRecord, AuthSnapshot, ConnectorBatchInput, ConvItemRow, ConvSessionRow,
    ConvTurnRow, CursorInput, DerivationContext, DerivePlan, EpisodeConnectorRow,
    EpisodeEvidenceSearchRow, EpisodeExtraction, EpisodeMemberRow, EpisodeRow, EpisodeStatus,
    EvidenceAnchorRow, ExistingRawEventKey, IngestPlan, InsightAnchorRow, InsightKind, InsightRow,
    ItemType, NormalizedRawEvent, ProjectionPlan, PurgePlan, RawEventInput, RawEventRow,
    RepairPlan, ReplayPlan, RunbookRecord, RunbookVerification, SearchCommandCandidateRow,
    SearchCommandsResult, SearchEpisodeFtsRow, SearchEpisodesFilter, SearchEpisodesResult,
    SelectorType, SourceCursorRow, VerificationExtraction, VerificationKind, VerificationRow,
    VerificationStatus, WorkspaceRow, WorkspaceTokenPlan, build_search_doc_redacted,
    canonical_json, canonical_json_string, normalize_fts_query as normalize_fts_query_impl,
    stable_hash, stable_id, workspace_stable_id,
};
use crate::error::{AxiomError, Result};

const EPISODE_GAP_TURNS: usize = 2;

pub struct EpisodeSearchRows<'a> {
    pub fts_rows: &'a [SearchEpisodeFtsRow],
    pub evidence_rows: &'a [EpisodeEvidenceSearchRow],
    pub episodes: &'a [EpisodeRow],
    pub insights: &'a [InsightRow],
    pub verifications: &'a [VerificationRow],
    pub connector_rows: &'a [EpisodeConnectorRow],
}

pub fn normalize_raw_event(input: &RawEventInput) -> Result<NormalizedRawEvent> {
    input.validate()?;
    let payload = canonical_json(&input.payload);
    let payload_json = canonical_json_string(&payload);
    let payload_sha256_hex = stable_hash(&[payload_json.as_str()]);
    let event_basis = json!({
        "connector": input.connector.trim(),
        "native_session_id": input.native_session_id.trim(),
        "native_event_id": input.native_event_id.as_deref().map(str::trim),
        "event_type": input.event_type.trim(),
        "ts_ms": input.ts_ms,
        "payload_sha256": payload_sha256_hex,
    });
    Ok(NormalizedRawEvent {
        row: RawEventRow {
            stable_id: stable_id("raw", &event_basis),
            connector: input.connector.trim().to_string(),
            native_schema_version: input.native_schema_version.clone(),
            native_session_id: input.native_session_id.trim().to_string(),
            native_event_id: input
                .native_event_id
                .clone()
                .map(|value| value.trim().to_string()),
            event_type: input.event_type.trim().to_string(),
            ts_ms: input.ts_ms,
            payload_json,
            payload_sha256_hex,
        },
        dedupe_key: stable_id("dedupe", &event_basis),
    })
}

pub fn plan_ingest(
    existing: &[ExistingRawEventKey],
    input: &ConnectorBatchInput,
) -> Result<IngestPlan> {
    let existing_keys: BTreeSet<_> = existing.iter().map(|row| row.dedupe_key.clone()).collect();
    let mut adds = Vec::new();
    let mut skipped_dedupe_keys = Vec::new();
    for event in &input.events {
        let normalized = normalize_raw_event(event)?;
        if existing_keys.contains(&normalized.dedupe_key)
            || adds
                .iter()
                .any(|candidate: &NormalizedRawEvent| candidate.dedupe_key == normalized.dedupe_key)
        {
            skipped_dedupe_keys.push(normalized.dedupe_key.clone());
            continue;
        }
        adds.push(normalized);
    }
    let cursor_update = input.cursor.as_ref().and_then(|cursor| {
        input.events.first().map(|first| SourceCursorRow {
            connector: first.connector.clone(),
            cursor_key: cursor.cursor_key.clone(),
            cursor_value: cursor.cursor_value.clone(),
            updated_at_ms: cursor.updated_at_ms,
        })
    });
    let journal = input.events.first().map(|first| {
        let cursor_key = input
            .cursor
            .as_ref()
            .map(|cursor| cursor.cursor_key.clone());
        let cursor_value = input
            .cursor
            .as_ref()
            .map(|cursor| cursor.cursor_value.clone());
        crate::domain::ImportJournalRow {
            stable_id: stable_id(
                "import",
                &json!({
                    "connector": first.connector,
                    "adds": adds.iter().map(|row| row.row.stable_id.clone()).collect::<Vec<_>>(),
                    "skipped": skipped_dedupe_keys,
                    "cursor_key": cursor_key,
                    "cursor_value": cursor_value,
                }),
            ),
            connector: first.connector.clone(),
            imported_events: adds.len(),
            skipped_events: skipped_dedupe_keys.len(),
            cursor_key,
            cursor_value,
            applied_at_ms: input
                .cursor
                .as_ref()
                .map(|cursor| cursor.updated_at_ms)
                .unwrap_or_else(|| {
                    input
                        .events
                        .iter()
                        .map(|event| event.ts_ms)
                        .max()
                        .unwrap_or_default()
                }),
        }
    });
    let plan = IngestPlan {
        adds,
        cursor_update,
        skipped_dedupe_keys,
        journal,
    };
    plan.validate()?;
    Ok(plan)
}

fn payload_value(payload_json: &str) -> Result<Value> {
    serde_json::from_str(payload_json).map_err(Into::into)
}

fn payload_string(payload: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .filter_map(|key| payload.get(*key))
        .find_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn workspace_for_event(event: &RawEventRow) -> Result<WorkspaceRow> {
    let payload = payload_value(&event.payload_json)?;
    let canonical_root = payload_string(
        &payload,
        &["workspace_root", "canonical_root", "worktree_path", "cwd"],
    )
    .unwrap_or_else(|| "global/default".to_string());
    Ok(WorkspaceRow {
        stable_id: workspace_stable_id(&canonical_root),
        canonical_root: canonical_root.clone(),
        repo_remote: payload_string(&payload, &["repo_remote"]),
        branch: payload_string(&payload, &["branch"]),
        worktree_path: payload_string(&payload, &["worktree_path", "cwd"]),
    })
}

fn derive_actor(event: &RawEventRow) -> Result<String> {
    let payload = payload_value(&event.payload_json)?;
    Ok(
        payload_string(&payload, &["actor", "role"]).unwrap_or_else(|| {
            if event.event_type.contains("tool") {
                "tool".to_string()
            } else if event.event_type.contains("assistant")
                || event.event_type.contains("response")
            {
                "assistant".to_string()
            } else {
                "user".to_string()
            }
        }),
    )
}

fn derive_item_type(event: &RawEventRow) -> Result<ItemType> {
    let payload = payload_value(&event.payload_json)?;
    let actor = derive_actor(event)?;
    Ok(payload_string(&payload, &["item_type"])
        .as_deref()
        .and_then(|value| ItemType::parse(value).ok())
        .unwrap_or_else(|| match actor.as_str() {
            "assistant" => ItemType::AssistantMsg,
            "tool" => {
                if event.event_type.contains("result") || event.event_type.contains("output") {
                    ItemType::ToolResult
                } else {
                    ItemType::ToolCall
                }
            }
            _ => ItemType::UserMsg,
        }))
}

fn derive_body_text(event: &RawEventRow) -> Result<Option<String>> {
    let payload = payload_value(&event.payload_json)?;
    Ok(payload_string(
        &payload,
        &["body_text", "text", "content", "message"],
    ))
}

fn turn_sort_key(items: &[RawEventRow]) -> i64 {
    items
        .iter()
        .map(|event| event.ts_ms)
        .min()
        .unwrap_or_default()
}

pub fn plan_projection(raw_events: &[RawEventRow]) -> Result<ProjectionPlan> {
    let mut workspaces_by_id = BTreeMap::<String, WorkspaceRow>::new();
    let mut session_events = BTreeMap::<(String, String), Vec<RawEventRow>>::new();
    for event in raw_events {
        event.validate()?;
        let workspace = workspace_for_event(event)?;
        workspaces_by_id.insert(workspace.stable_id.clone(), workspace);
        session_events
            .entry((event.connector.clone(), event.native_session_id.clone()))
            .or_default()
            .push(event.clone());
    }

    let mut conv_sessions = Vec::new();
    let mut conv_turns = Vec::new();
    let mut conv_items = Vec::new();
    let mut artifacts = Vec::new();
    let mut evidence_anchors = Vec::new();

    for ((connector, native_session_id), mut events) in session_events {
        let workspace = workspace_for_event(events.first().ok_or_else(|| {
            AxiomError::Validation("session must contain at least one raw event".to_string())
        })?)?;
        events.sort_by(|left, right| {
            left.ts_ms
                .cmp(&right.ts_ms)
                .then(left.stable_id.cmp(&right.stable_id))
        });
        let session_payload = payload_value(&events[0].payload_json)?;
        let last_payload = payload_value(
            &events
                .last()
                .ok_or_else(|| {
                    AxiomError::Validation("session must contain a last raw event".to_string())
                })?
                .payload_json,
        )?;
        let session_record = ConvSessionRow {
            stable_id: stable_id("session", &(connector.as_str(), native_session_id.as_str())),
            connector: connector.clone(),
            native_session_id: native_session_id.clone(),
            workspace_id: Some(workspace.stable_id.clone()),
            title: payload_string(&session_payload, &["title", "session_title"]),
            transcript_uri: payload_string(&session_payload, &["transcript_uri"]),
            status: payload_string(&last_payload, &["status"])
                .unwrap_or_else(|| "active".to_string()),
            started_at_ms: events.first().map(|event| event.ts_ms),
            ended_at_ms: events.last().map(|event| event.ts_ms),
        };
        let session_id = session_record.stable_id.clone();
        conv_sessions.push(session_record);

        let mut turns = BTreeMap::<String, Vec<RawEventRow>>::new();
        for event in events {
            let payload = payload_value(&event.payload_json)?;
            let turn_key =
                if let Some(turn_id) = payload_string(&payload, &["turn_id", "native_turn_id"]) {
                    turn_id
                } else {
                    stable_id("turnkey", &(event.ts_ms / 1_000, derive_actor(&event)?))
                };
            turns.entry(turn_key).or_default().push(event);
        }

        let mut ordered_turns = turns.into_iter().collect::<Vec<_>>();
        ordered_turns.sort_by(|left, right| {
            turn_sort_key(&left.1)
                .cmp(&turn_sort_key(&right.1))
                .then(left.0.cmp(&right.0))
        });

        for (turn_index, (turn_key, turn_events)) in ordered_turns.into_iter().enumerate() {
            let actor = derive_actor(turn_events.first().ok_or_else(|| {
                AxiomError::Validation("turn must contain at least one raw event".to_string())
            })?)?;
            let turn_record = ConvTurnRow {
                stable_id: stable_id(
                    "turn",
                    &(session_id.as_str(), turn_key.as_str(), turn_index),
                ),
                session_id: session_id.clone(),
                native_turn_id: Some(turn_key),
                turn_index,
                actor,
            };
            let turn_id = turn_record.stable_id.clone();
            conv_turns.push(turn_record);

            for event in turn_events {
                let item_record = ConvItemRow {
                    stable_id: stable_id("item", &(turn_id.as_str(), event.stable_id.as_str())),
                    turn_id: turn_id.clone(),
                    item_type: derive_item_type(&event)?,
                    tool_name: {
                        let payload = payload_value(&event.payload_json)?;
                        payload_string(&payload, &["tool_name", "tool"])
                    },
                    body_text: derive_body_text(&event)?,
                    payload_json: event.payload_json.clone(),
                };
                let item_id = item_record.stable_id.clone();
                if let Some(text) = item_record.body_text.as_ref() {
                    evidence_anchors.push(EvidenceAnchorRow {
                        stable_id: stable_id("anchor", &(item_id.as_str(), "body")),
                        item_id: item_id.clone(),
                        selector_type: SelectorType::TextSpan,
                        selector_json: json!({"start": 0, "end": text.chars().count()}).to_string(),
                        quoted_text: Some(text.chars().take(200).collect()),
                    });
                } else {
                    evidence_anchors.push(EvidenceAnchorRow {
                        stable_id: stable_id("anchor", &(item_id.as_str(), "payload")),
                        item_id: item_id.clone(),
                        selector_type: SelectorType::JsonPointer,
                        selector_json: Value::String("/".to_string()).to_string(),
                        quoted_text: None,
                    });
                }

                let payload = payload_value(&item_record.payload_json)?;
                if let Some(items) = payload.get("artifacts").and_then(Value::as_array) {
                    for artifact in items {
                        let uri = payload_string(artifact, &["uri", "path"]).unwrap_or_default();
                        artifacts.push(ArtifactRow {
                            stable_id: stable_id("artifact", &(item_id.as_str(), uri.as_str())),
                            item_id: item_id.clone(),
                            uri,
                            mime: payload_string(artifact, &["mime"]),
                            sha256_hex: payload_string(artifact, &["sha256"]),
                            bytes: artifact
                                .get("bytes")
                                .and_then(Value::as_u64)
                                .or_else(|| artifact.get("size").and_then(Value::as_u64)),
                        });
                    }
                }

                conv_items.push(item_record);
            }
        }
    }

    conv_sessions.sort_by(|left, right| left.stable_id.cmp(&right.stable_id));
    conv_turns.sort_by(|left, right| {
        left.session_id
            .cmp(&right.session_id)
            .then(left.turn_index.cmp(&right.turn_index))
    });
    conv_items.sort_by(|left, right| {
        left.turn_id
            .cmp(&right.turn_id)
            .then(left.stable_id.cmp(&right.stable_id))
    });
    artifacts.sort_by(|left, right| {
        left.item_id
            .cmp(&right.item_id)
            .then(left.stable_id.cmp(&right.stable_id))
    });
    evidence_anchors.sort_by(|left, right| {
        left.item_id
            .cmp(&right.item_id)
            .then(left.stable_id.cmp(&right.stable_id))
    });

    let plan = ProjectionPlan {
        workspaces: workspaces_by_id.into_values().collect(),
        conv_sessions,
        conv_turns,
        conv_items,
        artifacts,
        evidence_anchors,
    };
    plan.validate()?;
    Ok(plan)
}

fn transcript_for_turns(
    turns: &[ConvTurnRow],
    items_by_turn: &HashMap<String, Vec<ConvItemRow>>,
) -> String {
    let mut chunks = Vec::new();
    for turn in turns {
        if let Some(items) = items_by_turn.get(&turn.stable_id) {
            for item in items {
                let body = item
                    .body_text
                    .clone()
                    .unwrap_or_else(|| item.payload_json.clone());
                chunks.push(format!("{}: {}", turn.actor, body));
            }
        }
    }
    chunks.join("\n")
}

fn transcript_tokens(text: &str) -> BTreeSet<String> {
    text.split(|ch: char| !ch.is_alphanumeric())
        .filter(|token| token.len() >= 3)
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

fn topic_shift(current: &str, next: &str) -> bool {
    let current_tokens = transcript_tokens(current);
    let next_tokens = transcript_tokens(next);
    if current_tokens.is_empty() || next_tokens.is_empty() {
        return false;
    }
    let overlap = current_tokens.intersection(&next_tokens).count();
    overlap * 4 < next_tokens.len()
}

pub fn plan_derivation_contexts(
    sessions: &[ConvSessionRow],
    turns: &[ConvTurnRow],
    items: &[ConvItemRow],
) -> Vec<DerivationContext> {
    let mut items_by_turn: HashMap<String, Vec<ConvItemRow>> = HashMap::new();
    for item in items {
        items_by_turn
            .entry(item.turn_id.clone())
            .or_default()
            .push(item.clone());
    }
    for values in items_by_turn.values_mut() {
        values.sort_by(|left, right| left.stable_id.cmp(&right.stable_id));
    }

    let mut turns_by_session: HashMap<String, Vec<ConvTurnRow>> = HashMap::new();
    for turn in turns {
        turns_by_session
            .entry(turn.session_id.clone())
            .or_default()
            .push(turn.clone());
    }
    for session_turns in turns_by_session.values_mut() {
        session_turns.sort_by(|left, right| {
            left.turn_index
                .cmp(&right.turn_index)
                .then(left.stable_id.cmp(&right.stable_id))
        });
    }

    let mut contexts = Vec::new();
    let mut ordered_sessions = sessions.to_vec();
    ordered_sessions.sort_by(|left, right| {
        left.started_at_ms
            .cmp(&right.started_at_ms)
            .then(left.stable_id.cmp(&right.stable_id))
    });

    for session in ordered_sessions {
        let Some(session_turns) = turns_by_session.get(&session.stable_id) else {
            continue;
        };
        let mut current = Vec::<ConvTurnRow>::new();
        let flush = |turns: &[ConvTurnRow], contexts: &mut Vec<DerivationContext>| {
            if turns.is_empty() {
                return;
            }
            let transcript = transcript_for_turns(turns, &items_by_turn);
            contexts.push(DerivationContext {
                episode_id: stable_id("episode", &transcript),
                workspace_id: session.workspace_id.clone(),
                turn_ids: turns.iter().map(|turn| turn.stable_id.clone()).collect(),
                opened_at_ms: session.started_at_ms.unwrap_or_default(),
                closed_at_ms: session.ended_at_ms,
                transcript,
            });
        };

        for turn in session_turns {
            let single_transcript =
                transcript_for_turns(std::slice::from_ref(turn), &items_by_turn);
            let should_split = !current.is_empty()
                && turn.actor == "user"
                && (current.len() >= EPISODE_GAP_TURNS
                    || topic_shift(
                        &transcript_for_turns(&current, &items_by_turn),
                        &single_transcript,
                    ));
            if should_split {
                flush(&current, &mut contexts);
                current.clear();
            }
            current.push(turn.clone());
        }
        flush(&current, &mut contexts);
    }
    contexts
}

fn find_anchor_for_text(
    anchor_pool: &[EvidenceAnchorRow],
    items_by_id: &HashMap<String, ConvItemRow>,
    needle: Option<&str>,
) -> Option<String> {
    let needle = needle.map(str::trim).filter(|text| !text.is_empty())?;
    let needle_lower = needle.to_ascii_lowercase();
    anchor_pool.iter().find_map(|anchor| {
        let quoted = anchor
            .quoted_text
            .as_ref()
            .map(|value| value.to_ascii_lowercase())
            .unwrap_or_default();
        let body = items_by_id
            .get(&anchor.item_id)
            .and_then(|item| item.body_text.as_ref())
            .map(|value| value.to_ascii_lowercase())
            .unwrap_or_default();
        if quoted.contains(&needle_lower) || body.contains(&needle_lower) {
            Some(anchor.stable_id.clone())
        } else {
            None
        }
    })
}

pub fn plan_derivation(
    contexts: &[DerivationContext],
    turns: &[ConvTurnRow],
    items: &[ConvItemRow],
    anchors: &[EvidenceAnchorRow],
    extractions: &HashMap<String, EpisodeExtraction>,
    verifications: &HashMap<String, Vec<VerificationExtraction>>,
) -> Result<DerivePlan> {
    let turns_by_id: HashMap<_, _> = turns
        .iter()
        .map(|turn| (turn.stable_id.clone(), turn.clone()))
        .collect();
    let items_by_turn: HashMap<_, _> = items.iter().fold(
        HashMap::<String, Vec<ConvItemRow>>::new(),
        |mut acc, item| {
            acc.entry(item.turn_id.clone())
                .or_default()
                .push(item.clone());
            acc
        },
    );
    let items_by_id: HashMap<_, _> = items
        .iter()
        .map(|item| (item.stable_id.clone(), item.clone()))
        .collect();

    let mut episodes = Vec::new();
    let mut episode_members = Vec::new();
    let mut insights = Vec::new();
    let mut insight_anchors = Vec::new();
    let mut verification_rows = Vec::new();

    for context in contexts {
        let extraction = extractions
            .get(&context.episode_id)
            .cloned()
            .unwrap_or_else(|| EpisodeExtraction {
                problem: context
                    .transcript
                    .lines()
                    .next()
                    .unwrap_or("Conversation episode")
                    .to_string(),
                ..EpisodeExtraction::default()
            });
        let turn_anchor_pool = context
            .turn_ids
            .iter()
            .flat_map(|turn_id| {
                items_by_turn
                    .get(turn_id)
                    .into_iter()
                    .flat_map(|items| items.iter())
                    .filter_map(|item| {
                        anchors
                            .iter()
                            .find(|anchor| anchor.item_id == item.stable_id)
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let episode_status = if verifications
            .get(&context.episode_id)
            .into_iter()
            .flatten()
            .any(|verification| verification.status == VerificationStatus::Pass)
        {
            EpisodeStatus::Solved
        } else {
            EpisodeStatus::Open
        };

        episodes.push(EpisodeRow {
            stable_id: context.episode_id.clone(),
            workspace_id: context.workspace_id.clone(),
            problem_signature: stable_hash(&[extraction.problem.as_str()]),
            status: episode_status,
            opened_at_ms: context.opened_at_ms,
            closed_at_ms: context.closed_at_ms,
        });

        for turn_id in &context.turn_ids {
            if turns_by_id.contains_key(turn_id) {
                episode_members.push(EpisodeMemberRow {
                    episode_id: context.episode_id.clone(),
                    turn_id: turn_id.clone(),
                });
            }
        }

        let mut push_insight = |kind: InsightKind, summary: Option<String>, confidence: f64| {
            let Some(summary) = summary.filter(|value| !value.trim().is_empty()) else {
                return;
            };
            let stable_id_value = stable_id(
                "insight",
                &(context.episode_id.as_str(), kind.as_str(), summary.as_str()),
            );
            let anchor_id = find_anchor_for_text(&turn_anchor_pool, &items_by_id, Some(&summary))
                .or_else(|| {
                    turn_anchor_pool
                        .first()
                        .map(|anchor| anchor.stable_id.clone())
                });
            insights.push(InsightRow {
                stable_id: stable_id_value.clone(),
                episode_id: context.episode_id.clone(),
                kind,
                summary: summary.clone(),
                normalized_text: summary.to_ascii_lowercase(),
                extractor_version: "episode_extractor_v1".to_string(),
                confidence,
                stale: false,
            });
            if let Some(anchor_id) = anchor_id {
                insight_anchors.push(InsightAnchorRow {
                    insight_id: stable_id_value,
                    anchor_id,
                });
            }
        };

        push_insight(InsightKind::Problem, Some(extraction.problem.clone()), 0.9);
        push_insight(InsightKind::RootCause, extraction.root_cause.clone(), 0.7);
        push_insight(InsightKind::Fix, extraction.fix.clone(), 0.8);
        for decision in extraction.decisions {
            push_insight(InsightKind::Decision, Some(decision), 0.6);
        }
        for command in extraction.commands {
            push_insight(InsightKind::Command, Some(command), 0.85);
        }
        for snippet in extraction.snippets {
            push_insight(InsightKind::Snippet, Some(snippet), 0.55);
        }

        for verification in verifications
            .get(&context.episode_id)
            .cloned()
            .unwrap_or_default()
        {
            let evidence_id = find_anchor_for_text(
                &turn_anchor_pool,
                &items_by_id,
                verification
                    .evidence
                    .as_deref()
                    .or(verification.summary.as_deref()),
            );
            verification_rows.push(VerificationRow {
                stable_id: stable_id(
                    "verification",
                    &(
                        context.episode_id.as_str(),
                        verification.kind.as_str(),
                        verification.status.as_str(),
                        verification.summary.as_deref().unwrap_or(""),
                    ),
                ),
                episode_id: context.episode_id.clone(),
                kind: verification.kind,
                status: verification.status,
                summary: verification.summary,
                evidence_id,
            });
        }
    }

    insights.sort_by(|left, right| left.stable_id.cmp(&right.stable_id));
    insight_anchors.sort_by(|left, right| {
        left.insight_id
            .cmp(&right.insight_id)
            .then(left.anchor_id.cmp(&right.anchor_id))
    });
    verification_rows.sort_by(|left, right| left.stable_id.cmp(&right.stable_id));

    let search_docs_redacted = episodes
        .iter()
        .map(|episode| {
            let episode_insights = insights
                .iter()
                .filter(|insight| insight.episode_id == episode.stable_id)
                .cloned()
                .collect::<Vec<_>>();
            let episode_verifications = verification_rows
                .iter()
                .filter(|row| row.episode_id == episode.stable_id)
                .cloned()
                .collect::<Vec<_>>();
            build_search_doc_redacted(
                &episode.stable_id,
                &episode_insights,
                &episode_verifications,
            )
        })
        .collect::<Vec<_>>();

    let plan = DerivePlan {
        episodes,
        episode_members,
        insights,
        insight_anchors,
        verifications: verification_rows,
        search_docs_redacted,
    };
    plan.validate()?;
    Ok(plan)
}

pub fn plan_replay(
    raw_events: &[RawEventRow],
    extractions: &HashMap<String, EpisodeExtraction>,
    verifications: &HashMap<String, Vec<VerificationExtraction>>,
) -> Result<ReplayPlan> {
    let projection = plan_projection(raw_events)?;
    let contexts = plan_derivation_contexts(
        &projection.conv_sessions,
        &projection.conv_turns,
        &projection.conv_items,
    );
    let derivation = plan_derivation(
        &contexts,
        &projection.conv_turns,
        &projection.conv_items,
        &projection.evidence_anchors,
        extractions,
        verifications,
    )?;
    let plan = ReplayPlan {
        projection,
        derivation,
    };
    plan.validate()?;
    Ok(plan)
}

pub fn raw_event_matches_purge(
    event: &RawEventRow,
    connector: Option<&str>,
    workspace_id: Option<&str>,
) -> Result<bool> {
    if let Some(expected) = connector
        && event.connector != expected
    {
        return Ok(false);
    }
    if let Some(expected) = workspace_id
        && workspace_for_event(event)?.stable_id != expected
    {
        return Ok(false);
    }
    Ok(true)
}

pub fn plan_purge(
    raw_events: &[RawEventRow],
    connector: Option<&str>,
    workspace_id: Option<&str>,
    extractions: &HashMap<String, EpisodeExtraction>,
    verifications: &HashMap<String, Vec<VerificationExtraction>>,
) -> Result<PurgePlan> {
    let deleted_raw_event_ids = raw_events
        .iter()
        .try_fold(BTreeSet::new(), |mut acc, event| {
            if raw_event_matches_purge(event, connector, workspace_id)? {
                acc.insert(event.stable_id.clone());
            }
            Ok::<_, AxiomError>(acc)
        })?;
    let surviving_events = raw_events
        .iter()
        .filter(|event| !deleted_raw_event_ids.contains(&event.stable_id))
        .cloned()
        .collect::<Vec<_>>();
    let replay = plan_replay(&surviving_events, extractions, verifications)?;
    let plan = PurgePlan {
        connector: connector.map(ToOwned::to_owned),
        workspace_id: workspace_id.map(ToOwned::to_owned),
        deleted_raw_event_ids: deleted_raw_event_ids.into_iter().collect(),
        projection: replay.projection,
        derivation: replay.derivation,
    };
    plan.validate()?;
    Ok(plan)
}

pub fn plan_repair(
    existing_raw_events: &[RawEventRow],
    ingest: &IngestPlan,
    extractions: &HashMap<String, EpisodeExtraction>,
    verifications: &HashMap<String, Vec<VerificationExtraction>>,
) -> Result<RepairPlan> {
    let mut raw_events = existing_raw_events.to_vec();
    raw_events.extend(ingest.adds.iter().map(|event| event.row.clone()));
    let plan = RepairPlan {
        ingest: ingest.clone(),
        replay: plan_replay(&raw_events, extractions, verifications)?,
    };
    plan.validate()?;
    Ok(plan)
}

pub fn plan_workspace_token_grant(canonical_root: &str, token: &str) -> Result<WorkspaceTokenPlan> {
    if canonical_root.trim().is_empty() {
        return Err(AxiomError::Validation(
            "workspace_root must not be empty".to_string(),
        ));
    }
    if token.trim().is_empty() {
        return Err(AxiomError::Validation(
            "token must not be empty".to_string(),
        ));
    }
    Ok(WorkspaceTokenPlan {
        workspace_id: workspace_stable_id(canonical_root.trim()),
        token_sha256: stable_hash(&[token.trim()]),
    })
}

pub fn apply_workspace_token_plan(
    snapshot: &AuthSnapshot,
    plan: &WorkspaceTokenPlan,
) -> AuthSnapshot {
    let mut next = snapshot.clone();
    next.schema_version = crate::domain::RENEWAL_SCHEMA_VERSION.to_string();
    next.grants
        .retain(|grant| grant.workspace_id != plan.workspace_id);
    next.grants.push(AuthGrantRecord {
        workspace_id: plan.workspace_id.clone(),
        token_sha256: plan.token_sha256.clone(),
    });
    next.grants
        .sort_by(|left, right| left.workspace_id.cmp(&right.workspace_id));
    next
}

pub fn synthesize_runbook(
    episode: &EpisodeRow,
    insights: &[InsightRow],
    insight_anchors: &[InsightAnchorRow],
    verifications: &[VerificationRow],
) -> Result<RunbookRecord> {
    let problem = insights
        .iter()
        .find(|insight| insight.kind == InsightKind::Problem)
        .map(|insight| insight.summary.clone())
        .unwrap_or_else(|| episode.problem_signature.clone());
    let root_cause = insights
        .iter()
        .find(|insight| insight.kind == InsightKind::RootCause)
        .map(|insight| insight.summary.clone());
    let fix = insights
        .iter()
        .find(|insight| insight.kind == InsightKind::Fix)
        .map(|insight| insight.summary.clone());
    let mut commands = insights
        .iter()
        .filter(|insight| insight.kind == InsightKind::Command)
        .map(|insight| insight.summary.clone())
        .collect::<Vec<_>>();
    commands.sort();
    commands.dedup();
    let mut evidence = insight_anchors
        .iter()
        .filter(|anchor| {
            insights
                .iter()
                .any(|insight| insight.stable_id == anchor.insight_id)
        })
        .map(|anchor| format!("axiom://evidence/{}", anchor.anchor_id))
        .collect::<Vec<_>>();
    evidence.sort();
    evidence.dedup();
    let verification = verifications
        .iter()
        .map(|row| RunbookVerification {
            kind: row.kind,
            status: row.status,
            summary: row.summary.clone(),
            evidence: row
                .evidence_id
                .as_ref()
                .map(|value| format!("axiom://evidence/{value}")),
        })
        .collect::<Vec<_>>();
    let runbook = RunbookRecord {
        episode_id: episode.stable_id.clone(),
        workspace_id: episode.workspace_id.clone(),
        problem,
        root_cause,
        fix,
        commands,
        verification,
        evidence,
    };
    runbook.validate()?;
    Ok(runbook)
}

pub fn deterministic_directory_cursor(
    events: &[RawEventInput],
    latest_path: Option<&str>,
) -> Option<CursorInput> {
    latest_path.map(|path| CursorInput {
        cursor_key: "path".to_string(),
        cursor_value: path.to_string(),
        updated_at_ms: events
            .iter()
            .map(|event| event.ts_ms)
            .max()
            .unwrap_or_default(),
    })
}

pub fn normalize_fts_query(query: &str) -> Option<String> {
    normalize_fts_query_impl(query)
}

pub fn filter_matches(
    filter: &SearchEpisodesFilter,
    workspace_id: Option<&str>,
    connector: Option<&str>,
    status: EpisodeStatus,
) -> bool {
    if let Some(expected) = filter.workspace_id.as_deref()
        && workspace_id != Some(expected)
    {
        return false;
    }
    if let Some(expected) = filter.connector.as_deref()
        && connector != Some(expected)
    {
        return false;
    }
    if let Some(expected) = filter.status
        && status != expected
    {
        return false;
    }
    true
}

pub fn search_episode_results(
    query: &str,
    limit: usize,
    filter: &SearchEpisodesFilter,
    rows: EpisodeSearchRows<'_>,
) -> Vec<SearchEpisodesResult> {
    let mut results = aggregate_episode_search_results(rows.fts_rows, filter, rows.insights);
    if results.is_empty() && !query.trim().is_empty() {
        results = fallback_episode_search_results(
            query,
            filter,
            rows.evidence_rows,
            rows.episodes,
            rows.insights,
            rows.verifications,
            rows.connector_rows,
        );
    }
    sort_episode_search_results(&mut results);
    results.truncate(limit);
    results
}

pub fn search_command_results(
    query: &str,
    limit: usize,
    candidates: &[SearchCommandCandidateRow],
    workspace_id: Option<&str>,
) -> Vec<SearchCommandsResult> {
    let lowered = query.to_ascii_lowercase();
    let mut rows = candidates
        .iter()
        .filter(|candidate| {
            workspace_id.is_none_or(|expected| candidate.workspace_id.as_deref() == Some(expected))
        })
        .filter_map(|candidate| {
            let command_lower = candidate.command.to_ascii_lowercase();
            if !command_lower.contains(&lowered) {
                return None;
            }
            Some(SearchCommandsResult {
                episode_id: candidate.episode_id.clone(),
                command: candidate.command.clone(),
                score: lowered.len() as f64 / command_lower.len().max(1) as f64,
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(left.command.cmp(&right.command))
            .then(left.episode_id.cmp(&right.episode_id))
    });
    rows.truncate(limit);
    rows
}

fn aggregate_episode_search_results(
    fts_rows: &[SearchEpisodeFtsRow],
    filter: &SearchEpisodesFilter,
    insights: &[InsightRow],
) -> Vec<SearchEpisodesResult> {
    let mut aggregated = BTreeMap::<String, SearchEpisodesResult>::new();
    for row in fts_rows.iter().filter(|row| {
        filter_matches(
            filter,
            row.workspace_id.as_deref(),
            row.connector.as_deref(),
            row.status,
        )
    }) {
        let entry =
            aggregated
                .entry(row.episode_id.clone())
                .or_insert_with(|| SearchEpisodesResult {
                    episode_id: row.episode_id.clone(),
                    workspace_id: row.workspace_id.clone(),
                    connector: row.connector.clone(),
                    status: row.status,
                    problem: String::new(),
                    root_cause: None,
                    fix: None,
                    score: f64::MIN,
                });
        match (row.matched_kind, row.matched_summary.as_ref()) {
            (Some(InsightKind::Problem), Some(summary)) if entry.problem.is_empty() => {
                entry.problem = summary.clone()
            }
            (Some(InsightKind::RootCause), Some(summary)) if entry.root_cause.is_none() => {
                entry.root_cause = Some(summary.clone())
            }
            (Some(InsightKind::Fix), Some(summary)) if entry.fix.is_none() => {
                entry.fix = Some(summary.clone())
            }
            _ => {}
        }
        entry.score = entry.score.max(1.0 + f64::from(row.pass_boost));
    }
    for entry in aggregated.values_mut() {
        hydrate_episode_summary(entry, insights);
    }
    aggregated.into_values().collect()
}

fn fallback_episode_search_results(
    query: &str,
    filter: &SearchEpisodesFilter,
    evidence_rows: &[EpisodeEvidenceSearchRow],
    episodes: &[EpisodeRow],
    insights: &[InsightRow],
    verifications: &[VerificationRow],
    connector_rows: &[EpisodeConnectorRow],
) -> Vec<SearchEpisodesResult> {
    let query_lower = query.to_ascii_lowercase();
    let connectors = first_connector_by_episode(connector_rows);
    let evidence_by_episode = evidence_rows.iter().fold(
        BTreeMap::<String, Vec<&EpisodeEvidenceSearchRow>>::new(),
        |mut acc, row| {
            acc.entry(row.episode_id.clone()).or_default().push(row);
            acc
        },
    );
    let mut results = Vec::new();
    for episode in episodes {
        let connector = connectors.get(&episode.stable_id).cloned().flatten();
        if !filter_matches(
            filter,
            episode.workspace_id.as_deref(),
            connector.as_deref(),
            episode.status,
        ) {
            continue;
        }
        let episode_insights = insights
            .iter()
            .filter(|insight| insight.episode_id == episode.stable_id)
            .collect::<Vec<_>>();
        let haystack = episode_insights
            .iter()
            .map(|insight| format!("{} {}", insight.summary, insight.normalized_text))
            .collect::<Vec<_>>()
            .join("\n")
            .to_ascii_lowercase();
        let evidence_haystack = evidence_by_episode
            .get(&episode.stable_id)
            .into_iter()
            .flatten()
            .map(|row| {
                format!(
                    "{}\n{}",
                    row.quoted_text.clone().unwrap_or_default(),
                    row.body_text.clone().unwrap_or_default()
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
            .to_ascii_lowercase();
        let insight_match = haystack.contains(&query_lower);
        let evidence_match = evidence_haystack.contains(&query_lower);
        if !insight_match && !evidence_match {
            continue;
        }
        let has_pass = verifications.iter().any(|verification| {
            verification.episode_id == episode.stable_id
                && verification.status == VerificationStatus::Pass
        });
        results.push(SearchEpisodesResult {
            episode_id: episode.stable_id.clone(),
            workspace_id: episode.workspace_id.clone(),
            connector,
            status: episode.status,
            problem: episode_insights
                .iter()
                .find(|insight| insight.kind == InsightKind::Problem)
                .map(|insight| insight.summary.clone())
                .unwrap_or_default(),
            root_cause: episode_insights
                .iter()
                .find(|insight| insight.kind == InsightKind::RootCause)
                .map(|insight| insight.summary.clone()),
            fix: episode_insights
                .iter()
                .find(|insight| insight.kind == InsightKind::Fix)
                .map(|insight| insight.summary.clone()),
            score: evidence_fallback_score(insight_match, evidence_match, has_pass),
        });
    }
    results
}

fn evidence_fallback_score(insight_match: bool, evidence_match: bool, has_pass: bool) -> f64 {
    let mut score = 0.0;
    if insight_match {
        score += 1.0;
    }
    if evidence_match {
        score += 0.75;
    }
    if has_pass {
        score += 1.0;
    }
    score
}

fn first_connector_by_episode(
    connector_rows: &[EpisodeConnectorRow],
) -> BTreeMap<String, Option<String>> {
    let mut by_episode = BTreeMap::<String, (usize, Option<String>)>::new();
    for row in connector_rows {
        match by_episode.get(&row.episode_id) {
            Some((existing_turn_index, _)) if *existing_turn_index <= row.turn_index => {}
            _ => {
                by_episode.insert(
                    row.episode_id.clone(),
                    (row.turn_index, row.connector.clone()),
                );
            }
        }
    }
    by_episode
        .into_iter()
        .map(|(episode_id, (_, connector))| (episode_id, connector))
        .collect()
}

fn hydrate_episode_summary(entry: &mut SearchEpisodesResult, insights: &[InsightRow]) {
    for insight in insights
        .iter()
        .filter(|insight| insight.episode_id == entry.episode_id)
    {
        match insight.kind {
            InsightKind::Problem if entry.problem.is_empty() => {
                entry.problem = insight.summary.clone()
            }
            InsightKind::RootCause if entry.root_cause.is_none() => {
                entry.root_cause = Some(insight.summary.clone())
            }
            InsightKind::Fix if entry.fix.is_none() => entry.fix = Some(insight.summary.clone()),
            _ => {}
        }
    }
}

fn sort_episode_search_results(results: &mut [SearchEpisodesResult]) {
    results.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(left.episode_id.cmp(&right.episode_id))
    });
}

pub fn parse_verification_transcript(transcript: &str) -> Vec<VerificationExtraction> {
    let mut rows = Vec::new();
    for line in transcript.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lowered = trimmed.to_ascii_lowercase();
        if lowered.contains("pass") {
            rows.push(VerificationExtraction {
                kind: VerificationKind::Test,
                status: VerificationStatus::Pass,
                summary: Some(trimmed.to_string()),
                evidence: Some(trimmed.to_string()),
                pass_condition: Some("transcript contains pass signal".to_string()),
                exit_code: extract_exit_code(trimmed),
                human_confirmed: false,
            });
        }
        if let Some(exit_code) = extract_exit_code(trimmed) {
            rows.push(VerificationExtraction {
                kind: VerificationKind::CommandExit,
                status: if exit_code == 0 {
                    VerificationStatus::Pass
                } else {
                    VerificationStatus::Fail
                },
                summary: Some(trimmed.to_string()),
                evidence: Some(trimmed.to_string()),
                pass_condition: Some("exit code parsed from transcript".to_string()),
                exit_code: Some(exit_code),
                human_confirmed: false,
            });
        }
        if is_human_confirmation(&lowered) {
            rows.push(VerificationExtraction {
                kind: VerificationKind::HumanConfirm,
                status: VerificationStatus::Pass,
                summary: Some(trimmed.to_string()),
                evidence: Some(trimmed.to_string()),
                pass_condition: Some("human confirmation phrase detected".to_string()),
                exit_code: None,
                human_confirmed: true,
            });
        }
    }
    merge_verification_extractions(&rows)
}

pub fn merge_verification_extractions(
    candidates: &[VerificationExtraction],
) -> Vec<VerificationExtraction> {
    let mut ordered = BTreeMap::<String, VerificationExtraction>::new();
    for candidate in candidates {
        let summary = candidate.summary.as_deref().unwrap_or_default();
        let evidence = candidate.evidence.as_deref().unwrap_or_default();
        let pass_condition = candidate.pass_condition.as_deref().unwrap_or_default();
        let key = stable_id(
            "verification_parse",
            &(
                candidate.kind.as_str(),
                candidate.status.as_str(),
                summary,
                evidence,
                pass_condition,
                candidate.exit_code.unwrap_or_default(),
                candidate.human_confirmed,
            ),
        );
        ordered.entry(key).or_insert_with(|| candidate.clone());
    }
    ordered.into_values().collect()
}

fn extract_exit_code(text: &str) -> Option<i64> {
    let digits = text
        .split(|ch: char| !(ch.is_ascii_digit() || ch == '-'))
        .find(|part| !part.is_empty() && part.chars().all(|ch| ch == '-' || ch.is_ascii_digit()))?;
    let has_exit_marker = text.to_ascii_lowercase().contains("exit");
    if !has_exit_marker {
        return None;
    }
    digits.parse().ok()
}

fn is_human_confirmation(lowered: &str) -> bool {
    ["it worked", "works now", "fixed", "resolved", "confirmed"]
        .iter()
        .any(|phrase| lowered.contains(phrase))
}
