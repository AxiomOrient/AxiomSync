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
        root.join("docs/INDEX.md"),
        root.join("docs/API_CONTRACT.md"),
        root.join("docs/KERNEL_SINK_CONTRACT.md"),
        root.join("docs/RUNTIME_ARCHITECTURE.md"),
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
        root.join("docs/INDEX.md"),
        root.join("docs/API_CONTRACT.md"),
        root.join("docs/TESTING.md"),
        root.join("docs/RELEASE_RUNBOOK.md"),
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
