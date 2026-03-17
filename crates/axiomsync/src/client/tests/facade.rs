use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use tempfile::tempdir;

use crate::models::{
    AddEventRequest, Kind, LinkRequest, MetadataFilter, NamespaceKey, RepoMountRequest,
};
use crate::{AxiomSync, AxiomUri};

#[test]
fn add_event_persists_event_and_projection() {
    let temp = tempdir().expect("tempdir");
    let app = AxiomSync::new(temp.path()).expect("app");
    app.initialize().expect("init");

    let event = app
        .add_event(AddEventRequest {
            event_id: "evt-1".to_string(),
            uri: AxiomUri::parse("axiom://events/acme/incidents/1").expect("uri"),
            namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
            kind: Kind::new("incident").expect("kind"),
            event_time: 1_710_000_000,
            title: Some("OAuth outage".to_string()),
            summary_text: Some("oauth token failures".to_string()),
            severity: Some("high".to_string()),
            actor_uri: None,
            subject_uri: None,
            run_id: Some("run-1".to_string()),
            session_id: None,
            tags: vec!["oauth".to_string()],
            attrs: serde_json::json!({"env": "prod"}),
            object_uri: None,
            content_hash: Some("hash-1".to_string()),
            created_at: Some(1_710_000_001),
        })
        .expect("add event");

    let stored = app
        .state
        .query_events(crate::models::EventQuery {
            namespace_prefix: Some(NamespaceKey::parse("acme").expect("namespace")),
            kind: Some(Kind::new("incident").expect("kind")),
            start_time: None,
            end_time: None,
            limit: Some(10),
            include_tombstoned: false,
        })
        .expect("query events");
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].event_id, event.event_id);

    let docs = app.state.list_search_documents().expect("search docs");
    assert!(docs.iter().any(|doc| doc.uri == event.uri.to_string()));
}

#[test]
fn link_records_persists_global_link() {
    let temp = tempdir().expect("tempdir");
    let app = AxiomSync::new(temp.path()).expect("app");

    let link = app
        .link_records(LinkRequest {
            link_id: "link-1".to_string(),
            namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
            from_uri: AxiomUri::parse("axiom://events/acme/incidents/1").expect("from"),
            relation: "resolved_by".to_string(),
            to_uri: AxiomUri::parse("axiom://resources/acme/runbooks/auth").expect("to"),
            weight: 0.9,
            attrs: serde_json::json!({"source": "incident"}),
            created_at: Some(123),
        })
        .expect("link");

    let stored = app
        .state
        .query_links(crate::models::LinkQuery {
            namespace_prefix: Some(NamespaceKey::parse("acme").expect("namespace")),
            from_uri: Some(link.from_uri.clone()),
            to_uri: None,
            relation: Some("resolved_by".to_string()),
            limit: Some(10),
        })
        .expect("query links");
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].link_id, "link-1");
}

#[test]
fn mount_repo_ingests_and_registers_resource_record() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path().join("repo");
    fs::create_dir_all(&repo).expect("repo dir");
    fs::write(repo.join("README.md"), "# Demo\n\nrepo mount").expect("write readme");

    let app = AxiomSync::new(temp.path().join("runtime")).expect("app");
    app.initialize().expect("init");

    let report = app
        .mount_repo(RepoMountRequest {
            source_path: repo.to_string_lossy().to_string(),
            target_uri: AxiomUri::parse("axiom://resources/acme/repos/demo").expect("uri"),
            namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
            kind: Kind::new("repository").expect("kind"),
            title: Some("Demo Repo".to_string()),
            tags: vec!["repo".to_string()],
            attrs: serde_json::json!({"origin": "test"}),
            wait: false,
        })
        .expect("mount repo");

    assert_eq!(
        report.root_uri.to_string(),
        "axiom://resources/acme/repos/demo"
    );
    let stored = app
        .state
        .get_resource(&report.root_uri)
        .expect("get resource")
        .expect("resource");
    assert_eq!(stored.kind.as_str(), "repository");
    assert_eq!(stored.namespace.as_path(), "acme/platform");
    assert!(stored.object_uri.is_some());
}

#[test]
fn mount_repo_root_projection_namespace_and_kind_survive_reindex_all() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path().join("repo");
    fs::create_dir_all(&repo).expect("repo dir");
    fs::write(
        repo.join("README.md"),
        "# Demo\n\nrepo mount stability test",
    )
    .expect("write");

    let app = AxiomSync::new(temp.path().join("runtime")).expect("app");
    app.initialize().expect("init");

    app.mount_repo(RepoMountRequest {
        source_path: repo.to_string_lossy().to_string(),
        target_uri: AxiomUri::parse("axiom://resources/acme/repos/stable").expect("uri"),
        namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
        kind: Kind::new("repository").expect("kind"),
        title: Some("Stable Repo".to_string()),
        tags: vec!["repo".to_string()],
        attrs: serde_json::json!({}),
        wait: false,
    })
    .expect("mount repo");

    // Global reindex must not overwrite mount-root namespace/kind projection.
    app.reindex_all().expect("reindex all");

    let doc = app
        .state
        .get_search_document("axiom://resources/acme/repos/stable")
        .expect("get search doc")
        .expect("search doc must exist after reindex");

    assert!(
        doc.tags.iter().any(|t| t == "ns:acme/platform"),
        "reindex must preserve ns tag on mount root; got tags: {:?}",
        doc.tags
    );
    assert!(
        doc.tags.iter().any(|t| t == "kind:repository"),
        "reindex must preserve kind tag on mount root; got tags: {:?}",
        doc.tags
    );
}

