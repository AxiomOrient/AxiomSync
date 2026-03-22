use std::collections::{BTreeSet, HashMap};

use crate::domain::{ConvItemRow, ConvSessionRow, ConvTurnRow, DerivationContext, stable_id};

const EPISODE_GAP_TURNS: usize = 2;

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
