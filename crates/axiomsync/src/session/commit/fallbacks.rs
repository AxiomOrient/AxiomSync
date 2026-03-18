use super::Session;

const MEMORY_DEDUP_CONFIG_WARNING_ERROR: &str = "unsupported memory dedup mode; falling back to auto";

pub(super) fn record_memory_extractor_fallback(
    session: &Session,
    mode_requested: &str,
    error: &str,
) {
    let uri = format!("axiom://session/{}", session.session_id);
    let _ = session.state.enqueue_dead_letter(
        "memory_extract_fallback",
        &uri,
        serde_json::json!({
            "session_id": session.session_id,
            "mode_requested": mode_requested,
            "error": error,
        }),
    );
}

pub(super) fn record_memory_dedup_fallback(session: &Session, mode_requested: &str, error: &str) {
    let uri = format!("axiom://session/{}", session.session_id);
    let _ = session.state.enqueue_dead_letter(
        "memory_dedup_fallback",
        &uri,
        serde_json::json!({
            "session_id": session.session_id,
            "mode_requested": mode_requested,
            "error": error,
        }),
    );
}

pub(super) fn record_memory_dedup_config_warning(session: &Session, mode_requested: &str) {
    let uri = format!("axiom://session/{}", session.session_id);
    let _ = session.state.enqueue_dead_letter(
        "memory_dedup_config",
        &uri,
        serde_json::json!({
            "session_id": session.session_id,
            "mode_requested": mode_requested,
            "mode_selected": "auto",
            "error": MEMORY_DEDUP_CONFIG_WARNING_ERROR,
        }),
    );
}