#[test]
fn mount_repo_realistic_identity_workspace_preserves_rich_resource_metadata_and_searchability() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path().join("identity-control-plane");
    write_fixture(
        &repo.join("README.md"),
        r#"# Identity Control Plane

The identity control plane owns OAuth token minting, JWKS publication, device session recovery,
and emergency client rotation for critical production incidents.
"#,
    );
    write_fixture(
        &repo.join("runbooks/oauth-major-incident.md"),
        r#"# OAuth Major Incident

1. Confirm refresh token failures exceed the paging threshold for five minutes.
2. Enable break-glass client rotation and publish the new JWKS bundle.
3. Drain the auth-worker queue and restart the token exchange deployment.
4. Verify device login recovers before reopening traffic.
"#,
    );
    write_fixture(
        &repo.join("contracts/token-refresh-api.md"),
        r#"# Token Refresh API

`POST /oauth/token` accepts refresh tokens, rotates them on success, and emits an incident event
when refresh token failure rates cross the SLO burn alert.
"#,
    );
    write_fixture(
        &repo.join("postmortems/2026-02-oauth-outage.md"),
        r#"# OAuth February Outage

The outage started after a stale JWKS bundle blocked token verification. Recovery required
break-glass client rotation, auth-worker restart, and replay of stuck device sessions.
"#,
    );
    write_fixture(
        &repo.join("services/auth-worker/src/retry_policy.rs"),
        "pub const RETRY_BUDGET: u32 = 7;\n",
    );

    let app = AxiomSync::new(temp.path().join("runtime")).expect("app");
    app.initialize().expect("init");

    let mount = app
        .mount_repo(RepoMountRequest {
            source_path: repo.to_string_lossy().to_string(),
            target_uri: AxiomUri::parse("axiom://resources/acme/repos/identity-control-plane")
                .expect("uri"),
            namespace: NamespaceKey::parse("acme/identity").expect("namespace"),
            kind: Kind::new("repository").expect("kind"),
            title: Some("Identity Control Plane".to_string()),
            tags: vec![
                "Identity".to_string(),
                "Critical Path".to_string(),
                "Runbook".to_string(),
            ],
            attrs: serde_json::json!({
                "owner_team": "identity-platform",
                "tier": "critical",
                "compliance": ["soc2", "gdpr"],
                "services": ["auth-worker", "token-exchange", "jwks-publisher"],
            }),
            wait: true,
        })
        .expect("mount repo");

    let stored = app
        .state
        .get_resource(&mount.root_uri)
        .expect("get resource")
        .expect("resource");
    assert_eq!(stored.uri, mount.root_uri);
    assert_eq!(stored.namespace.as_path(), "acme/identity");
    assert_eq!(stored.kind.as_str(), "repository");
    assert_eq!(stored.title.as_deref(), Some("Identity Control Plane"));
    assert_eq!(
        stored.tags,
        vec![
            "critical path".to_string(),
            "identity".to_string(),
            "runbook".to_string()
        ]
    );
    assert_eq!(
        stored.attrs,
        serde_json::json!({
            "owner_team": "identity-platform",
            "tier": "critical",
            "compliance": ["soc2", "gdpr"],
            "services": ["auth-worker", "token-exchange", "jwks-publisher"],
        })
    );
    assert!(stored.mime.is_none());
    assert_eq!(
        stored.excerpt_text.as_deref(),
        Some("Identity Control Plane")
    );
    assert!(stored.object_uri.is_some());
    assert!(stored.tombstoned_at.is_none());
    assert!(stored.created_at > 0);
    assert!(stored.updated_at >= stored.created_at);
    assert_eq!(
        stored.content_hash.len(),
        64,
        "content_hash must be a 32-byte hex blake3 digest"
    );
    assert_ne!(
        stored.content_hash,
        blake3::hash(repo.to_string_lossy().as_bytes())
            .to_hex()
            .to_string(),
        "content_hash must be derived from tree content, not the path string"
    );

    let mount_object_uri = stored.object_uri.clone().expect("mount object uri");
    let mount_object: serde_json::Value = serde_json::from_slice(
        &app.fs
            .read_bytes(&mount_object_uri)
            .expect("read mount object bytes"),
    )
    .expect("mount object json");
    assert_eq!(
        mount_object,
        serde_json::json!({
            "source_path": repo.to_string_lossy().to_string(),
            "target_uri": "axiom://resources/acme/repos/identity-control-plane",
            "namespace": "acme/identity",
            "kind": "repository",
            "title": "Identity Control Plane",
            "tags": ["Identity", "Critical Path", "Runbook"],
            "attrs": {
                "owner_team": "identity-platform",
                "tier": "critical",
                "compliance": ["soc2", "gdpr"],
                "services": ["auth-worker", "token-exchange", "jwks-publisher"]
            }
        })
    );

    let listed = app
        .state
        .list_resources(crate::models::ResourceQuery {
            namespace_prefix: Some(NamespaceKey::parse("acme").expect("namespace")),
            kind: Some(Kind::new("repository").expect("kind")),
            limit: Some(10),
            include_tombstoned: false,
        })
        .expect("list resources");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].resource_id, stored.resource_id);

    let find = app
        .find(
            "break-glass client rotation jwks auth-worker restart",
            Some("axiom://resources/acme/repos/identity-control-plane"),
            Some(10),
            None,
            None,
        )
        .expect("find mounted documents");
    let uris = find
        .query_results
        .iter()
        .map(|hit| hit.uri.as_str())
        .collect::<Vec<_>>();
    assert!(
        uris.iter()
            .any(|uri| uri.ends_with("/runbooks/oauth-major-incident.md"))
    );
    assert!(
        uris.iter()
            .any(|uri| uri.ends_with("/postmortems/2026-02-oauth-outage.md"))
    );
    assert!(
        uris.iter()
            .any(|uri| uri.ends_with("/contracts/token-refresh-api.md"))
    );
}

#[test]
fn add_event_externalizes_large_raw_payload() {
    let temp = tempdir().expect("tempdir");
    let app = AxiomSync::new(temp.path()).expect("app");
    app.initialize().expect("init");

    let payload = "x".repeat(6 * 1024);
    let event = app
        .add_event(AddEventRequest {
            event_id: "evt-ext".to_string(),
            uri: AxiomUri::parse("axiom://events/acme/incidents/ext").expect("uri"),
            namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
            kind: Kind::new("incident").expect("kind"),
            event_time: 1_710_000_100,
            title: Some("Large payload".to_string()),
            summary_text: Some("externalized".to_string()),
            severity: None,
            actor_uri: None,
            subject_uri: None,
            run_id: None,
            session_id: None,
            tags: Vec::new(),
            attrs: serde_json::json!({
                "raw_payload": payload,
                "env": "prod"
            }),
            object_uri: None,
            content_hash: None,
            created_at: Some(1_710_000_101),
        })
        .expect("add event");

    let object_uri = event.object_uri.expect("object uri");
    let raw = app.fs.read_bytes(&object_uri).expect("read object");
    let stored_payload: serde_json::Value = serde_json::from_slice(&raw).expect("json");
    assert!(stored_payload.get("raw_payload").is_some());
    assert!(event.attrs.get("raw_payload").is_none());
}

