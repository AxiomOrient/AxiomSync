use std::fs;
use std::process::Command;
use std::{env, path::PathBuf};

use serde_json::{Value, json};
use tempfile::tempdir;

fn cli_bin_path() -> PathBuf {
    if let Ok(path) = env::var("CARGO_BIN_EXE_axiomsync") {
        return PathBuf::from(path);
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .join("target")
        .join("debug")
        .join(if cfg!(windows) {
            "axiomsync.exe"
        } else {
            "axiomsync"
        })
}

#[test]
fn cli_help_exposes_renewal_commands() {
    let output = Command::new(cli_bin_path())
        .arg("--help")
        .output()
        .expect("help");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for command in [
        "sink", "project", "derive", "search", "runbook", "mcp", "web",
    ] {
        assert!(stdout.contains(command), "{command}");
    }
    assert!(!stdout.contains("connector"), "{stdout}");
}

#[test]
fn sink_help_exposes_file_driven_boundary() {
    let sink_output = Command::new(cli_bin_path())
        .args(["sink", "--help"])
        .output()
        .expect("sink help");
    assert!(sink_output.status.success());
    let sink_stdout = String::from_utf8_lossy(&sink_output.stdout);
    assert!(sink_stdout.contains("raw-only"), "{sink_stdout}");
    assert!(
        sink_stdout.contains("plan-append-raw-events"),
        "{sink_stdout}"
    );
    assert!(sink_stdout.contains("apply-ingest-plan"), "{sink_stdout}");
    assert!(
        sink_stdout.contains("plan-upsert-source-cursor"),
        "{sink_stdout}"
    );
    assert!(
        sink_stdout.contains("apply-source-cursor-plan"),
        "{sink_stdout}"
    );
    assert!(!sink_stdout.contains("serve"), "{sink_stdout}");
    assert!(!sink_stdout.contains("health"), "{sink_stdout}");

    let project_output = Command::new(cli_bin_path())
        .args(["project", "--help"])
        .output()
        .expect("project help");
    assert!(project_output.status.success());
    let project_stdout = String::from_utf8_lossy(&project_output.stdout);
    assert!(
        project_stdout.contains("plan-admin-grant"),
        "{project_stdout}"
    );
    assert!(
        project_stdout.contains("apply-admin-grant-plan"),
        "{project_stdout}"
    );
}

#[test]
fn cli_real_use_flow_handles_mixed_records_end_to_end() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path();
    let fixtures_dir = root.join("fixtures");
    let plans_dir = root.join("plans");
    fs::create_dir_all(&fixtures_dir).expect("fixtures");
    fs::create_dir_all(&plans_dir).expect("plans");

    let init_output = Command::new(cli_bin_path())
        .args(["--root", root.to_str().expect("root"), "init"])
        .output()
        .expect("init");
    assert!(init_output.status.success(), "{init_output:?}");

    let raw_events_path = fixtures_dir.join("raw-events.json");
    fs::write(
        &raw_events_path,
        serde_json::to_vec_pretty(&json!({
            "request_id": "req-1",
            "events": [
                {
                    "source": "codex",
                    "native_schema_version": "agent-record-v1",
                    "native_session_id": "thread-session",
                    "native_event_id": "evt-1",
                    "event_type": "user_message",
                    "ts_ms": 1_710_000_000_000u64,
                    "payload": {
                        "workspace_root": "/repo/app",
                        "turn_id": "turn-1",
                        "actor": "user",
                        "text": "Investigate timeout error"
                    }
                },
                {
                    "source": "codex",
                    "native_schema_version": "agent-record-v1",
                    "native_session_id": "runtime-session",
                    "native_event_id": "evt-2",
                    "event_type": "task_state",
                    "ts_ms": 1_710_000_001_000u64,
                    "payload": {
                        "workspace_root": "/repo/app",
                        "record_type": "task_state",
                        "subject": {"kind": "task", "id": "task-1", "parent_id": "run-1"},
                        "runtime": {"run_id": "run-1", "task_id": "task-1", "role": "do", "status": "running"},
                        "task": {"title": "Investigate timeout"},
                        "body": {"text": "Task is running"}
                    }
                }
            ]
        }))
        .expect("raw events"),
    )
    .expect("write raw events");

    let document_events_path = fixtures_dir.join("document-events.json");
    fs::write(
        &document_events_path,
        serde_json::to_vec_pretty(&json!({
            "request_id": "req-2",
            "events": [
                {
                    "source": "gemini_cli",
                    "native_schema_version": "agent-record-v1",
                    "native_session_id": "docs-session",
                    "native_event_id": "evt-3",
                    "event_type": "document_snapshot",
                    "ts_ms": 1_710_000_003_000u64,
                    "payload": {
                        "workspace_root": "/repo/app",
                        "record_type": "document_snapshot",
                        "subject": {"kind": "document", "id": "mission", "path": "program/MISSION.md"},
                        "document": {"kind": "mission", "path": "program/MISSION.md", "title": "Mission"},
                        "body": {"text": "Stabilize timeout handling"}
                    }
                }
            ]
        }))
        .expect("document events"),
    )
    .expect("write document events");

    let cursor_path = fixtures_dir.join("cursor.json");
    fs::write(
        &cursor_path,
        serde_json::to_vec_pretty(&json!({
            "source": "codex",
            "cursor": {
                "cursor_key": "events",
                "cursor_value": "cursor-1",
                "updated_at_ms": 1_710_000_002_000u64
            }
        }))
        .expect("cursor json"),
    )
    .expect("write cursor");

    let ingest_plan_path = plans_dir.join("ingest-plan.json");
    let ingest_plan = Command::new(cli_bin_path())
        .args([
            "--root",
            root.to_str().expect("root"),
            "sink",
            "plan-append-raw-events",
            "--file",
            raw_events_path.to_str().expect("raw path"),
        ])
        .output()
        .expect("plan ingest");
    assert!(ingest_plan.status.success(), "{ingest_plan:?}");
    fs::write(&ingest_plan_path, &ingest_plan.stdout).expect("write ingest plan");
    let ingest_plan_value: Value =
        serde_json::from_slice(&ingest_plan.stdout).expect("ingest plan json");
    assert_eq!(ingest_plan_value["adds"].as_array().map(Vec::len), Some(2));

    let ingest_apply = Command::new(cli_bin_path())
        .args([
            "--root",
            root.to_str().expect("root"),
            "sink",
            "apply-ingest-plan",
            "--file",
            ingest_plan_path.to_str().expect("ingest plan path"),
        ])
        .output()
        .expect("apply ingest");
    assert!(ingest_apply.status.success(), "{ingest_apply:?}");

    let cursor_plan_path = plans_dir.join("cursor-plan.json");
    let cursor_plan = Command::new(cli_bin_path())
        .args([
            "--root",
            root.to_str().expect("root"),
            "sink",
            "plan-upsert-source-cursor",
            "--file",
            cursor_path.to_str().expect("cursor path"),
        ])
        .output()
        .expect("plan cursor");
    assert!(cursor_plan.status.success(), "{cursor_plan:?}");
    fs::write(&cursor_plan_path, &cursor_plan.stdout).expect("write cursor plan");

    let cursor_apply = Command::new(cli_bin_path())
        .args([
            "--root",
            root.to_str().expect("root"),
            "sink",
            "apply-source-cursor-plan",
            "--file",
            cursor_plan_path.to_str().expect("cursor plan path"),
        ])
        .output()
        .expect("apply cursor");
    assert!(cursor_apply.status.success(), "{cursor_apply:?}");

    let document_ingest_plan_path = plans_dir.join("document-ingest-plan.json");
    let document_ingest_plan = Command::new(cli_bin_path())
        .args([
            "--root",
            root.to_str().expect("root"),
            "sink",
            "plan-append-raw-events",
            "--file",
            document_events_path.to_str().expect("document path"),
        ])
        .output()
        .expect("plan document ingest");
    assert!(
        document_ingest_plan.status.success(),
        "{document_ingest_plan:?}"
    );
    fs::write(&document_ingest_plan_path, &document_ingest_plan.stdout)
        .expect("write document ingest plan");

    let document_ingest_apply = Command::new(cli_bin_path())
        .args([
            "--root",
            root.to_str().expect("root"),
            "sink",
            "apply-ingest-plan",
            "--file",
            document_ingest_plan_path
                .to_str()
                .expect("document ingest plan path"),
        ])
        .output()
        .expect("apply document ingest");
    assert!(
        document_ingest_apply.status.success(),
        "{document_ingest_apply:?}"
    );

    let replay_plan_path = plans_dir.join("replay-plan.json");
    let replay_plan = Command::new(cli_bin_path())
        .args([
            "--root",
            root.to_str().expect("root"),
            "project",
            "plan-rebuild",
        ])
        .output()
        .expect("plan rebuild");
    assert!(replay_plan.status.success(), "{replay_plan:?}");
    fs::write(&replay_plan_path, &replay_plan.stdout).expect("write replay plan");
    let replay_plan_value: Value =
        serde_json::from_slice(&replay_plan.stdout).expect("replay plan json");
    assert_eq!(
        replay_plan_value["projection"]["document_records"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        replay_plan_value["projection"]["execution_runs"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );

    let replay_apply = Command::new(cli_bin_path())
        .args([
            "--root",
            root.to_str().expect("root"),
            "project",
            "apply-replay-plan",
            "--file",
            replay_plan_path.to_str().expect("replay plan path"),
        ])
        .output()
        .expect("apply replay");
    assert!(replay_apply.status.success(), "{replay_apply:?}");

    let search_output = Command::new(cli_bin_path())
        .args(["--root", root.to_str().expect("root"), "search", "timeout"])
        .output()
        .expect("search");
    assert!(search_output.status.success(), "{search_output:?}");
    let search_value: Value = serde_json::from_slice(&search_output.stdout).expect("search json");
    assert_eq!(search_value.as_array().map(Vec::len), Some(1));
    assert_eq!(search_value[0]["source"].as_str(), Some("codex"));
    assert_eq!(
        search_value[0]["problem"].as_str(),
        Some("user: Investigate timeout error")
    );
}
