use anyhow::{Context as _, Result};
use axiomsync::AxiomSync;
use axiomsync::client::BenchmarkFixtureCreateOptions;
use axiomsync::models::{
    AddEventRequest, BenchmarkGateOptions, BenchmarkRunOptions, EvalRunOptions, EventArchivePlan,
    EventQuery, Kind, LinkRequest, NamespaceKey, ReleaseGateBenchmarkGatePlan,
    ReleaseGateBenchmarkRunPlan, ReleaseGateEvalPlan, ReleaseGateOperabilityPlan,
    ReleaseGatePackOptions, ReleaseGateReplayPlan, ReleaseSecurityAuditMode, RepoMountRequest,
};

use crate::cli::{
    BenchmarkCommand, BenchmarkFixtureCommand, DoctorCommand, EvalCommand, EvalGoldenCommand,
    EventArchiveCommand, EventCommand, LinkCommand, MigrateCommand, RelationCommand,
    ReleaseCommand, ReleaseSecurityAuditModeArg, RepoCommand, SecurityAuditModeArg,
    SecurityCommand, SessionCommand, TraceCommand,
};

use super::print_json;

pub(super) fn handle_session(app: &AxiomSync, command: SessionCommand) -> Result<()> {
    match command {
        SessionCommand::Create { id } => {
            let session = app.session(id.as_deref());
            session.load()?;
            println!("{}", session.session_id);
        }
        SessionCommand::Add { id, role, text } => {
            let session = app.session(Some(&id));
            session.load()?;
            let message = session.add_message(&role, text)?;
            print_json(&message)?;
        }
        SessionCommand::Commit { id } => {
            let session = app.session(Some(&id));
            session.load()?;
            let result = session.commit()?;
            print_json(&result)?;
        }
        SessionCommand::List => {
            let sessions = app.sessions()?;
            print_json(&sessions)?;
        }
        SessionCommand::Delete { id } => {
            let deleted = app.delete(&id)?;
            println!("{deleted}");
        }
    }
    Ok(())
}

fn run_benchmark_fixture_command(app: &AxiomSync, command: BenchmarkFixtureCommand) -> Result<()> {
    match command {
        BenchmarkFixtureCommand::Create {
            name,
            query_limit,
            include_golden,
            include_trace,
            include_stress,
            trace_expectations,
        } => {
            let summary = app.create_benchmark_fixture(
                &name,
                BenchmarkFixtureCreateOptions {
                    query_limit,
                    include_golden,
                    include_trace,
                    include_stress,
                    trace_expectations,
                },
            )?;
            print_json(&summary)?;
        }
        BenchmarkFixtureCommand::List { limit } => {
            let fixtures = app.list_benchmark_fixtures(limit)?;
            print_json(&fixtures)?;
        }
    }
    Ok(())
}

pub(super) fn handle_trace(app: &AxiomSync, command: TraceCommand) -> Result<()> {
    match command {
        TraceCommand::Requests {
            limit,
            operation,
            status,
        } => {
            let logs =
                app.list_request_logs_filtered(limit, operation.as_deref(), status.as_deref())?;
            print_json(&logs)?;
        }
        TraceCommand::List { limit } => {
            let traces = app.list_traces(limit)?;
            print_json(&traces)?;
        }
        TraceCommand::Get { trace_id } => {
            let trace = app.get_trace(&trace_id)?;
            print_json(&trace)?;
        }
        TraceCommand::Replay { trace_id, limit } => {
            let replay = app.replay_trace(&trace_id, limit)?;
            print_json(&replay)?;
        }
        TraceCommand::Stats {
            limit,
            include_replays,
        } => {
            let stats = app.trace_metrics(limit, include_replays)?;
            print_json(&stats)?;
        }
        TraceCommand::Snapshot {
            limit,
            include_replays,
        } => {
            let snapshot = app.create_trace_metrics_snapshot(limit, include_replays)?;
            print_json(&snapshot)?;
        }
        TraceCommand::Snapshots { limit } => {
            let snapshots = app.list_trace_metrics_snapshots(limit)?;
            print_json(&snapshots)?;
        }
        TraceCommand::Trend {
            limit,
            request_type,
        } => {
            let trend = app.trace_metrics_trend(limit, request_type.as_deref())?;
            print_json(&trend)?;
        }
        TraceCommand::Evidence {
            trace_limit,
            request_limit,
            enforce,
        } => {
            let report = app.collect_operability_evidence(trace_limit, request_limit)?;
            print_json(&report)?;
            if enforce && !report.passed {
                anyhow::bail!("operability evidence checks failed");
            }
        }
    }
    Ok(())
}