#[test]
fn execute_event_archive_applies_retention_profile_and_writes_jsonl_bundle() {
    let temp = tempdir().expect("tempdir");
    let app = AxiomSync::new(temp.path()).expect("app");
    app.initialize().expect("init");

    app.add_events(vec![
        AddEventRequest {
            event_id: "log-1".to_string(),
            uri: AxiomUri::parse("axiom://events/acme/logs/1").expect("uri"),
            namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
            kind: Kind::new("log").expect("kind"),
            event_time: 1_710_000_200,
            title: Some("Auth worker log".to_string()),
            summary_text: Some("oauth retry loop detected".to_string()),
            severity: None,
            actor_uri: None,
            subject_uri: None,
            run_id: Some("run-log".to_string()),
            session_id: None,
            tags: vec!["oauth".to_string()],
            attrs: serde_json::json!({"line": "retry"}),
            object_uri: None,
            content_hash: None,
            created_at: Some(1_710_000_201),
        },
        AddEventRequest {
            event_id: "log-2".to_string(),
            uri: AxiomUri::parse("axiom://events/acme/logs/2").expect("uri"),
            namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
            kind: Kind::new("log").expect("kind"),
            event_time: 1_710_000_210,
            title: Some("Auth worker log".to_string()),
            summary_text: Some("oauth retry loop stabilized".to_string()),
            severity: None,
            actor_uri: None,
            subject_uri: None,
            run_id: Some("run-log".to_string()),
            session_id: None,
            tags: vec!["oauth".to_string()],
            attrs: serde_json::json!({"line": "stable"}),
            object_uri: None,
            content_hash: None,
            created_at: Some(1_710_000_211),
        },
    ])
    .expect("add log events");

    let plan = app
        .plan_event_archive(
            "oauth-log-archive",
            crate::models::EventQuery {
                namespace_prefix: Some(NamespaceKey::parse("acme").expect("namespace")),
                kind: Some(Kind::new("log").expect("kind")),
                start_time: Some(1_710_000_000),
                end_time: Some(1_710_000_300),
                limit: Some(10),
                include_tombstoned: false,
            },
            Some("ephemeral log compaction".to_string()),
            Some("test-suite".to_string()),
        )
        .expect("plan event archive");
    let report = app
        .execute_event_archive(plan)
        .expect("execute event archive");

    assert_eq!(report.event_count, 2);
    assert_eq!(report.retention, crate::models::RetentionClass::Ephemeral);
    assert_eq!(
        report.archive_reason.as_deref(),
        Some("ephemeral log compaction")
    );
    assert_eq!(report.archived_by.as_deref(), Some("test-suite"));
    let payload = app
        .fs
        .read(&report.object_uri)
        .expect("read archive payload");
    let lines = payload.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 2);
    assert!(lines.iter().all(|line| line.contains("\"kind\":\"log\"")));
    let compacted = app
        .state
        .query_events(crate::models::EventQuery {
            namespace_prefix: Some(NamespaceKey::parse("acme").expect("namespace")),
            kind: Some(Kind::new("log").expect("kind")),
            start_time: Some(1_710_000_000),
            end_time: Some(1_710_000_300),
            limit: Some(10),
            include_tombstoned: false,
        })
        .expect("query compacted events");
    assert_eq!(compacted.len(), 2);
    assert!(
        compacted
            .iter()
            .all(|event| event.object_uri.as_ref() == Some(&report.object_uri))
    );
    assert!(compacted.iter().all(|event| {
        event
            .attrs
            .get("archived")
            .and_then(|value| value.get("archive_id"))
            .and_then(|value| value.as_str())
            == Some("oauth-log-archive")
    }));
    let docs = app.state.list_search_documents().expect("list docs");
    assert!(
        !docs
            .iter()
            .any(|doc| doc.uri == "axiom://events/acme/logs/1")
    );
    assert!(
        !docs
            .iter()
            .any(|doc| doc.uri == "axiom://events/acme/logs/2")
    );
}

#[test]
fn execute_event_archive_rejects_when_planned_event_set_drifted() {
    let temp = tempdir().expect("tempdir");
    let app = AxiomSync::new(temp.path()).expect("app");
    app.initialize().expect("init");

    app.add_event(AddEventRequest {
        event_id: "log-1".to_string(),
        uri: AxiomUri::parse("axiom://events/acme/logs/1").expect("uri"),
        namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
        kind: Kind::new("log").expect("kind"),
        event_time: 1_710_000_200,
        title: Some("Auth worker log".to_string()),
        summary_text: Some("oauth retry loop detected".to_string()),
        severity: None,
        actor_uri: None,
        subject_uri: None,
        run_id: None,
        session_id: None,
        tags: vec!["oauth".to_string()],
        attrs: serde_json::json!({}),
        object_uri: None,
        content_hash: None,
        created_at: Some(1_710_000_201),
    })
    .expect("add event");

    let plan = app
        .plan_event_archive(
            "oauth-log-archive",
            crate::models::EventQuery {
                namespace_prefix: Some(NamespaceKey::parse("acme").expect("namespace")),
                kind: Some(Kind::new("log").expect("kind")),
                start_time: Some(1_710_000_000),
                end_time: Some(1_710_000_300),
                limit: Some(10),
                include_tombstoned: false,
            },
            None,
            None,
        )
        .expect("plan event archive");

    app.add_event(AddEventRequest {
        event_id: "log-2".to_string(),
        uri: AxiomUri::parse("axiom://events/acme/logs/2").expect("uri"),
        namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
        kind: Kind::new("log").expect("kind"),
        event_time: 1_710_000_210,
        title: Some("Late auth worker log".to_string()),
        summary_text: Some("unexpected extra event".to_string()),
        severity: None,
        actor_uri: None,
        subject_uri: None,
        run_id: None,
        session_id: None,
        tags: vec!["oauth".to_string()],
        attrs: serde_json::json!({}),
        object_uri: None,
        content_hash: None,
        created_at: Some(1_710_000_211),
    })
    .expect("add drift event");

    let err = app
        .execute_event_archive(plan)
        .expect_err("drifted archive plan must fail");
    assert!(
        err.to_string()
            .contains("different event set than the approved plan")
    );
}

