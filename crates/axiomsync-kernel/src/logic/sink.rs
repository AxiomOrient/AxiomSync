use crate::domain::{
    AppendRawEventsRequest, ConnectorBatchInput, SourceCursorRow, SourceCursorUpsertPlan,
    UpsertSourceCursorRequest,
};
use crate::error::Result;

pub fn append_request_batch(request: &AppendRawEventsRequest) -> Result<ConnectorBatchInput> {
    request.validate()?;
    Ok(ConnectorBatchInput {
        events: request.events.clone(),
        cursor: None,
    })
}

pub fn plan_source_cursor_upsert(
    request: &UpsertSourceCursorRequest,
) -> Result<SourceCursorUpsertPlan> {
    request.validate()?;
    let plan = SourceCursorUpsertPlan {
        cursor: SourceCursorRow {
            connector: request.source.clone(),
            cursor_key: request.cursor.cursor_key.clone(),
            cursor_value: request.cursor.cursor_value.clone(),
            updated_at_ms: request.cursor.updated_at_ms,
        },
    };
    plan.validate()?;
    Ok(plan)
}
