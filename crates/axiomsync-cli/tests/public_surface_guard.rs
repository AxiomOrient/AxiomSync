use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn read(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

fn join(parts: &[&str]) -> String {
    parts.concat()
}

#[test]
fn public_surface_excludes_legacy_terms() {
    let root = repo_root();
    let files = [
        root.join("README.md"),
        root.join("docs/API_CONTRACT.md"),
        root.join("docs/KERNEL_SINK_CONTRACT.md"),
        root.join("docs/RELAY_INTEROP.md"),
        root.join("docs/RUNTIME_ARCHITECTURE.md"),
        root.join("docs/TESTING.md"),
        root.join("docs/RELEASE_RUNBOOK.md"),
        root.join("crates/axiomsync-cli/src/lib.rs"),
        root.join("crates/axiomsync-http/src/lib.rs"),
        root.join("crates/axiomsync-kernel/src/mcp.rs"),
    ];

    let forbidden = [
        join(&["request", "_id"]),
        join(&["connector", "_name"]),
        join(&["session", "://"]),
        join(&["axiom://", "sessions/"]),
        join(&["/api/", "sessions/"]),
        join(&["/api/", "episodes/"]),
        join(&["/api/", "runbooks/"]),
        ".route(\"/cases/{id}\"".to_string(),
        join(&["search", "-", "episodes"]),
        "SearchEpisodesRequest".to_string(),
        "id_arg_alias(".to_string(),
        join(&["find", "-", "runbook"]),
        join(&["compat", " get-"]),
        "final-form".to_string(),
        "final_form_compat".to_string(),
        "web --help".to_string(),
        join(&["conv", "_session"]),
        join(&["search", "_doc_redacted"]),
        join(&["import", "_journal"]),
    ];

    for path in files {
        let body = read(&path);
        for needle in &forbidden {
            assert!(
                !body.contains(needle.as_str()),
                "unexpected legacy token `{needle}` in {}",
                path.display()
            );
        }
    }
}

#[test]
fn release_docs_do_not_reference_deleted_packages() {
    let root = repo_root();
    let files = [
        root.join("README.md"),
        root.join("docs/API_CONTRACT.md"),
        root.join("docs/TESTING.md"),
        root.join("docs/RELEASE_RUNBOOK.md"),
        root.join("docs/RELAY_INTEROP.md"),
    ];

    for path in files {
        let body = read(&path);
        for needle in [
            join(&["axiomsync-", "final-form-docs-package"]),
            join(&["axiomsync-", "b8e8828-audit-patch-package"]),
        ] {
            assert!(
                !body.contains(needle.as_str()),
                "unexpected deleted package reference `{needle}` in {}",
                path.display()
            );
        }
    }
}

#[test]
fn api_contract_lists_current_canonical_routes_and_commands() {
    let root = repo_root();
    let api_contract = read(&root.join("docs/API_CONTRACT.md"));
    let readme = read(&root.join("README.md"));
    let sink_contract = read(&root.join("docs/KERNEL_SINK_CONTRACT.md"));
    let testing = read(&root.join("docs/TESTING.md"));
    let release_runbook = read(&root.join("docs/RELEASE_RUNBOOK.md"));

    for route in [
        "POST /sink/raw-events/plan",
        "POST /sink/raw-events/apply",
        "POST /sink/source-cursors/plan",
        "POST /sink/source-cursors/apply",
        "POST /admin/projection/plan",
        "POST /admin/projection/apply",
        "POST /admin/derivations/plan",
        "POST /admin/derivations/apply",
        "POST /admin/replay/plan",
        "POST /admin/replay/apply",
        "GET /api/cases/{id}",
        "GET /api/threads/{id}",
        "GET /api/runs",
        "GET /api/runs/{id}",
        "GET /api/tasks/{id}",
        "GET /api/documents/{id}",
        "GET /api/evidence/{id}",
        "POST /api/query/search-cases",
        "POST /mcp",
    ] {
        assert!(
            api_contract.contains(route),
            "missing route `{route}` from API contract"
        );
    }

    for command in [
        "axiomsync-cli sink plan-append-raw-events",
        "axiomsync-cli sink apply-ingest-plan",
        "axiomsync-cli sink plan-upsert-source-cursor",
        "axiomsync-cli sink apply-source-cursor-plan",
        "axiomsync-cli project plan-rebuild",
        "axiomsync-cli project apply-replay-plan",
        "axiomsync-cli query search-cases",
        "axiomsync-cli mcp serve",
        "axiomsync-cli serve",
    ] {
        assert!(
            api_contract.contains(command),
            "missing command `{command}` from API contract"
        );
    }

    for body in [&readme, &api_contract, &sink_contract] {
        assert!(
            body.contains("RELAY_INTEROP.md"),
            "missing relay interop contract reference"
        );
    }

    for path in [
        "crates/axiomsync-cli/tests/replay_pipeline.rs",
        "crates/axiomsync-cli/tests/sink_contract.rs",
        "crates/axiomsync-cli/tests/http_and_mcp.rs",
        "crates/axiomsync-cli/tests/relay_interop.rs",
    ] {
        assert!(
            testing.contains(path),
            "missing test path `{path}` from TESTING.md"
        );
    }

    for body in [&testing, &release_runbook] {
        assert!(
            body.contains("./scripts/verify-release.sh"),
            "missing one-shot verification script reference"
        );
        assert!(
            body.contains(
                "cargo test -p axiomsync-cli --test relay_interop relay_http_delivery_smoke_commits_only_after_both_apply_phases -- --nocapture"
            ),
            "missing explicit relay interop smoke gate"
        );
    }
}

#[test]
fn changelog_contains_current_workspace_version_entry() {
    let root = repo_root();
    let cargo = read(&root.join("Cargo.toml"));
    let changelog = read(&root.join("CHANGELOG.md"));

    let workspace_version = cargo
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix("version = \"")
                .and_then(|value| value.strip_suffix('"'))
        })
        .expect("workspace version");

    assert!(
        changelog.contains(&format!("## v{workspace_version} -")),
        "missing changelog entry for v{workspace_version}"
    );
}