#[test]
fn realistic_event_timeline_covers_full_fields_filters_search_and_archive_flow() {
    let temp = tempdir().expect("tempdir");
    let app = AxiomSync::new(temp.path()).expect("app");
    app.initialize().expect("init");

    let repo = temp.path().join("incident-assets");
    write_fixture(
        &repo.join("runbooks/oauth-major-incident.md"),
        r#"# OAuth Major Incident

Use break-glass client rotation, refresh the JWKS bundle, restart auth-worker, and replay
stuck token exchange jobs until refresh token success rates stabilize.
"#,
    );
    let mount = app
        .mount_repo(RepoMountRequest {
            source_path: repo.to_string_lossy().to_string(),
            target_uri: AxiomUri::parse("axiom://resources/acme/repos/incident-assets")
                .expect("uri"),
            namespace: NamespaceKey::parse("acme/identity").expect("namespace"),
            kind: Kind::new("repository").expect("kind"),
            title: Some("Incident Assets".to_string()),
            tags: vec!["runbook".to_string(), "incident".to_string()],
            attrs: serde_json::json!({"owner_team": "identity-platform"}),
            wait: true,
        })
        .expect("mount incident assets");

    let runbook_uri = AxiomUri::parse(
        "axiom://resources/acme/repos/incident-assets/runbooks/oauth-major-incident.md",
    )
    .expect("runbook uri");
    let actor_uri =
        AxiomUri::parse("axiom://resources/acme/repos/incident-assets").expect("actor uri");

    let oversized_payload = serde_json::json!({
        "cluster": "prod-seoul-2",
        "service": "token-exchange",
        "error_rate": 0.64,
        "customer_impact": {
            "failed_refreshes": 18421,
            "regions": ["ap-northeast-2", "us-west-2"],
            "support_ticket_ids": ["SUP-10421", "SUP-10433", "SUP-10437"]
        },
        "timeline": [
            "02:14 alert fired",
            "02:17 auth-worker saturation confirmed",
            "02:21 break-glass client rotation started",
            "02:26 jwks publisher redeployed"
        ],
        "raw_payload": "token-refresh-failure ".repeat(600)
    });

    let events = app
        .add_events(vec![
            AddEventRequest {
                event_id: "inc-2026-03-15-oauth-refresh".to_string(),
                uri: AxiomUri::parse("axiom://events/acme/incidents/oauth-refresh-2026-03-15")
                    .expect("incident uri"),
                namespace: NamespaceKey::parse("acme/identity/prod").expect("namespace"),
                kind: Kind::new("incident").expect("kind"),
                event_time: 1_710_500_000,
                title: Some("OAuth refresh token failure storm".to_string()),
                summary_text: Some(
                    "refresh token verification failed after stale JWKS publication".to_string(),
                ),
                severity: Some("sev1".to_string()),
                actor_uri: Some(actor_uri.clone()),
                subject_uri: Some(runbook_uri.clone()),
                run_id: Some("run-oauth-rollback-20260315".to_string()),
                session_id: Some("s-incident-20260315".to_string()),
                tags: vec![
                    "OAuth".to_string(),
                    "SEV1".to_string(),
                    "customer-impact".to_string(),
                ],
                attrs: oversized_payload,
                object_uri: None,
                content_hash: Some("manual-hash-should-be-overridden".to_string()),
                created_at: Some(1_710_500_005),
            },
            AddEventRequest {
                event_id: "run-2026-03-15-oauth-rollback".to_string(),
                uri: AxiomUri::parse("axiom://events/acme/runs/oauth-rollback-2026-03-15")
                    .expect("run uri"),
                namespace: NamespaceKey::parse("acme/identity/prod").expect("namespace"),
                kind: Kind::new("run").expect("kind"),
                event_time: 1_710_500_120,
                title: Some("OAuth rollback execution".to_string()),
                summary_text: Some(
                    "rotated clients, redeployed jwks publisher, restarted auth-worker".to_string(),
                ),
                severity: Some("info".to_string()),
                actor_uri: Some(actor_uri.clone()),
                subject_uri: Some(runbook_uri.clone()),
                run_id: Some("run-oauth-rollback-20260315".to_string()),
                session_id: Some("s-incident-20260315".to_string()),
                tags: vec![
                    "rollback".to_string(),
                    "oauth".to_string(),
                    "operator".to_string(),
                ],
                attrs: serde_json::json!({
                    "operator": "sre-oncall",
                    "change_ticket": "CHG-2026-0315-44",
                    "steps": ["rotate client", "publish jwks", "restart auth-worker"]
                }),
                object_uri: None,
                content_hash: Some("run-hash-1".to_string()),
                created_at: Some(1_710_500_121),
            },
            AddEventRequest {
                event_id: "log-2026-03-15-auth-worker".to_string(),
                uri: AxiomUri::parse("axiom://events/acme/logs/auth-worker-2026-03-15")
                    .expect("log uri"),
                namespace: NamespaceKey::parse("acme/identity/prod").expect("namespace"),
                kind: Kind::new("log").expect("kind"),
                event_time: 1_710_500_090,
                title: Some("Auth-worker hot log".to_string()),
                summary_text: Some(
                    "token exchange queue backed up while refresh verification failed".to_string(),
                ),
                severity: None,
                actor_uri: Some(actor_uri.clone()),
                subject_uri: Some(runbook_uri.clone()),
                run_id: Some("run-oauth-rollback-20260315".to_string()),
                session_id: Some("s-incident-20260315".to_string()),
                tags: vec!["oauth".to_string(), "log".to_string(), "queue".to_string()],
                attrs: serde_json::json!({
                    "stream": "stderr",
                    "line_count": 412,
                    "sample": "refresh verify failed: stale jwks kid=prod-2026-03-15"
                }),
                object_uri: None,
                content_hash: None,
                created_at: Some(1_710_500_091),
            },
        ])
        .expect("add event timeline");
    assert_eq!(events.len(), 3);

    let incident = app
        .state
        .query_events(crate::models::EventQuery {
            namespace_prefix: Some(NamespaceKey::parse("acme/identity").expect("namespace")),
            kind: Some(Kind::new("incident").expect("kind")),
            start_time: Some(1_710_499_000),
            end_time: Some(1_710_501_000),
            limit: Some(10),
            include_tombstoned: false,
        })
        .expect("query incident");
    assert_eq!(incident.len(), 1);
    let incident = &incident[0];
    assert_eq!(incident.event_id, "inc-2026-03-15-oauth-refresh");
    assert_eq!(incident.namespace.as_path(), "acme/identity/prod");
    assert_eq!(incident.kind.as_str(), "incident");
    assert_eq!(incident.severity.as_deref(), Some("sev1"));
    assert_eq!(incident.actor_uri.as_ref(), Some(&actor_uri));
    assert_eq!(incident.subject_uri.as_ref(), Some(&runbook_uri));
    assert_eq!(
        incident.run_id.as_deref(),
        Some("run-oauth-rollback-20260315")
    );
    assert_eq!(incident.session_id.as_deref(), Some("s-incident-20260315"));
    assert_eq!(
        incident.tags,
        vec![
            "customer-impact".to_string(),
            "oauth".to_string(),
            "sev1".to_string()
        ]
    );
    assert!(incident.attrs.get("raw_payload").is_none());
    assert!(
        incident
            .attrs
            .get("externalized")
            .and_then(|value| value.get("bytes"))
            .is_some()
    );
    assert!(incident.object_uri.is_some());
    assert!(incident.content_hash.is_some());
    assert!(incident.tombstoned_at.is_none());
    assert_eq!(incident.created_at, 1_710_500_005);

    let incident_object_uri = incident.object_uri.clone().expect("incident object uri");
    let incident_object: serde_json::Value = serde_json::from_slice(
        &app.fs
            .read_bytes(&incident_object_uri)
            .expect("read incident object"),
    )
    .expect("incident object json");
    assert_eq!(
        incident_object
            .get("customer_impact")
            .and_then(|value| value.get("failed_refreshes"))
            .and_then(|value| value.as_i64()),
        Some(18_421)
    );
    assert!(incident_object.get("raw_payload").is_some());

    let filtered_search = app
        .search(
            "OAuth refresh token failure storm stale JWKS publication",
            None,
            Some("s-incident-20260315"),
            Some(10),
            None,
            Some(MetadataFilter {
                fields: HashMap::from([
                    (
                        "namespace_prefix".to_string(),
                        serde_json::json!("acme/identity/prod"),
                    ),
                    ("kind".to_string(), serde_json::json!("incident")),
                    (
                        "start_time".to_string(),
                        serde_json::json!(1_710_499_000_i64),
                    ),
                    ("end_time".to_string(), serde_json::json!(1_710_501_000_i64)),
                ]),
            }),
        )
        .expect("search incidents");
    assert!(
        filtered_search
            .query_results
            .iter()
            .any(|hit| hit.uri == "axiom://events/acme/incidents/oauth-refresh-2026-03-15")
    );

    let archive_plan = app
        .plan_event_archive(
            "oauth-hot-log-2026-03-15",
            crate::models::EventQuery {
                namespace_prefix: Some(NamespaceKey::parse("acme/identity").expect("namespace")),
                kind: Some(Kind::new("log").expect("kind")),
                start_time: Some(1_710_499_000),
                end_time: Some(1_710_501_000),
                limit: Some(10),
                include_tombstoned: false,
            },
            Some("compact hot logs".to_string()),
            Some("incident-suite".to_string()),
        )
        .expect("plan log archive");
    let archive = app
        .execute_event_archive(archive_plan)
        .expect("archive log events");
    assert_eq!(archive.event_count, 1);
    assert_eq!(archive.retention, crate::models::RetentionClass::Ephemeral);

    let archived_logs = app
        .state
        .query_events(crate::models::EventQuery {
            namespace_prefix: Some(NamespaceKey::parse("acme/identity").expect("namespace")),
            kind: Some(Kind::new("log").expect("kind")),
            start_time: Some(1_710_499_000),
            end_time: Some(1_710_501_000),
            limit: Some(10),
            include_tombstoned: false,
        })
        .expect("query archived log events");
    assert_eq!(archived_logs.len(), 1);
    assert_eq!(
        archived_logs[0].object_uri.as_ref(),
        Some(&archive.object_uri)
    );
    assert_eq!(
        archived_logs[0]
            .attrs
            .get("archived")
            .and_then(|value| value.get("archive_id"))
            .and_then(|value| value.as_str()),
        Some("oauth-hot-log-2026-03-15")
    );

    let docs = app.state.list_search_documents().expect("list search docs");
    assert!(
        docs.iter()
            .any(|doc| { doc.uri == "axiom://events/acme/incidents/oauth-refresh-2026-03-15" })
    );
    assert!(
        !docs
            .iter()
            .any(|doc| { doc.uri == "axiom://events/acme/logs/auth-worker-2026-03-15" })
    );

    let mounted_runbook_search = app
        .find(
            "break-glass client rotation replay stuck token exchange jobs",
            Some(&mount.root_uri.to_string()),
            Some(10),
            None,
            None,
        )
        .expect("find runbook");
    assert!(mounted_runbook_search.query_results.iter().any(|hit| {
        hit.uri == "axiom://resources/acme/repos/incident-assets/runbooks/oauth-major-incident.md"
    }));
}

