use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result, bail};
use axiomsync::models::{AddEventRequest, Kind, NamespaceKey};
use axiomsync::{AxiomSync, AxiomUri};
use serde::Serialize;

#[derive(Clone, Copy)]
struct ScenarioSpec {
    name: &'static str,
    file_count: usize,
    query_count: usize,
    queue_events: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ScenarioReport {
    scenario: String,
    corpus_files: usize,
    event_batch_count: usize,
    event_batch_ingest_ms: u128,
    cold_boot_ms: u128,
    warm_boot_ms: u128,
    full_reindex_ms: u128,
    first_search_ms: u128,
    steady_search_p50_ms: u128,
    steady_search_p95_ms: u128,
    context_db_bytes: u64,
    db_growth_bytes: u64,
    queue_replay_processed: usize,
    queue_replay_ms: u128,
    queue_replay_events_per_sec: f64,
}

#[derive(Debug, Serialize)]
struct BaselineReport {
    tool: &'static str,
    reports: Vec<ScenarioReport>,
}

const SCENARIOS: [ScenarioSpec; 3] = [
    ScenarioSpec {
        name: "small",
        file_count: 12,
        query_count: 8,
        queue_events: 6,
    },
    ScenarioSpec {
        name: "medium",
        file_count: 48,
        query_count: 12,
        queue_events: 10,
    },
    ScenarioSpec {
        name: "stress",
        file_count: 120,
        query_count: 20,
        queue_events: 16,
    },
];
const SEARCH_SCOPE_URI: &str = "axiom://resources/runtime-baseline";
const QUEUE_SCOPE_PREFIX: &str = "axiom://resources/runtime-baseline-queue";
const SEARCH_LIMIT: usize = 10;

fn main() -> Result<()> {
    let args = Args::parse()?;
    let scenarios = resolve_scenarios(args.scenario.as_deref())?;
    let mut reports = Vec::with_capacity(scenarios.len());
    let workspace_root = args.root.unwrap_or_else(|| {
        env::temp_dir().join(format!(
            "axiomsync-runtime-baseline-{}",
            uuid::Uuid::new_v4()
        ))
    });
    fs::create_dir_all(&workspace_root)
        .with_context(|| format!("create workspace root {}", workspace_root.display()))?;

    for spec in scenarios {
        let scenario_root = workspace_root.join(spec.name);
        if scenario_root.exists() {
            fs::remove_dir_all(&scenario_root)
                .with_context(|| format!("reset scenario root {}", scenario_root.display()))?;
        }
        fs::create_dir_all(&scenario_root)
            .with_context(|| format!("create scenario root {}", scenario_root.display()))?;
        reports.push(run_scenario(spec, &scenario_root)?);
    }

    let report = BaselineReport {
        tool: "runtime_baseline",
        reports,
    };
    let json = serde_json::to_string_pretty(&report)?;
    let markdown = render_markdown(&report);

    if let Some(path) = args.json_out.as_deref() {
        fs::write(path, &json).with_context(|| format!("write json report {}", path.display()))?;
    }
    if let Some(path) = args.markdown_out.as_deref() {
        fs::write(path, &markdown)
            .with_context(|| format!("write markdown report {}", path.display()))?;
    }

    println!("{json}");
    Ok(())
}

fn run_scenario(spec: ScenarioSpec, root: &Path) -> Result<ScenarioReport> {
    let corpus_dir = root.join("corpus");
    let queue_dir = root.join("queue_payloads");
    write_corpus(&corpus_dir, spec.file_count, "baseline")?;
    write_corpus(&queue_dir, spec.queue_events, "queue")?;
    let queries = build_queries(spec.query_count);
    let event_batch = build_events(spec.file_count)?;
    let event_batch_count = event_batch.len();

    let app = AxiomSync::new(root)?;
    app.initialize()?;
    let baseline_db_bytes = context_db_bytes(root)?;
    app.add_resource(
        corpus_dir.to_str().context("corpus dir utf8")?,
        Some(SEARCH_SCOPE_URI),
        None,
        None,
        true,
        None,
    )?;
    let event_batch_ingest_ms = timed_ms(|| {
        let _ = app.add_events(event_batch)?;
        Ok(())
    })?;
    let context_db_bytes = context_db_bytes(root)?;
    let db_growth_bytes = context_db_bytes.saturating_sub(baseline_db_bytes);
    drop(app);

    let cold_boot_ms = timed_ms(|| {
        let app = AxiomSync::new(root)?;
        app.prepare_runtime()?;
        Ok(())
    })?;
    let warm_boot_ms = timed_ms(|| {
        let app = AxiomSync::new(root)?;
        app.prepare_runtime()?;
        Ok(())
    })?;

    let runtime = AxiomSync::new(root)?;
    runtime.prepare_runtime()?;

    let first_search_ms = timed_ms(|| {
        let _ = runtime.find(
            queries[0].as_str(),
            Some(SEARCH_SCOPE_URI),
            Some(SEARCH_LIMIT),
            None,
            None,
        )?;
        Ok(())
    })?;

    let mut steady_samples = Vec::with_capacity(queries.len());
    for query in &queries {
        steady_samples.push(timed_ms(|| {
            let _ = runtime.find(
                query.as_str(),
                Some(SEARCH_SCOPE_URI),
                Some(SEARCH_LIMIT),
                None,
                None,
            )?;
            Ok(())
        })?);
    }
    steady_samples.sort_unstable();

    let full_reindex_ms = timed_ms(|| {
        runtime.reindex_all()?;
        Ok(())
    })?;

    for idx in 0..spec.queue_events {
        let source = queue_dir.join(format!("queue-{idx:03}.md"));
        let target = format!("{QUEUE_SCOPE_PREFIX}/{idx:03}");
        runtime.add_resource(
            source.to_str().context("queue source utf8")?,
            Some(&target),
            None,
            None,
            false,
            None,
        )?;
    }
    let queue_started = Instant::now();
    let replay = runtime.replay_outbox(spec.queue_events.saturating_mul(4), false)?;
    let queue_replay_ms = queue_started.elapsed().as_millis();
    let queue_replay_events_per_sec = if queue_replay_ms == 0 {
        replay.processed as f64
    } else {
        replay.processed as f64 / (queue_replay_ms as f64 / 1000.0)
    };

    Ok(ScenarioReport {
        scenario: spec.name.to_string(),
        corpus_files: spec.file_count,
        event_batch_count,
        event_batch_ingest_ms,
        cold_boot_ms,
        warm_boot_ms,
        full_reindex_ms,
        first_search_ms,
        steady_search_p50_ms: percentile(&steady_samples, 50),
        steady_search_p95_ms: percentile(&steady_samples, 95),
        context_db_bytes,
        db_growth_bytes,
        queue_replay_processed: replay.processed,
        queue_replay_ms,
        queue_replay_events_per_sec,
    })
}

fn write_corpus(dir: &Path, count: usize, prefix: &str) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("create corpus dir {}", dir.display()))?;
    for idx in 0..count {
        let token = format!("{prefix}-token-{idx:03}");
        let body = format!(
            "# {prefix} {idx}\n\n{token}\n\ncontext manager baseline measurement {token}\n"
        );
        let path = dir.join(format!("{prefix}-{idx:03}.md"));
        fs::write(&path, body).with_context(|| format!("write corpus file {}", path.display()))?;
    }
    Ok(())
}

