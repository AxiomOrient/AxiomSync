use std::process::Command;
use std::{env, path::PathBuf};

use serde_json::Value;
use tempfile::tempdir;

fn cli_bin_path() -> PathBuf {
    if let Ok(path) = env::var("CARGO_BIN_EXE_axiomsync") {
        return PathBuf::from(path);
    }
    if let Ok(path) = env::var("CARGO_BIN_EXE_axiomsync_cli") {
        return PathBuf::from(path);
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .expect("workspace root");
    let bin_name = if cfg!(windows) {
        "axiomsync.exe"
    } else {
        "axiomsync"
    };
    let fallback = workspace_root.join("target").join("debug").join(bin_name);
    assert!(
        fallback.exists(),
        "axiomsync binary not found at {}",
        fallback.display()
    );
    fallback
}

#[test]
fn queue_status_process_contract_returns_success_with_json_payload() {
    // Pseudocode:
    // Given a fresh root
    // When running `axiomsync queue status`
    // Then process exits with success and emits queue JSON payload.
    let root = tempdir().expect("tempdir");
    let output = Command::new(cli_bin_path())
        .args([
            "--root",
            root.path().to_str().expect("root path"),
            "queue",
            "status",
        ])
        .output()
        .expect("run queue status");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"counts\""));
    assert!(stdout.contains("\"lanes\""));
}

#[test]
fn benchmark_gate_enforce_process_contract_returns_non_zero_on_gate_failure() {
    // Pseudocode:
    // Given no benchmark reports
    // When running `axiomsync benchmark gate --enforce`
    // Then process exits non-zero and exposes enforce failure reason.
    let root = tempdir().expect("tempdir");
    let output = Command::new(cli_bin_path())
        .args([
            "--root",
            root.path().to_str().expect("root path"),
            "benchmark",
            "gate",
            "--enforce",
        ])
        .output()
        .expect("run benchmark gate enforce");

    assert!(
        !output.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("benchmark gate failed"));
}