#[test]
fn plan_event_archive_rejects_mixed_retention_classes() {
    let temp = tempdir().expect("tempdir");
    let app = AxiomSync::new(temp.path()).expect("app");
    app.initialize().expect("init");

    app.add_events(vec![
        AddEventRequest {
            event_id: "incident-1".to_string(),
            uri: AxiomUri::parse("axiom://events/acme/incidents/1").expect("uri"),
            namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
            kind: Kind::new("incident").expect("kind"),
            event_time: 1_710_000_200,
            title: Some("Incident".to_string()),
            summary_text: Some("oauth outage".to_string()),
            severity: Some("high".to_string()),
            actor_uri: None,
            subject_uri: None,
            run_id: Some("run-1".to_string()),
            session_id: None,
            tags: vec!["oauth".to_string()],
            attrs: serde_json::json!({}),
            object_uri: None,
            content_hash: None,
            created_at: Some(1_710_000_201),
        },
        AddEventRequest {
            event_id: "log-1".to_string(),
            uri: AxiomUri::parse("axiom://events/acme/logs/1").expect("uri"),
            namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
            kind: Kind::new("log").expect("kind"),
            event_time: 1_710_000_210,
            title: Some("Log".to_string()),
            summary_text: Some("oauth retry".to_string()),
            severity: None,
            actor_uri: None,
            subject_uri: None,
            run_id: Some("run-1".to_string()),
            session_id: None,
            tags: vec!["oauth".to_string()],
            attrs: serde_json::json!({}),
            object_uri: None,
            content_hash: None,
            created_at: Some(1_710_000_211),
        },
    ])
    .expect("add events");

    let err = app
        .plan_event_archive(
            "mixed-retention",
            crate::models::EventQuery {
                namespace_prefix: Some(NamespaceKey::parse("acme").expect("namespace")),
                kind: None,
                start_time: Some(1_710_000_000),
                end_time: Some(1_710_000_300),
                limit: Some(10),
                include_tombstoned: false,
            },
            None,
            None,
        )
        .expect_err("mixed retention must fail");
    assert!(format!("{err:#}").contains("same retention class"));
}

