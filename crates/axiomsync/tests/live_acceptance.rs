use std::fs;
use std::sync::Arc;

use axiomsync::connectors::ConnectorAdapter;
use axiomsync::domain::{EpisodeExtraction, VerificationExtraction};
use axiomsync::kernel::AxiomSync;
use axiomsync::llm::MockLlmClient;
use axiomsync::mcp;
use axiomsync::ports::ConnectorPort;
use serde_json::json;
use tempfile::tempdir;

fn live_app(connectors_toml: Option<&str>) -> AxiomSync {
    let temp = tempdir().expect("tempdir");
    let root = temp.keep();
    if let Some(connectors_toml) = connectors_toml {
        fs::write(root.join("connectors.toml"), connectors_toml).expect("write connectors config");
    }
    axiomsync::open_with_llm(
        root,
        Arc::new(MockLlmClient {
            extraction: EpisodeExtraction {
                problem: "live connector episode".to_string(),
                ..EpisodeExtraction::default()
            },
            verifications: vec![VerificationExtraction::default()],
        }),
    )
    .expect("app")
}

fn assert_live_output(app: &AxiomSync) {
    let replay = app.plan_replay().expect("plan replay");
    app.apply_replay(&replay).expect("apply replay");
    let runbooks = app.list_runbooks().expect("runbooks");
    assert!(!runbooks.is_empty(), "live ingest should produce runbooks");

    let workspace_id = runbooks[0].workspace_id.clone().expect("workspace id");
    let roots = mcp::handle_request(
        app,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "roots/list"
        }),
        Some(&workspace_id),
    )
    .expect("roots");
    assert!(
        roots["result"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty()),
        "live run should expose scoped roots"
    );
}

async fn run_live_payload_connector_smoke(connector: &str, env_key: &str) {
    let Ok(path) = std::env::var(env_key) else {
        eprintln!("skipping {connector} live smoke; {env_key} not set");
        return;
    };
    let app = live_app(None);
    let adapter = ConnectorAdapter::from_connector_label(connector);
    let batch = adapter
        .load_batch(Some(std::path::Path::new(&path)), None, None, None)
        .expect("connector batch");
    let plan = app.plan_ingest(&batch).expect("plan ingest");
    app.apply_ingest(&plan).expect("apply ingest");
    assert_live_output(&app);
}

async fn run_live_codex_sync_smoke() {
    let Ok(base_url) = std::env::var("AXIOMSYNC_LIVE_CODEX_APP_SERVER_BASE_URL") else {
        eprintln!("skipping codex live smoke; AXIOMSYNC_LIVE_CODEX_APP_SERVER_BASE_URL not set");
        return;
    };
    let api_key = std::env::var("AXIOMSYNC_LIVE_CODEX_API_KEY").ok();
    let connectors_toml = match api_key {
        Some(api_key) => format!(
            r#"[codex]
enabled = true
app_server_base_url = "{}"
api_key = "{}"
"#,
            base_url, api_key
        ),
        None => format!(
            r#"[codex]
enabled = true
app_server_base_url = "{}"
"#,
            base_url
        ),
    };
    let app = live_app(Some(&connectors_toml));
    let adapter = ConnectorAdapter::Codex;
    let value = adapter
        .load_sync_config_value(&app)
        .expect("codex sync payload");
    let batch = adapter.parse_batch(value).expect("codex batch");
    let plan = app.plan_ingest(&batch).expect("plan ingest");
    app.apply_ingest(&plan).expect("apply ingest");
    assert_live_output(&app);
}

async fn run_live_gemini_directory_smoke() {
    let Ok(dir) = std::env::var("AXIOMSYNC_LIVE_GEMINI_DIR") else {
        eprintln!("skipping gemini live smoke; AXIOMSYNC_LIVE_GEMINI_DIR not set");
        return;
    };
    let app = live_app(None);
    let adapter = ConnectorAdapter::GeminiCli;
    let batch = adapter
        .load_dir_batch(std::path::Path::new(&dir))
        .expect("gemini dir batch");
    let plan = app.plan_ingest(&batch).expect("plan ingest");
    app.apply_ingest(&plan).expect("apply ingest");
    assert_live_output(&app);
}

#[tokio::test]
#[ignore = "opt-in live connector proof: set AXIOMSYNC_LIVE_CHATGPT_PAYLOAD"]
async fn live_chatgpt_ingest_replay_and_mcp_smoke() {
    run_live_payload_connector_smoke("chatgpt", "AXIOMSYNC_LIVE_CHATGPT_PAYLOAD").await;
}

#[tokio::test]
#[ignore = "opt-in live connector proof: set AXIOMSYNC_LIVE_CODEX_APP_SERVER_BASE_URL"]
async fn live_codex_ingest_replay_and_mcp_smoke() {
    run_live_codex_sync_smoke().await;
}

#[tokio::test]
#[ignore = "opt-in live connector proof: set AXIOMSYNC_LIVE_CLAUDE_CODE_PAYLOAD"]
async fn live_claude_code_ingest_replay_and_mcp_smoke() {
    run_live_payload_connector_smoke("claude_code", "AXIOMSYNC_LIVE_CLAUDE_CODE_PAYLOAD").await;
}

#[tokio::test]
#[ignore = "opt-in live connector proof: set AXIOMSYNC_LIVE_GEMINI_DIR"]
async fn live_gemini_cli_ingest_replay_and_mcp_smoke() {
    run_live_gemini_directory_smoke().await;
}