#[test]
fn clean_root_release_verify_process_contract_covers_archive_flow() {
    let root = tempdir().expect("tempdir");
    let source = root.path().join("oauth.md");
    std::fs::write(
        &source,
        "# OAuth\n\nrefresh token rotation and jwks recovery runbook",
    )
    .expect("write source");

    let init = Command::new(cli_bin_path())
        .args(["--root", root.path().to_str().expect("root path"), "init"])
        .output()
        .expect("run init");
    assert!(
        init.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&init.stderr)
    );

    let add = Command::new(cli_bin_path())
        .args([
            "--root",
            root.path().to_str().expect("root path"),
            "add",
            source.to_str().expect("source path"),
            "--target",
            "axiom://resources/clean-root-demo",
        ])
        .output()
        .expect("run add");
    assert!(
        add.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&add.stderr)
    );

    let search = Command::new(cli_bin_path())
        .args([
            "--root",
            root.path().to_str().expect("root path"),
            "search",
            "refresh token rotation",
            "--target",
            "axiom://resources/clean-root-demo",
        ])
        .output()
        .expect("run search");
    assert!(
        search.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&search.stderr)
    );
    let search_json: Value = serde_json::from_slice(&search.stdout).expect("search json");
    assert!(search_json["query_results"].as_array().is_some());

    let event_add = Command::new(cli_bin_path())
        .args([
            "--root",
            root.path().to_str().expect("root path"),
            "event",
            "add",
            "--event-id",
            "log-1",
            "--uri",
            "axiom://events/acme/logs/1",
            "--namespace",
            "acme/platform",
            "--kind",
            "log",
            "--event-time",
            "1710000000",
            "--title",
            "OAuth hot log",
            "--summary",
            "refresh retries detected",
        ])
        .output()
        .expect("run event add");
    assert!(
        event_add.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&event_add.stderr)
    );

    let archive_plan = Command::new(cli_bin_path())
        .args([
            "--root",
            root.path().to_str().expect("root path"),
            "event",
            "archive",
            "plan",
            "--archive-id",
            "clean-root-log-archive",
            "--namespace",
            "acme",
            "--kind",
            "log",
            "--start-time",
            "1709999999",
            "--end-time",
            "1710000100",
        ])
        .output()
        .expect("run archive plan");
    assert!(
        archive_plan.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&archive_plan.stderr)
    );
    let archive_plan_json: Value =
        serde_json::from_slice(&archive_plan.stdout).expect("archive plan json");
    let plan_file = root.path().join("archive-plan.json");
    std::fs::write(
        &plan_file,
        serde_json::to_vec_pretty(&archive_plan_json).expect("serialize plan"),
    )
    .expect("write plan file");

    let archive_execute = Command::new(cli_bin_path())
        .args([
            "--root",
            root.path().to_str().expect("root path"),
            "event",
            "archive",
            "execute",
            "--plan-file",
            plan_file.to_str().expect("plan file"),
        ])
        .output()
        .expect("run archive execute");
    assert!(
        archive_execute.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&archive_execute.stderr)
    );
    let archive_execute_json: Value =
        serde_json::from_slice(&archive_execute.stdout).expect("archive execute json");
    assert_eq!(archive_execute_json["archive_id"], "clean-root-log-archive");

    let doctor_storage = Command::new(cli_bin_path())
        .args([
            "--root",
            root.path().to_str().expect("root path"),
            "doctor",
            "storage",
            "--json",
        ])
        .output()
        .expect("run doctor storage");
    assert!(
        doctor_storage.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&doctor_storage.stderr)
    );
    let doctor_storage_json: Value =
        serde_json::from_slice(&doctor_storage.stdout).expect("doctor storage json");
    assert!(doctor_storage_json["search_document_count"].is_u64());

    let doctor_retrieval = Command::new(cli_bin_path())
        .args([
            "--root",
            root.path().to_str().expect("root path"),
            "doctor",
            "retrieval",
            "--json",
        ])
        .output()
        .expect("run doctor retrieval");
    assert!(
        doctor_retrieval.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&doctor_retrieval.stderr)
    );
    let doctor_retrieval_json: Value =
        serde_json::from_slice(&doctor_retrieval.stdout).expect("doctor retrieval json");
    assert!(doctor_retrieval_json["fts_ready"].is_boolean());

    let migrate_inspect = Command::new(cli_bin_path())
        .args([
            "--root",
            root.path().to_str().expect("root path"),
            "migrate",
            "inspect",
            "--json",
        ])
        .output()
        .expect("run migrate inspect");
    assert!(
        migrate_inspect.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&migrate_inspect.stderr)
    );
    let backup_dir = root.path().join("migration-backups");
    let migrate_apply = Command::new(cli_bin_path())
        .args([
            "--root",
            root.path().to_str().expect("root path"),
            "migrate",
            "apply",
            "--backup-dir",
            backup_dir.to_str().expect("backup dir"),
            "--json",
        ])
        .output()
        .expect("run migrate apply");
    assert!(
        migrate_apply.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&migrate_apply.stderr)
    );
    let migrate_apply_json: Value =
        serde_json::from_slice(&migrate_apply.stdout).expect("migrate apply json");
    assert!(migrate_apply_json["applied_run"]["finished_at"].is_string());
    assert!(backup_dir.exists());

    let release_verify = Command::new(cli_bin_path())
        .args([
            "--root",
            root.path().to_str().expect("root path"),
            "release",
            "verify",
            "--json",
        ])
        .output()
        .expect("run release verify");
    assert!(
        release_verify.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&release_verify.stderr)
    );
    let verify_json: Value =
        serde_json::from_slice(&release_verify.stdout).expect("release verify json");
    assert!(verify_json["storage"]["context_schema_version"].is_string());
    assert!(verify_json["storage"]["search_docs_fts_schema_version"].is_string());
    assert!(verify_json["storage"]["release_contract_version"].is_string());
    assert!(verify_json["retrieval"]["fts_ready"].is_boolean());
}