pub(super) fn handle_relation(app: &AxiomSync, command: RelationCommand) -> Result<()> {
    match command {
        RelationCommand::List { owner_uri } => {
            let relations = app.relations(&owner_uri)?;
            print_json(&serde_json::json!({
                "status": "ok",
                "owner_uri": owner_uri,
                "relations": relations
            }))?;
        }
        RelationCommand::Link {
            owner_uri,
            relation_id,
            uris,
            reason,
        } => {
            let relation = app.link(&owner_uri, &relation_id, uris, &reason)?;
            print_json(&serde_json::json!({
                "status": "ok",
                "owner_uri": owner_uri,
                "relation": relation
            }))?;
        }
        RelationCommand::Unlink {
            owner_uri,
            relation_id,
        } => {
            let removed = app.unlink(&owner_uri, &relation_id)?;
            print_json(&serde_json::json!({
                "status": "ok",
                "owner_uri": owner_uri,
                "relation_id": relation_id,
                "removed": removed
            }))?;
        }
    }
    Ok(())
}

pub(super) fn handle_repo(app: &AxiomSync, command: RepoCommand) -> Result<()> {
    match command {
        RepoCommand::Mount {
            source_path,
            target_uri,
            namespace,
            kind,
            title,
            tags,
            wait,
        } => {
            let report = app.mount_repo(RepoMountRequest {
                source_path,
                target_uri: axiomsync::AxiomUri::parse(&target_uri)?,
                namespace: namespace.parse()?,
                kind: kind.parse()?,
                title,
                tags,
                attrs: serde_json::json!({}),
                wait,
            })?;
            print_json(&report)?;
        }
    }
    Ok(())
}

pub(super) fn handle_event(app: &AxiomSync, command: EventCommand) -> Result<()> {
    match command {
        EventCommand::Add {
            event_id,
            uri,
            namespace,
            kind,
            event_time,
            title,
            summary,
            severity,
            run_id,
            session_id,
            tags,
        } => {
            let event = app.add_event(AddEventRequest {
                event_id,
                uri: axiomsync::AxiomUri::parse(&uri)?,
                namespace: namespace.parse()?,
                kind: kind.parse()?,
                event_time,
                title,
                summary_text: summary,
                severity,
                actor_uri: None,
                subject_uri: None,
                run_id,
                session_id,
                tags,
                attrs: serde_json::json!({}),
                object_uri: None,
                content_hash: None,
                created_at: None,
            })?;
            print_json(&event)?;
        }
        EventCommand::Import {
            file,
            namespace,
            kind,
        } => {
            let raw = std::fs::read_to_string(&file)?;
            let namespace = namespace.parse()?;
            let kind = kind.parse()?;
            let batch = parse_event_import_requests(&raw, &namespace, &kind)?;
            let events = app.add_events(batch)?;
            print_json(&serde_json::json!({
                "status": "ok",
                "count": events.len(),
                "events": events,
            }))?;
        }
        EventCommand::Archive { command } => match command {
            EventArchiveCommand::Plan {
                archive_id,
                namespace,
                kind,
                start_time,
                end_time,
                limit,
                archive_reason,
                archived_by,
            } => {
                let plan = app.plan_event_archive(
                    &archive_id,
                    EventQuery {
                        namespace_prefix: namespace
                            .as_deref()
                            .map(NamespaceKey::parse)
                            .transpose()?,
                        kind: kind.as_deref().map(Kind::new).transpose()?,
                        start_time,
                        end_time,
                        limit,
                        include_tombstoned: false,
                    },
                    archive_reason,
                    archived_by,
                )?;
                print_json(&plan)?;
            }
            EventArchiveCommand::Execute { plan_file } => {
                let raw = std::fs::read_to_string(&plan_file).with_context(|| {
                    format!("failed to read plan file '{}'", plan_file.display())
                })?;
                let plan = serde_json::from_str::<EventArchivePlan>(&raw).with_context(|| {
                    format!(
                        "failed to parse EventArchivePlan from '{}'",
                        plan_file.display()
                    )
                })?;
                let report = app.execute_event_archive(plan)?;
                print_json(&report)?;
            }
        },
    }
    Ok(())
}

