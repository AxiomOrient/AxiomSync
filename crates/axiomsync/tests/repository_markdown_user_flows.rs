use std::fs;
use std::path::{Path, PathBuf};

use axiomsync::AxiomSync;
use axiomsync::models::{AddResourceIngestOptions, AddResourceRequest, SearchRequest};
use tempfile::tempdir;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .map(PathBuf::from)
        .expect("workspace root")
}

fn real_markdown_fixtures() -> Vec<&'static str> {
    vec![
        "README.md",
        "CHANGELOG.md",
        "docs/API_CONTRACT.md",
        "docs/RELEASE_RUNBOOK.md",
        "docs/RETRIEVAL_ARCHITECTURE.md",
        "docs/IMPLEMENTATION_SPEC.md",
        "docs/INDEX.md",
    ]
}

fn stage_real_markdown_set(staging_root: &Path) -> PathBuf {
    let workspace_root = workspace_root();
    for relative in real_markdown_fixtures() {
        let source = workspace_root.join(relative);
        let target = staging_root.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).expect("create staged parent");
        }
        fs::copy(&source, &target).expect("copy markdown fixture");
    }
    staging_root.to_path_buf()
}

fn ingest_real_markdown_set(app: &AxiomSync, source_dir: &Path) {
    let mut request = AddResourceRequest::new(source_dir.to_str().expect("source dir").to_string());
    request.target = Some("axiom://resources/repository-markdown".to_string());
    request.wait = true;
    request.ingest_options = AddResourceIngestOptions::markdown_only_defaults();
    app.add_resource_with_ingest_options(request)
        .expect("ingest real markdown set");
    app.wait_processed(Some(30)).expect("wait processed");
}

#[test]
fn repository_markdown_find_queries_hit_real_release_docs() {
    let temp = tempdir().expect("tempdir");
    let staging = tempdir().expect("staging tempdir");
    let staged_source = stage_real_markdown_set(staging.path());
    let app = AxiomSync::new(temp.path()).expect("app");
    app.initialize().expect("init");
    ingest_real_markdown_set(&app, &staged_source);

    let verify_query = app
        .find(
            "release verify --json",
            Some("axiom://resources/repository-markdown"),
            Some(5),
            None,
            None,
        )
        .expect("find release verify");
    assert!(
        verify_query
            .query_results
            .iter()
            .any(|hit| hit.uri.ends_with("/docs/RELEASE_RUNBOOK.md")
                || hit.uri.ends_with("/docs/API_CONTRACT.md"))
    );

    let migrate_query = app
        .find(
            "migrate apply --backup-dir",
            Some("axiom://resources/repository-markdown"),
            Some(5),
            None,
            None,
        )
        .expect("find migrate apply");
    assert!(migrate_query.query_results.iter().any(
        |hit| hit.uri.ends_with("/docs/API_CONTRACT.md")
            || hit.uri.ends_with("/docs/IMPLEMENTATION_SPEC.md")
            || hit.uri.ends_with("/docs/INDEX.md")
    ));
}

#[test]
fn repository_markdown_session_search_behaves_like_release_operator_workflow() {
    let temp = tempdir().expect("tempdir");
    let staging = tempdir().expect("staging tempdir");
    let staged_source = stage_real_markdown_set(staging.path());
    let app = AxiomSync::new(temp.path()).expect("app");
    app.initialize().expect("init");
    ingest_real_markdown_set(&app, &staged_source);

    let session = app.session(Some("release-operator"));
    session.load().expect("session load");
    session
        .add_message(
            "user",
            "doctor retrieval --json, release verify --json, fts_fallback_used 확인 필요",
        )
        .expect("session message");

    let result = app
        .search_with_request(SearchRequest {
            query: "release verify json fts_fallback_used".to_string(),
            target_uri: Some("axiom://resources/repository-markdown".to_string()),
            session: Some("release-operator".to_string()),
            limit: Some(8),
            score_threshold: None,
            min_match_tokens: None,
            filter: None,
            budget: None,
            runtime_hints: Vec::new(),
        })
        .expect("search with request");

    assert!(
        result
            .trace
            .as_ref()
            .expect("trace")
            .scope_decision
            .mixed_intent
    );
    assert!(
        result
            .query_results
            .iter()
            .any(|hit| hit.uri.ends_with("/docs/RELEASE_RUNBOOK.md")
                || hit.uri.ends_with("/docs/API_CONTRACT.md"))
    );
}