fn build_queries(count: usize) -> Vec<String> {
    (0..count)
        .map(|idx| format!("baseline-token-{idx:03}"))
        .collect()
}

fn build_events(count: usize) -> Result<Vec<AddEventRequest>> {
    let namespace = NamespaceKey::parse("baseline/runtime")?;
    let incident_kind = Kind::new("incident")?;
    (0..count)
        .map(|idx| {
            Ok(AddEventRequest {
                event_id: format!("evt-{idx:03}"),
                uri: AxiomUri::parse(&format!("axiom://events/baseline/runtime/{idx:03}"))?,
                namespace: namespace.clone(),
                kind: incident_kind.clone(),
                event_time: 1_710_000_000 + idx as i64,
                title: Some(format!("Baseline incident {idx:03}")),
                summary_text: Some(format!(
                    "baseline-token-{idx:03} event ingest measurement for runtime baseline"
                )),
                severity: Some("low".to_string()),
                actor_uri: None,
                subject_uri: None,
                run_id: Some(format!("run-{idx:03}")),
                session_id: None,
                tags: vec!["baseline".to_string(), "incident".to_string()],
                attrs: serde_json::json!({
                    "scenario": "runtime_baseline",
                    "ordinal": idx,
                }),
                object_uri: None,
                content_hash: None,
                created_at: Some(1_710_000_100 + idx as i64),
            })
        })
        .collect::<Result<Vec<_>>>()
}