pub(super) fn handle_doctor(app: &AxiomSync, command: DoctorCommand) -> Result<()> {
    match command {
        DoctorCommand::Storage { json } => {
            require_json_flag(json, "doctor storage")?;
            print_json(&app.doctor_storage()?)?
        }
        DoctorCommand::Retrieval { json } => {
            require_json_flag(json, "doctor retrieval")?;
            print_json(&app.doctor_retrieval()?)?
        }
    }
    Ok(())
}

pub(super) fn handle_migrate(app: &AxiomSync, command: MigrateCommand) -> Result<()> {
    match command {
        MigrateCommand::Inspect { json } => {
            require_json_flag(json, "migrate inspect")?;
            print_json(&app.migrate_inspect()?)?
        }
        MigrateCommand::Apply { backup_dir, json } => {
            require_json_flag(json, "migrate apply")?;
            print_json(&app.migrate_apply(backup_dir.as_deref())?)?
        }
    }
    Ok(())
}

fn parse_event_import_requests(
    raw: &str,
    namespace: &NamespaceKey,
    kind: &Kind,
) -> Result<Vec<AddEventRequest>> {
    event_import_values(raw)?
        .into_iter()
        .enumerate()
        .map(|(index, value)| build_import_event_request(index, value, namespace, kind))
        .collect()
}

fn event_import_values(raw: &str) -> Result<Vec<serde_json::Value>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    match serde_json::from_str::<serde_json::Value>(trimmed) {
        Ok(serde_json::Value::Array(values)) => Ok(values),
        Ok(value @ serde_json::Value::Object(_)) => Ok(vec![value]),
        Ok(_) => Err(anyhow::anyhow!(
            "event import payload must be a JSON object, JSON array, or JSONL object stream"
        )),
        Err(_) => raw
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(serde_json::from_str::<serde_json::Value>)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into),
    }
}

fn build_import_event_request(
    index: usize,
    value: serde_json::Value,
    namespace: &NamespaceKey,
    kind: &Kind,
) -> Result<AddEventRequest> {
    let Some(object) = value.as_object() else {
        anyhow::bail!("event import entry {} must be a JSON object", index + 1);
    };
    let uri = object
        .get("uri")
        .and_then(|value| value.as_str())
        .ok_or_else(|| anyhow::anyhow!("event import entry {} missing uri", index + 1))?;
    let event_time = object
        .get("event_time")
        .and_then(|value| value.as_i64())
        .ok_or_else(|| anyhow::anyhow!("event import entry {} missing event_time", index + 1))?;

    Ok(AddEventRequest {
        event_id: object
            .get("event_id")
            .and_then(|value| value.as_str())
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("import-{}", index + 1)),
        uri: axiomsync::AxiomUri::parse(uri)?,
        namespace: namespace.clone(),
        kind: kind.clone(),
        event_time,
        title: object
            .get("title")
            .and_then(|value| value.as_str())
            .map(ToString::to_string),
        summary_text: object
            .get("summary_text")
            .or_else(|| object.get("summary"))
            .and_then(|value| value.as_str())
            .map(ToString::to_string),
        severity: object
            .get("severity")
            .and_then(|value| value.as_str())
            .map(ToString::to_string),
        actor_uri: None,
        subject_uri: None,
        run_id: object
            .get("run_id")
            .and_then(|value| value.as_str())
            .map(ToString::to_string),
        session_id: object
            .get("session_id")
            .and_then(|value| value.as_str())
            .map(ToString::to_string),
        tags: object
            .get("tags")
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(ToString::to_string))
                    .collect()
            })
            .unwrap_or_default(),
        attrs: import_event_attrs(object),
        object_uri: None,
        content_hash: None,
        created_at: object.get("created_at").and_then(|value| value.as_i64()),
    })
}

