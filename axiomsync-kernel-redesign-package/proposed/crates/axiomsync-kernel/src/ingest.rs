use axiomsync_domain::model::{RawEventBatch, RawEventEnvelope};

pub struct IngestPlan {
    pub accepted: Vec<RawEventEnvelope>,
    pub rejected: Vec<(usize, String)>,
    pub projection_required: bool,
}

pub fn plan_ingest(batch: RawEventBatch) -> IngestPlan {
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();

    for (idx, event) in batch.events.into_iter().enumerate() {
        if event.connector.trim().is_empty() {
            rejected.push((idx, "missing connector".to_string()));
            continue;
        }
        if event.event_kind.trim().is_empty() {
            rejected.push((idx, "missing event_kind".to_string()));
            continue;
        }
        accepted.push(event);
    }

    IngestPlan {
        projection_required: !accepted.is_empty(),
        accepted,
        rejected,
    }
}