fn context_db_bytes(root: &Path) -> Result<u64> {
    Ok(fs::metadata(root.join("context.db"))
        .with_context(|| format!("read context.db size under {}", root.display()))?
        .len())
}

fn timed_ms<T>(f: impl FnOnce() -> Result<T>) -> Result<u128> {
    let started = Instant::now();
    let _ = f()?;
    Ok(started.elapsed().as_millis())
}

fn percentile(sorted: &[u128], pct: usize) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let capped = pct.min(100);
    let idx = ((sorted.len() - 1) * capped) / 100;
    sorted[idx]
}

fn render_markdown(report: &BaselineReport) -> String {
    let mut out = String::from("# Runtime Baseline\n\n");
    out.push_str("| Scenario | Corpus | Event batch | Event ingest ms | Cold boot ms | Warm boot ms | Reindex ms | First search ms | Steady p50 ms | Steady p95 ms | DB bytes | DB growth bytes | Queue replay eps |\n");
    out.push_str("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for row in &report.reports {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.2} |\n",
            row.scenario,
            row.corpus_files,
            row.event_batch_count,
            row.event_batch_ingest_ms,
            row.cold_boot_ms,
            row.warm_boot_ms,
            row.full_reindex_ms,
            row.first_search_ms,
            row.steady_search_p50_ms,
            row.steady_search_p95_ms,
            row.context_db_bytes,
            row.db_growth_bytes,
            row.queue_replay_events_per_sec,
        ));
    }
    out
}

fn resolve_scenarios(name: Option<&str>) -> Result<Vec<ScenarioSpec>> {
    match name.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(SCENARIOS.to_vec()),
        Some("all") => Ok(SCENARIOS.to_vec()),
        Some(value) => SCENARIOS
            .iter()
            .copied()
            .find(|spec| spec.name == value)
            .map(|spec| vec![spec])
            .ok_or_else(|| anyhow::anyhow!("unknown scenario: {value}")),
    }
}

struct Args {
    root: Option<PathBuf>,
    scenario: Option<String>,
    json_out: Option<PathBuf>,
    markdown_out: Option<PathBuf>,
}

impl Args {
    fn parse() -> Result<Self> {
        let mut root = None;
        let mut scenario = None;
        let mut json_out = None;
        let mut markdown_out = None;
        let mut iter = env::args().skip(1);

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--root" => root = Some(next_path(&mut iter, "--root")?),
                "--scenario" => scenario = Some(next_string(&mut iter, "--scenario")?),
                "--json-out" => json_out = Some(next_path(&mut iter, "--json-out")?),
                "--markdown-out" => markdown_out = Some(next_path(&mut iter, "--markdown-out")?),
                "-h" | "--help" => {
                    print_help();
                    std::process::exit(0);
                }
                other => bail!("unknown argument: {other}"),
            }
        }

        Ok(Self {
            root,
            scenario,
            json_out,
            markdown_out,
        })
    }
}

fn next_string(iter: &mut impl Iterator<Item = String>, flag: &str) -> Result<String> {
    iter.next()
        .with_context(|| format!("missing value for {flag}"))
}

fn next_path(iter: &mut impl Iterator<Item = String>, flag: &str) -> Result<PathBuf> {
    Ok(PathBuf::from(next_string(iter, flag)?))
}

fn print_help() {
    println!(
        "Usage: cargo run -p axiomsync --bin runtime_baseline -- [--root <path>] [--scenario <small|medium|stress|all>] [--json-out <path>] [--markdown-out <path>]"
    );
}