fn import_event_attrs(object: &serde_json::Map<String, serde_json::Value>) -> serde_json::Value {
    if let Some(attrs) = object.get("attrs") {
        return attrs.clone();
    }

    let attrs = object
        .iter()
        .filter(|(key, _)| {
            !matches!(
                key.as_str(),
                "event_id"
                    | "uri"
                    | "event_time"
                    | "title"
                    | "summary"
                    | "summary_text"
                    | "severity"
                    | "run_id"
                    | "session_id"
                    | "tags"
                    | "attrs"
                    | "created_at"
            )
        })
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    serde_json::Value::Object(attrs)
}

pub(super) fn handle_link(app: &AxiomSync, command: LinkCommand) -> Result<()> {
    match command {
        LinkCommand::Add {
            link_id,
            namespace,
            from_uri,
            relation,
            to_uri,
            weight,
        } => {
            let link = app.link_records(LinkRequest {
                link_id,
                namespace: namespace.parse()?,
                from_uri: axiomsync::AxiomUri::parse(&from_uri)?,
                relation,
                to_uri: axiomsync::AxiomUri::parse(&to_uri)?,
                weight,
                attrs: serde_json::json!({}),
                created_at: None,
            })?;
            print_json(&link)?;
        }
        LinkCommand::List {
            namespace,
            from_uri,
            to_uri,
            relation,
            limit,
        } => {
            let links = app.state.query_links(axiomsync::models::LinkQuery {
                namespace_prefix: namespace.map(|value| value.parse()).transpose()?,
                from_uri: from_uri
                    .map(|value| axiomsync::AxiomUri::parse(&value))
                    .transpose()?,
                to_uri: to_uri
                    .map(|value| axiomsync::AxiomUri::parse(&value))
                    .transpose()?,
                relation,
                limit,
            })?;
            print_json(&links)?;
        }
    }
    Ok(())
}

pub(super) fn handle_eval(app: &AxiomSync, command: EvalCommand) -> Result<()> {
    match command {
        EvalCommand::Run {
            trace_limit,
            query_limit,
            search_limit,
            include_golden,
            golden_only,
        } => {
            let report = app.run_eval_loop_with_options(&EvalRunOptions {
                trace_limit,
                query_limit,
                search_limit,
                include_golden,
                golden_only,
            })?;
            print_json(&report)?;
        }
        EvalCommand::Golden { command } => match command {
            EvalGoldenCommand::List => {
                let cases = app.list_eval_golden_queries()?;
                print_json(&cases)?;
            }
            EvalGoldenCommand::Add {
                query,
                target,
                expected_top,
            } => {
                let result =
                    app.add_eval_golden_query(&query, target.as_deref(), expected_top.as_deref())?;
                print_json(&result)?;
            }
            EvalGoldenCommand::MergeFromTraces {
                trace_limit,
                max_add,
            } => {
                let result = app.merge_eval_golden_from_traces(trace_limit, max_add)?;
                print_json(&result)?;
            }
        },
    }
    Ok(())
}

pub(super) fn handle_benchmark(app: &AxiomSync, command: BenchmarkCommand) -> Result<()> {
    match command {
        BenchmarkCommand::Run {
            query_limit,
            search_limit,
            include_golden,
            include_trace,
            include_stress,
            trace_expectations,
            fixture_name,
        } => {
            let options = BenchmarkRunOptions {
                query_limit,
                search_limit,
                include_golden,
                include_trace,
                include_stress,
                trace_expectations,
                fixture_name,
            };
            let report = app.run_benchmark_suite(&options)?;
            print_json(&report)?;
        }
        BenchmarkCommand::Amortized {
            query_limit,
            search_limit,
            include_golden,
            include_trace,
            include_stress,
            trace_expectations,
            fixture_name,
            iterations,
        } => {
            let options = BenchmarkRunOptions {
                query_limit,
                search_limit,
                include_golden,
                include_trace,
                include_stress,
                trace_expectations,
                fixture_name,
            };
            let report = app.run_benchmark_suite_amortized(options, iterations)?;
            print_json(&report)?;
        }
        BenchmarkCommand::List { limit } => {
            let reports = app.list_benchmark_reports(limit)?;
            print_json(&reports)?;
        }
        BenchmarkCommand::Trend { limit } => {
            let trend = app.benchmark_trend(limit)?;
            print_json(&trend)?;
        }
        BenchmarkCommand::Gate {
            threshold_p95_ms,
            min_top1_accuracy,
            min_stress_top1_accuracy,
            gate_profile,
            max_p95_regression_pct,
            max_top1_regression_pct,
            window_size,
            required_passes,
            record,
            write_release_check,
            enforce,
        } => {
            let result = app.benchmark_gate_with_options(BenchmarkGateOptions {
                gate_profile,
                threshold_p95_ms,
                min_top1_accuracy,
                min_stress_top1_accuracy,
                max_p95_regression_pct,
                max_top1_regression_pct,
                window_size,
                required_passes,
                record,
                write_release_check,
            })?;
            print_json(&result)?;
            if enforce && !result.passed {
                anyhow::bail!("benchmark gate failed");
            }
        }
        BenchmarkCommand::Fixture { command } => run_benchmark_fixture_command(app, command)?,
    }
    Ok(())
}

