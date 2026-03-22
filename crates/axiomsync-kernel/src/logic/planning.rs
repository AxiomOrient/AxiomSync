use std::collections::HashMap;

use crate::domain::{
    EpisodeExtraction, IngestPlan, PurgePlan, RawEventRow, RepairPlan, ReplayPlan,
    VerificationExtraction,
};
use crate::error::Result;

use super::derivation::{plan_derivation, plan_derivation_contexts};
use super::projection::plan_projection;

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
    if let Some(expected) = workspace_id {
        let payload: serde_json::Value = serde_json::from_str(&event.payload_json)?;
        let canonical_root =
            super::projection::payload_string(&payload, &["workspace_root", "root", "cwd"])
                .unwrap_or_else(|| ".".to_string());
        if crate::domain::workspace_stable_id(&canonical_root) != expected {
            return Ok(false);
        }
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
    let mut deleted_raw_event_ids = Vec::new();
    let mut surviving = Vec::new();
    for event in raw_events {
        if raw_event_matches_purge(event, connector, workspace_id)? {
            deleted_raw_event_ids.push(event.stable_id.clone());
        } else {
            surviving.push(event.clone());
        }
    }
    let replay = plan_replay(&surviving, extractions, verifications)?;
    let plan = PurgePlan {
        connector: connector.map(ToOwned::to_owned),
        workspace_id: workspace_id.map(ToOwned::to_owned),
        deleted_raw_event_ids,
        projection: replay.projection,
        derivation: replay.derivation,
    };
    plan.validate()?;
    Ok(plan)
}

pub fn plan_repair(
    raw_events: &[RawEventRow],
    ingest: &IngestPlan,
    extractions: &HashMap<String, EpisodeExtraction>,
    verifications: &HashMap<String, Vec<VerificationExtraction>>,
) -> Result<RepairPlan> {
    let mut combined = raw_events.to_vec();
    combined.extend(ingest.adds.iter().map(|event| event.row.clone()));
    let replay = plan_replay(&combined, extractions, verifications)?;
    let plan = RepairPlan {
        ingest: ingest.clone(),
        replay,
    };
    plan.validate()?;
    Ok(plan)
}