#[test]
fn org_repo_workflow_runs_mount_event_link_search_and_session_flow_end_to_end() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path().join("org-repo");
    fs::create_dir_all(repo.join("contracts")).expect("contracts dir");
    fs::create_dir_all(repo.join("adrs")).expect("adrs dir");
    fs::create_dir_all(repo.join("runbooks")).expect("runbooks dir");
    fs::write(
        repo.join("contracts/auth.md"),
        "# Auth Contract\n\nOAuth refresh token rotation is mandatory for incident recovery.\n",
    )
    .expect("write contract");
    fs::write(
        repo.join("adrs/adr-0001-oauth.md"),
        "# ADR 0001\n\nWe use OAuth token rotation to reduce blast radius.\n",
    )
    .expect("write adr");
    fs::write(
        repo.join("runbooks/oauth.md"),
        "# OAuth Runbook\n\nIf refresh token failures spike, restart the auth worker and rotate keys.\n",
    )
    .expect("write runbook");

    let app = AxiomSync::new(temp.path().join("runtime")).expect("app");
    app.initialize().expect("init");

    let mount = app
        .mount_repo(RepoMountRequest {
            source_path: repo.to_string_lossy().to_string(),
            target_uri: AxiomUri::parse("axiom://resources/acme/repos/platform").expect("uri"),
            namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
            kind: Kind::new("repository").expect("kind"),
            title: Some("Platform Repo".to_string()),
            tags: vec!["platform".to_string(), "repo".to_string()],
            attrs: serde_json::json!({"owner": "platform"}),
            wait: true,
        })
        .expect("mount repo");

    let mounted_find = app
        .find(
            "OAuth refresh token rotation",
            Some(&mount.root_uri.to_string()),
            Some(10),
            None,
            None,
        )
        .expect("find mounted repo content");
    let mounted_uris = mounted_find
        .query_results
        .iter()
        .map(|hit| hit.uri.as_str())
        .collect::<Vec<_>>();
    assert!(
        mounted_uris
            .iter()
            .any(|uri| uri.ends_with("/contracts/auth.md"))
    );
    assert!(
        mounted_uris
            .iter()
            .any(|uri| uri.ends_with("/runbooks/oauth.md"))
    );

    let runbook_uri = mounted_find
        .query_results
        .iter()
        .find(|hit| hit.uri.ends_with("/runbooks/oauth.md"))
        .map(|hit| AxiomUri::parse(&hit.uri).expect("runbook uri"))
        .expect("mounted runbook hit");

    let events = app
        .add_events(vec![
            AddEventRequest {
                event_id: "evt-incident-1".to_string(),
                uri: AxiomUri::parse("axiom://events/acme/incidents/oauth-outage").expect("uri"),
                namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
                kind: Kind::new("incident").expect("kind"),
                event_time: 1_710_000_000,
                title: Some("OAuth outage".to_string()),
                summary_text: Some("refresh token failures spike across auth worker".to_string()),
                severity: Some("high".to_string()),
                actor_uri: None,
                subject_uri: None,
                run_id: Some("run-incident-1".to_string()),
                session_id: None,
                tags: vec!["oauth".to_string(), "incident".to_string()],
                attrs: serde_json::json!({"env": "prod", "component": "auth"}),
                object_uri: None,
                content_hash: None,
                created_at: Some(1_710_000_001),
            },
            AddEventRequest {
                event_id: "evt-run-1".to_string(),
                uri: AxiomUri::parse("axiom://events/acme/runs/auth-restart").expect("uri"),
                namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
                kind: Kind::new("run").expect("kind"),
                event_time: 1_710_000_030,
                title: Some("Auth worker restart".to_string()),
                summary_text: Some("rotated OAuth keys and restarted auth worker".to_string()),
                severity: None,
                actor_uri: None,
                subject_uri: None,
                run_id: Some("run-incident-1".to_string()),
                session_id: None,
                tags: vec!["oauth".to_string(), "runbook".to_string()],
                attrs: serde_json::json!({"env": "prod", "operator": "sre"}),
                object_uri: None,
                content_hash: None,
                created_at: Some(1_710_000_031),
            },
            AddEventRequest {
                event_id: "evt-log-1".to_string(),
                uri: AxiomUri::parse("axiom://events/acme/logs/auth-worker").expect("uri"),
                namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
                kind: Kind::new("log").expect("kind"),
                event_time: 1_710_000_040,
                title: Some("Auth worker log".to_string()),
                summary_text: Some("oauth retry loop observed in auth worker".to_string()),
                severity: None,
                actor_uri: None,
                subject_uri: None,
                run_id: Some("run-incident-1".to_string()),
                session_id: None,
                tags: vec!["oauth".to_string(), "log".to_string()],
                attrs: serde_json::json!({"env": "prod", "source": "worker-log"}),
                object_uri: None,
                content_hash: None,
                created_at: Some(1_710_000_041),
            },
        ])
        .expect("add events");
    assert_eq!(events.len(), 3);

    let link = app
        .link_records(LinkRequest {
            link_id: "link-oauth-runbook".to_string(),
            namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
            from_uri: events[0].uri.clone(),
            relation: "resolved_by".to_string(),
            to_uri: runbook_uri.clone(),
            weight: 1.0,
            attrs: serde_json::json!({"source": "incident_response"}),
            created_at: Some(1_710_000_032),
        })
        .expect("link runbook");
    assert_eq!(link.relation, "resolved_by");
    let stored_links = app
        .state
        .query_links(crate::models::LinkQuery {
            namespace_prefix: Some(NamespaceKey::parse("acme").expect("namespace")),
            from_uri: Some(events[0].uri.clone()),
            to_uri: Some(runbook_uri.clone()),
            relation: Some("resolved_by".to_string()),
            limit: Some(10),
        })
        .expect("query links");
    assert_eq!(stored_links.len(), 1);

    let session = app.session(Some("s-org-repo-e2e"));
    session.load().expect("session load");
    session
        .add_message(
            "user",
            "Investigate the OAuth outage and use the platform runbook for recovery.",
        )
        .expect("add message");
    let commit = session.commit().expect("commit");
    assert_eq!(commit.session_id, "s-org-repo-e2e");

    let persisted_events = app
        .state
        .query_events(crate::models::EventQuery {
            namespace_prefix: Some(NamespaceKey::parse("acme").expect("namespace")),
            kind: Some(Kind::new("incident").expect("kind")),
            start_time: Some(1_710_000_000),
            end_time: Some(1_710_000_100),
            limit: Some(10),
            include_tombstoned: false,
        })
        .expect("query events");
    assert_eq!(persisted_events.len(), 1);
    assert_eq!(persisted_events[0].uri, events[0].uri);

    let projected_docs = app.state.list_search_documents().expect("list search docs");
    assert!(
        projected_docs
            .iter()
            .any(|doc| doc.uri == events[0].uri.to_string())
    );
    let log_events = app
        .state
        .query_events(crate::models::EventQuery {
            namespace_prefix: Some(NamespaceKey::parse("acme").expect("namespace")),
            kind: Some(Kind::new("log").expect("kind")),
            start_time: Some(1_710_000_000),
            end_time: Some(1_710_000_100),
            limit: Some(10),
            include_tombstoned: false,
        })
        .expect("query log events");
    assert_eq!(log_events.len(), 1);
    assert_eq!(log_events[0].event_id, "evt-log-1");

    let search = app
        .search(
            "oauth outage recovery runbook",
            None,
            Some("s-org-repo-e2e"),
            Some(10),
            None,
            Some(MetadataFilter {
                fields: HashMap::from([(
                    "namespace_prefix".to_string(),
                    serde_json::json!("acme/platform"),
                )]),
            }),
        )
        .expect("search org workflow");

    assert!(!search.query_results.is_empty());
    assert!(
        search
            .query_plan
            .typed_queries
            .iter()
            .any(|typed| typed.kind == "session_recent")
    );
}