pub(super) fn handle_security(app: &AxiomSync, command: SecurityCommand) -> Result<()> {
    match command {
        SecurityCommand::Audit {
            workspace_dir,
            mode,
            enforce,
        } => {
            let mode = match mode {
                SecurityAuditModeArg::Offline => "offline",
                SecurityAuditModeArg::Strict => "strict",
            };
            let report = app.run_security_audit_with_mode(workspace_dir.as_deref(), Some(mode))?;
            print_json(&report)?;
            if enforce && !report.passed {
                anyhow::bail!("security audit failed");
            }
        }
    }
    Ok(())
}

pub(super) fn handle_release(app: &AxiomSync, command: ReleaseCommand) -> Result<()> {
    match command {
        ReleaseCommand::Verify { enforce, json } => {
            require_json_flag(json, "release verify")?;
            let report = app.release_verify()?;
            print_json(&report)?;
            if enforce && !report.is_healthy() {
                anyhow::bail!("release verify failed");
            }
        }
        ReleaseCommand::Pack {
            workspace_dir,
            replay_limit,
            replay_max_cycles,
            trace_limit,
            request_limit,
            eval_trace_limit,
            eval_query_limit,
            eval_search_limit,
            benchmark_query_limit,
            benchmark_search_limit,
            benchmark_threshold_p95_ms,
            benchmark_min_top1_accuracy,
            benchmark_min_stress_top1_accuracy,
            benchmark_max_p95_regression_pct,
            benchmark_max_top1_regression_pct,
            benchmark_window_size,
            benchmark_required_passes,
            security_audit_mode,
            enforce,
        } => {
            let security_audit_mode = match security_audit_mode {
                ReleaseSecurityAuditModeArg::Offline => ReleaseSecurityAuditMode::Offline,
                ReleaseSecurityAuditModeArg::Strict => ReleaseSecurityAuditMode::Strict,
            };
            let report = app.collect_release_gate_pack(&ReleaseGatePackOptions {
                workspace_dir,
                replay: ReleaseGateReplayPlan {
                    replay_limit,
                    replay_max_cycles,
                },
                operability: ReleaseGateOperabilityPlan {
                    trace_limit,
                    request_limit,
                },
                eval: ReleaseGateEvalPlan {
                    eval_trace_limit,
                    eval_query_limit,
                    eval_search_limit,
                },
                benchmark_run: ReleaseGateBenchmarkRunPlan {
                    benchmark_query_limit,
                    benchmark_search_limit,
                },
                benchmark_gate: ReleaseGateBenchmarkGatePlan {
                    benchmark_threshold_p95_ms,
                    benchmark_min_top1_accuracy,
                    benchmark_min_stress_top1_accuracy,
                    benchmark_max_p95_regression_pct,
                    benchmark_max_top1_regression_pct,
                    benchmark_window_size,
                    benchmark_required_passes,
                },
                security_audit_mode,
            })?;
            print_json(&report)?;
            if enforce && !report.passed {
                anyhow::bail!("release gate pack failed");
            }
        }
    }
    Ok(())
}

fn require_json_flag(enabled: bool, command: &str) -> Result<()> {
    if !enabled {
        anyhow::bail!("{command} requires --json");
    }
    Ok(())
}
