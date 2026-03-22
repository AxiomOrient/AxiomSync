use super::*;

pub(super) fn parse_batch(connector: &str, value: Value) -> Result<ConnectorBatchInput> {
    let events = if let Some(array) = value.as_array() {
        array
            .iter()
            .cloned()
            .map(|raw| parse_event(connector, raw))
            .collect::<Result<Vec<_>>>()?
    } else if let Some(events_arr) = value.get("events").and_then(Value::as_array) {
        let events = events_arr
            .iter()
            .cloned()
            .map(|raw| parse_event(connector, raw))
            .collect::<Result<Vec<_>>>()?;
        let cursor = value
            .get("cursor")
            .cloned()
            .map(serde_json::from_value)
            .transpose()?;
        return Ok(ConnectorBatchInput { events, cursor });
    } else {
        vec![parse_event(connector, value)?]
    };
    Ok(ConnectorBatchInput {
        events,
        cursor: None,
    })
}

fn parse_event(connector: &str, raw: Value) -> Result<RawEventInput> {
    let native_session_id = raw
        .get("native_session_id")
        .or_else(|| raw.get("conversation_id"))
        .or_else(|| raw.get("session_id"))
        .and_then(Value::as_str)
        .ok_or_else(|| AxiomError::Validation("missing native_session_id".to_string()))?;
    let event_type = raw
        .get("event_type")
        .or_else(|| raw.get("type"))
        .and_then(Value::as_str)
        .ok_or_else(|| AxiomError::Validation("missing event_type".to_string()))?;
    let ts_ms = raw
        .get("ts_ms")
        .or_else(|| raw.get("timestamp_ms"))
        .or_else(|| raw.get("timestamp"))
        .and_then(Value::as_i64)
        .unwrap_or_default();
    Ok(RawEventInput {
        connector: connector.to_string(),
        native_schema_version: raw
            .get("native_schema_version")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        native_session_id: native_session_id.to_string(),
        native_event_id: raw
            .get("native_event_id")
            .or_else(|| raw.get("message_id"))
            .or_else(|| raw.get("id"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        event_type: event_type.to_string(),
        ts_ms,
        payload: raw,
    })
}