#[test]
fn real_repo_markdown_workflow_exercises_live_ingest_retrieval_events_sessions_and_release_verify()
{
    let temp = tempdir().expect("tempdir");
    let app = AxiomSync::new(temp.path().join("runtime")).expect("app");
    app.initialize().expect("init");

    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let api_contract = workspace.join("docs/API_CONTRACT.md");
    let release_runbook = workspace.join("docs/RELEASE_RUNBOOK.md");
    let runtime_architecture = workspace.join("docs/RUNTIME_ARCHITECTURE.md");
    assert!(
        api_contract.exists(),
        "missing fixture {}",
        api_contract.display()
    );
    assert!(
        release_runbook.exists(),
        "missing fixture {}",
        release_runbook.display()
    );
    assert!(
        runtime_architecture.exists(),
        "missing fixture {}",
        runtime_architecture.display()
    );

    let direct_contract = app
        .add_resource(
            api_contract.to_str().expect("api contract str"),
            Some("axiom://resources/acme/reference/API_CONTRACT.md"),
            None,
            None,
            true,
            Some(30),
        )
        .expect("add api contract");
    let contract_find = app
        .find(
            "canonical retrieval result shape compat-json hit_buckets",
            Some(&direct_contract.root_uri),
            Some(5),
            None,
            None,
        )
        .expect("find api contract section");
    let contract_hit = contract_find
        .query_results
        .iter()
        .find(|hit| hit.matched_heading.as_deref() == Some("Retrieval Contract"))
        .expect("api contract hit");
    let contract_resource_uri = contract_hit.uri.clone();
    assert!(
        contract_resource_uri.starts_with(&direct_contract.root_uri),
        "api contract hit must stay under requested target: {} vs {}",
        contract_resource_uri,
        direct_contract.root_uri
    );
    assert_eq!(
        contract_hit.matched_heading.as_deref(),
        Some("Retrieval Contract")
    );
    assert!(contract_hit.snippet.is_some());

    let repo = temp.path().join("live-docs-repo");
    copy_fixture(&release_runbook, &repo.join("docs/RELEASE_RUNBOOK.md"));
    copy_fixture(
        &runtime_architecture,
        &repo.join("docs/RUNTIME_ARCHITECTURE.md"),
    );
    copy_fixture(&api_contract, &repo.join("docs/API_CONTRACT.md"));

    let mount = app
        .mount_repo(RepoMountRequest {
            source_path: repo.to_string_lossy().to_string(),
            target_uri: AxiomUri::parse("axiom://resources/acme/repos/live-docs").expect("uri"),
            namespace: NamespaceKey::parse("acme/release").expect("namespace"),
            kind: Kind::new("repository").expect("kind"),
            title: Some("Live Docs Repo".to_string()),
            tags: vec!["release".to_string(), "docs".to_string()],
            attrs: serde_json::json!({"source": "workspace-markdown"}),
            wait: true,
        })
        .expect("mount live docs repo");

    let mounted_root = mount.root_uri.to_string();
    let mounted_release_uri = format!("{mounted_root}/docs/RELEASE_RUNBOOK.md");
    let mounted_runtime_uri = format!("{mounted_root}/docs/RUNTIME_ARCHITECTURE.md");

    let release_find = app
        .find("decision rules", Some(&mounted_root), Some(10), None, None)
        .expect("find release runbook section");
    let release_hit = release_find
        .query_results
        .iter()
        .find(|hit| hit.uri == mounted_release_uri)
        .expect("mounted release runbook hit");
    assert_eq!(
        release_hit.matched_heading.as_deref(),
        Some("Release Decision Rules")
    );
    assert!(
        release_hit
            .snippet
            .as_deref()
            .is_some_and(|snippet| !snippet.is_empty())
    );

    let runtime_find = app
        .find(
            "main data flows add_events events table search_docs projection memory index sync",
            Some(&mounted_root),
            Some(10),
            None,
            None,
        )
        .expect("find runtime architecture section");
    let runtime_hit = runtime_find
        .query_results
        .iter()
        .find(|hit| hit.uri == mounted_runtime_uri)
        .expect("mounted runtime architecture hit");
    assert_eq!(
        runtime_hit.matched_heading.as_deref(),
        Some("Main Data Flows")
    );
    assert!(runtime_hit.snippet.is_some());

    let incident_uri =
        AxiomUri::parse("axiom://events/acme/release/incidents/live-docs-drill").expect("uri");
    let verify_uri =
        AxiomUri::parse("axiom://events/acme/release/runs/live-release-verify").expect("uri");
    let log_uri =
        AxiomUri::parse("axiom://events/acme/release/logs/live-release-probe").expect("uri");
    let events = app
        .add_events(vec![
            AddEventRequest {
                event_id: "evt-live-release-incident".to_string(),
                uri: incident_uri.clone(),
                namespace: NamespaceKey::parse("acme/release").expect("namespace"),
                kind: Kind::new("incident").expect("kind"),
                event_time: 1_710_700_000,
                title: Some("Release evidence drill".to_string()),
                summary_text: Some(
                    "release verify and required gates reviewed against live runbook sections"
                        .to_string(),
                ),
                severity: Some("high".to_string()),
                actor_uri: None,
                subject_uri: Some(
                    AxiomUri::parse(&mounted_release_uri).expect("mounted release uri"),
                ),
                run_id: Some("run-live-release".to_string()),
                session_id: None,
                tags: vec!["release".to_string(), "docs".to_string()],
                attrs: serde_json::json!({"section": "Release Decision Rules"}),
                object_uri: None,
                content_hash: None,
                created_at: Some(1_710_700_001),
            },
            AddEventRequest {
                event_id: "evt-live-release-verify".to_string(),
                uri: verify_uri.clone(),
                namespace: NamespaceKey::parse("acme/release").expect("namespace"),
                kind: Kind::new("run").expect("kind"),
                event_time: 1_710_700_010,
                title: Some("Release verify".to_string()),
                summary_text: Some(
                    "release verify confirmed context schema, FTS readiness, and retrieval backend"
                        .to_string(),
                ),
                severity: None,
                actor_uri: None,
                subject_uri: Some(
                    AxiomUri::parse(&contract_resource_uri).expect("api contract uri"),
                ),
                run_id: Some("run-live-release".to_string()),
                session_id: None,
                tags: vec!["release".to_string(), "verify".to_string()],
                attrs: serde_json::json!({"section": "Release Gate Contract"}),
                object_uri: None,
                content_hash: None,
                created_at: Some(1_710_700_011),
            },
            AddEventRequest {
                event_id: "evt-live-release-log".to_string(),
                uri: log_uri.clone(),
                namespace: NamespaceKey::parse("acme/release").expect("namespace"),
                kind: Kind::new("log").expect("kind"),
                event_time: 1_710_700_020,
                title: Some("Release probe log".to_string()),
                summary_text: Some(
                    "release pack strict gate emitted passed true and no benchmark reports"
                        .to_string(),
                ),
                severity: None,
                actor_uri: None,
                subject_uri: None,
                run_id: Some("run-live-release".to_string()),
                session_id: None,
                tags: vec!["release".to_string(), "log".to_string()],
                attrs: serde_json::json!({"source": "probe"}),
                object_uri: None,
                content_hash: None,
                created_at: Some(1_710_700_021),
            },
        ])
        .expect("add release events");
    assert_eq!(events.len(), 3);

    app.link_records(LinkRequest {
        link_id: "link-live-release-runbook".to_string(),
        namespace: NamespaceKey::parse("acme/release").expect("namespace"),
        from_uri: incident_uri.clone(),
        relation: "resolved_by".to_string(),
        to_uri: AxiomUri::parse(&mounted_release_uri).expect("mounted release uri"),
        weight: 1.0,
        attrs: serde_json::json!({"source": "runbook"}),
        created_at: Some(1_710_700_012),
    })
    .expect("link release runbook");
    app.link_records(LinkRequest {
        link_id: "link-live-release-contract".to_string(),
        namespace: NamespaceKey::parse("acme/release").expect("namespace"),
        from_uri: verify_uri.clone(),
        relation: "documents".to_string(),
        to_uri: AxiomUri::parse(&contract_resource_uri).expect("api contract uri"),
        weight: 0.8,
        attrs: serde_json::json!({"source": "contract"}),
        created_at: Some(1_710_700_013),
    })
    .expect("link api contract");
    let stored_links = app
        .state
        .query_links(crate::models::LinkQuery {
            namespace_prefix: Some(NamespaceKey::parse("acme").expect("namespace")),
            from_uri: None,
            to_uri: None,
            relation: None,
            limit: Some(10),
        })
        .expect("query stored links");
    assert_eq!(stored_links.len(), 2);

    let session = app.session(Some("s-live-repo-hard-test"));
    session.load().expect("session load");
    session
        .add_message(
            "user",
            "Check the live release docs, confirm required gates, and keep the incident evidence linked.",
        )
        .expect("add message");
    let commit = session.commit().expect("commit session");
    assert_eq!(commit.session_id, "s-live-repo-hard-test");
    let sessions = app.sessions().expect("list sessions");
    assert!(
        sessions
            .iter()
            .any(|info| info.session_id == "s-live-repo-hard-test")
    );

    let event_search = app
        .search(
            "release verify required gates evidence",
            Some("axiom://events"),
            Some("s-live-repo-hard-test"),
            Some(10),
            None,
            Some(MetadataFilter {
                fields: HashMap::from([(
                    "namespace_prefix".to_string(),
                    serde_json::json!("acme/release"),
                )]),
            }),
        )
        .expect("search release events");
    assert!(
        event_search
            .query_results
            .iter()
            .any(|hit| hit.uri == incident_uri.to_string())
    );
    assert!(
        event_search
            .query_results
            .iter()
            .any(|hit| hit.uri == verify_uri.to_string())
    );
    assert!(
        event_search
            .query_plan
            .typed_queries
            .iter()
            .any(|typed| typed.kind == "session_recent")
    );

    let archive_plan = app
        .plan_event_archive(
            "live-release-log-archive",
            crate::models::EventQuery {
                namespace_prefix: Some(NamespaceKey::parse("acme/release").expect("namespace")),
                kind: Some(Kind::new("log").expect("kind")),
                start_time: Some(1_710_700_000),
                end_time: Some(1_710_700_100),
                limit: Some(10),
                include_tombstoned: false,
            },
            Some("compact live probe logs".to_string()),
            Some("hard-test".to_string()),
        )
        .expect("plan release log archive");
    let archive = app
        .execute_event_archive(archive_plan)
        .expect("execute release log archive");
    assert_eq!(archive.event_count, 1);

    let docs = app.state.list_search_documents().expect("list search docs");
    assert!(
        !docs.iter().any(|doc| doc.uri == log_uri.to_string()),
        "archived log must be removed from search docs"
    );
    let archived_logs = app
        .state
        .query_events(crate::models::EventQuery {
            namespace_prefix: Some(NamespaceKey::parse("acme/release").expect("namespace")),
            kind: Some(Kind::new("log").expect("kind")),
            start_time: Some(1_710_700_000),
            end_time: Some(1_710_700_100),
            limit: Some(10),
            include_tombstoned: false,
        })
        .expect("query archived release logs");
    assert_eq!(archived_logs.len(), 1);
    assert_eq!(
        archived_logs[0].object_uri.as_ref(),
        Some(&archive.object_uri)
    );

    let verify = app.release_verify().expect("release verify");
    assert!(
        verify.is_healthy(),
        "release verify must be healthy: {verify:?}"
    );
    assert!(verify.storage.search_document_count >= 6);
    assert!(verify.storage.event_count >= 3);
    assert!(verify.storage.link_count >= 2);
    assert!(verify.retrieval.fts_ready);
    assert!(verify.retrieval.indexed_documents >= 6);
}

fn write_fixture(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fixture parent");
    }
    fs::write(path, content).expect("write fixture");
}

fn copy_fixture(source: &Path, target: &Path) {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).expect("create copy parent");
    }
    fs::copy(source, target).expect("copy fixture");
}
