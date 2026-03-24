use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;

use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use rusqlite::Connection;
use serde::Deserialize;
use serde_json::{Map, Value, json};
use tempfile::tempdir;
use tower::ServiceExt;

use axiomsync_domain::{
    AppendRawEventsRequest, IngestPlan, RawEventInput, SourceCursorUpsertPlan,
    UpsertSourceCursorRequest,
};
use axiomsync_kernel::AxiomSync;

fn fixture_path(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn fixture_json<T: for<'de> Deserialize<'de>>(name: &str) -> T {
    serde_json::from_slice(&fs::read(fixture_path(name)).expect("fixture file"))
        .expect("fixture json")
}

#[derive(Debug, Clone, Deserialize)]
struct RelayPacketBatch {
    batch_id: String,
    producer: String,
    received_at_ms: i64,
    packets: Vec<RelayPacket>,
    #[serde(default)]
    cursor_updates: Vec<RelayCursorUpdate>,
}

#[derive(Debug, Clone, Deserialize)]
struct RelayPacket {
    packet_id: String,
    source_kind: String,
    native_schema_version: Option<String>,
    source_identity: RelaySourceIdentity,
    event_type: String,
    observed_at_ms: i64,
    workspace_hint: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    artifact_refs: Vec<RelayArtifactRef>,
    #[serde(default)]
    payload: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct RelaySourceIdentity {
    connector: String,
    native_session_id: String,
    native_event_id: Option<String>,
    native_entry_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct RelayArtifactRef {
    uri: String,
    mime: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct RelayCursorUpdate {
    connector: String,
    cursor_key: String,
    cursor_value: String,
    updated_at_ms: i64,
}

#[derive(Debug, Clone)]
struct RelayTranslation {
    append_request: AppendRawEventsRequest,
    cursor_requests: Vec<UpsertSourceCursorRequest>,
}

fn translate_batch(batch: &RelayPacketBatch) -> RelayTranslation {
    RelayTranslation {
        append_request: AppendRawEventsRequest {
            batch_id: batch.batch_id.clone(),
            producer: batch.producer.clone(),
            received_at_ms: batch.received_at_ms,
            events: batch
                .packets
                .iter()
                .map(|packet| RawEventInput {
                    connector: packet.source_identity.connector.clone(),
                    native_schema_version: packet.native_schema_version.clone(),
                    session_kind: None,
                    external_session_key: Some(packet.source_identity.native_session_id.clone()),
                    external_entry_key: packet
                        .source_identity
                        .native_event_id
                        .clone()
                        .or_else(|| packet.source_identity.native_entry_id.clone()),
                    event_kind: Some(packet.event_type.clone()),
                    observed_at: None,
                    captured_at: None,
                    workspace_root: None,
                    content_hash: None,
                    dedupe_key: None,
                    ts_ms: Some(packet.observed_at_ms),
                    observed_at_ms: None,
                    captured_at_ms: None,
                    payload: translated_payload(packet),
                    raw_payload: None,
                    artifacts: Vec::new(),
                    hints: json!({}),
                })
                .collect(),
        },
        cursor_requests: batch
            .cursor_updates
            .iter()
            .map(|cursor| UpsertSourceCursorRequest {
                connector: cursor.connector.clone(),
                cursor_key: cursor.cursor_key.clone(),
                cursor_value: cursor.cursor_value.clone(),
                updated_at_ms: cursor.updated_at_ms,
            })
            .collect(),
    }
}

fn translated_payload(packet: &RelayPacket) -> Value {
    let mut map = match packet.payload.clone() {
        Value::Object(map) => map,
        other => {
            let mut map = Map::new();
            map.insert("value".to_string(), other);
            map
        }
    };
    map.insert(
        "relay".to_string(),
        json!({
            "packet_id": packet.packet_id,
            "source_kind": packet.source_kind,
        }),
    );
    if let Some(workspace_hint) = &packet.workspace_hint {
        map.entry("workspace_root".to_string())
            .or_insert_with(|| Value::String(workspace_hint.clone()));
    }
    if !packet.labels.is_empty() {
        map.entry("labels".to_string()).or_insert_with(|| {
            Value::Array(packet.labels.iter().cloned().map(Value::String).collect())
        });
    }
    if !packet.artifact_refs.is_empty() {
        map.entry("artifact_refs".to_string()).or_insert_with(|| {
            Value::Array(
                packet
                    .artifact_refs
                    .iter()
                    .map(|artifact| {
                        json!({
                            "uri": artifact.uri,
                            "mime": artifact.mime,
                        })
                    })
                    .collect(),
            )
        });
    }
    Value::Object(map)
}

struct RelayHarness {
    router: axum::Router,
    translation: RelayTranslation,
    raw_plan: Option<IngestPlan>,
    cursor_plans: Vec<SourceCursorUpsertPlan>,
    raw_applied: bool,
    cursors_applied: bool,
}

impl RelayHarness {
    fn new(app: AxiomSync, translation: RelayTranslation) -> Self {
        Self {
            router: axiomsync_http::router(app),
            translation,
            raw_plan: None,
            cursor_plans: Vec::new(),
            raw_applied: false,
            cursors_applied: false,
        }
    }

    fn commit_allowed(&self) -> bool {
        self.raw_applied && self.cursors_applied
    }

    async fn plan_raw(&mut self) -> Result<&IngestPlan, String> {
        if self.raw_plan.is_some() {
            return Err("raw plan already built".to_string());
        }
        let plan: IngestPlan = post_loopback(
            &self.router,
            "/sink/raw-events/plan",
            &self.translation.append_request,
        )
        .await?;
        self.raw_plan = Some(plan);
        Ok(self.raw_plan.as_ref().expect("raw plan"))
    }

    async fn apply_raw(&mut self) -> Result<(), String> {
        let plan = self
            .raw_plan
            .clone()
            .ok_or_else(|| "raw plan must be built before raw apply".to_string())?;
        let _: Value = post_loopback(&self.router, "/sink/raw-events/apply", &plan).await?;
        self.raw_applied = true;
        Ok(())
    }

    async fn plan_cursors(&mut self) -> Result<&[SourceCursorUpsertPlan], String> {
        self.plan_cursors_with(self.translation.cursor_requests.clone())
            .await
    }

    async fn plan_cursors_with(
        &mut self,
        requests: Vec<UpsertSourceCursorRequest>,
    ) -> Result<&[SourceCursorUpsertPlan], String> {
        if !self.raw_applied {
            return Err("raw apply must succeed before cursor planning".to_string());
        }
        self.cursor_plans.clear();
        for request in requests {
            let plan: SourceCursorUpsertPlan =
                post_loopback(&self.router, "/sink/source-cursors/plan", &request).await?;
            self.cursor_plans.push(plan);
        }
        Ok(&self.cursor_plans)
    }

    async fn apply_cursors(&mut self) -> Result<(), String> {
        if self.cursor_plans.is_empty() {
            return Err("cursor plan must be built before cursor apply".to_string());
        }
        for plan in &self.cursor_plans {
            let _: Value = post_loopback(&self.router, "/sink/source-cursors/apply", plan).await?;
        }
        self.cursors_applied = true;
        Ok(())
    }
}

async fn post_loopback<T: serde::Serialize, R: for<'de> Deserialize<'de>>(
    router: &axum::Router,
    uri: &str,
    payload: &T,
) -> Result<R, String> {
    let mut request = Request::builder()
        .uri(uri)
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(payload).map_err(|error| error.to_string())?,
        ))
        .map_err(|error| error.to_string())?;
    request.extensions_mut().insert(ConnectInfo(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        4400,
    )));
    let response = router
        .clone()
        .oneshot(request)
        .await
        .map_err(|error| error.to_string())?;
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .map_err(|error| error.to_string())?;
    if status != StatusCode::OK {
        return Err(format!(
            "{uri} returned {status}: {}",
            String::from_utf8_lossy(&body)
        ));
    }
    serde_json::from_slice(&body).map_err(|error| error.to_string())
}

#[test]
fn relay_fixture_translates_to_expected_sink_requests() {
    let batch: RelayPacketBatch = fixture_json("relay_packet_batch.json");
    let translation = translate_batch(&batch);
    let expected_append: AppendRawEventsRequest =
        fixture_json("relay_expected_append_raw_events.json");
    let expected_cursors: Vec<UpsertSourceCursorRequest> =
        fixture_json("relay_expected_cursor_upserts.json");

    assert_eq!(translation.append_request, expected_append);
    assert_eq!(translation.cursor_requests, expected_cursors);
}

#[tokio::test]
async fn relay_http_delivery_smoke_commits_only_after_both_apply_phases() {
    let temp = tempdir().expect("tempdir");
    let app = axiomsync_cli::open(temp.path()).expect("app");
    let batch: RelayPacketBatch = fixture_json("relay_packet_batch.json");
    let translation = translate_batch(&batch);
    let mut harness = RelayHarness::new(app.clone(), translation);

    assert!(!harness.commit_allowed());
    assert!(harness.plan_cursors().await.is_err());

    let raw_plan = harness.plan_raw().await.expect("raw plan");
    assert_eq!(raw_plan.receipts.len(), 2);
    harness.apply_raw().await.expect("raw apply");
    assert!(!harness.commit_allowed());

    let cursor_plans = harness.plan_cursors().await.expect("cursor plans");
    assert_eq!(cursor_plans.len(), 1);
    assert!(!harness.commit_allowed());

    harness.apply_cursors().await.expect("cursor apply");
    assert!(harness.commit_allowed());

    let conn = Connection::open(app.db_path()).expect("sqlite");
    let receipt_count: i64 = conn
        .query_row("select count(*) from ingress_receipts", [], |row| {
            row.get(0)
        })
        .expect("receipt count");
    let cursor_count: i64 = conn
        .query_row("select count(*) from source_cursor", [], |row| row.get(0))
        .expect("cursor count");
    assert_eq!(receipt_count, 2);
    assert_eq!(cursor_count, 1);
}

#[tokio::test]
async fn relay_duplicate_delivery_is_idempotent_across_both_sink_phases() {
    let temp = tempdir().expect("tempdir");
    let app = axiomsync_cli::open(temp.path()).expect("app");
    let batch: RelayPacketBatch = fixture_json("relay_packet_batch.json");
    let translation = translate_batch(&batch);

    let mut first = RelayHarness::new(app.clone(), translation.clone());
    first.plan_raw().await.expect("first raw plan");
    first.apply_raw().await.expect("first raw apply");
    first.plan_cursors().await.expect("first cursor plan");
    first.apply_cursors().await.expect("first cursor apply");
    assert!(first.commit_allowed());

    let mut second = RelayHarness::new(app.clone(), translation);
    let second_raw_plan = second.plan_raw().await.expect("second raw plan");
    assert!(second_raw_plan.receipts.is_empty());
    assert_eq!(second_raw_plan.skipped_dedupe_keys.len(), 2);
    second.apply_raw().await.expect("second raw apply");
    second.plan_cursors().await.expect("second cursor plan");
    second.apply_cursors().await.expect("second cursor apply");
    assert!(second.commit_allowed());

    let conn = Connection::open(app.db_path()).expect("sqlite");
    let receipt_count: i64 = conn
        .query_row("select count(*) from ingress_receipts", [], |row| {
            row.get(0)
        })
        .expect("receipt count");
    let cursor_count: i64 = conn
        .query_row("select count(*) from source_cursor", [], |row| row.get(0))
        .expect("cursor count");
    assert_eq!(receipt_count, 2);
    assert_eq!(cursor_count, 1);
}

#[tokio::test]
async fn cursor_plan_failure_after_raw_apply_blocks_commit() {
    let temp = tempdir().expect("tempdir");
    let app = axiomsync_cli::open(temp.path()).expect("app");
    let batch: RelayPacketBatch = fixture_json("relay_packet_batch.json");
    let translation = translate_batch(&batch);
    let mut harness = RelayHarness::new(app.clone(), translation.clone());

    harness.plan_raw().await.expect("raw plan");
    harness.apply_raw().await.expect("raw apply");
    assert!(!harness.commit_allowed());

    let mut broken_cursor = translation.cursor_requests.clone();
    broken_cursor[0].connector.clear();
    assert!(harness.plan_cursors_with(broken_cursor).await.is_err());
    assert!(!harness.commit_allowed());

    let conn = Connection::open(app.db_path()).expect("sqlite");
    let cursor_count: i64 = conn
        .query_row("select count(*) from source_cursor", [], |row| row.get(0))
        .expect("cursor count");
    assert_eq!(cursor_count, 0);
}
