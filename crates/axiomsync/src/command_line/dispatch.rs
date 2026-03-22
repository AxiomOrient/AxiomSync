use super::*;

pub fn run(cli: Cli) -> Result<()> {
    let app = crate::open(cli.root)?;
    match cli.command {
        Command::Init => print_json(&app.init()?)?,
        Command::Connector(args) => run_connector(app, args)?,
        Command::Project(args) => run_project(app, args)?,
        Command::Derive(args) => run_derive(app, args)?,
        Command::Search(args) => run_search(app, args)?,
        Command::Runbook(args) => {
            print_json(&serde_json::to_value(app.get_runbook(&args.episode_id)?)?)?
        }
        Command::Mcp(args) => match args.command {
            McpCommand::Serve {
                transport,
                addr,
                workspace_id,
            } => match transport {
                McpTransport::Stdio => runtime::build_runtime()?
                    .block_on(mcp::serve_stdio(app, workspace_id.as_deref()))?,
                McpTransport::Http => {
                    runtime::build_runtime()?.block_on(http_api::serve(app, addr))?
                }
            },
        },
        Command::Web(args) => {
            runtime::build_runtime()?.block_on(http_api::serve(app, args.addr))?
        }
    }
    Ok(())
}

fn run_connector(app: AxiomSync, args: ConnectorArgs) -> Result<()> {
    match args.command {
        ConnectorCommand::Ingest(args) => {
            let adapter = ConnectorAdapter::from_connector_label(&args.connector);
            let batch = adapter.load_batch(
                args.file.as_deref(),
                args.cursor_key,
                args.cursor_value,
                args.cursor_ts_ms,
            )?;
            runtime::apply_batch(app, batch, args.dry_run)?;
        }
        ConnectorCommand::Sync(args) => {
            let adapter = ConnectorAdapter::from_connector_name(args.connector);
            let batch = adapter.sync_batch(&app)?;
            runtime::apply_batch(app, batch, args.dry_run)?;
        }
        ConnectorCommand::Repair(args) => {
            let adapter = ConnectorAdapter::from_connector_name(args.connector);
            let batch = adapter.repair_batch(args.dir.as_deref())?;
            let plan = app.plan_repair(&batch)?;
            if args.dry_run {
                print_json(&serde_json::to_value(plan)?)?;
            } else {
                print_json(&serde_json::json!({
                    "plan": plan,
                    "applied": app.apply_repair(&plan)?,
                }))?;
            }
        }
        ConnectorCommand::Watch(args) => {
            let adapter = ConnectorAdapter::from_connector_name(args.connector);
            adapter.watch_batch(app, args.dry_run, args.once)?;
        }
        ConnectorCommand::Serve(args) => {
            let adapter = ConnectorAdapter::from_connector_name(args.connector);
            runtime::build_runtime()?.block_on(adapter.serve_connector_ingest(app, args.addr))?;
        }
    }
    Ok(())
}

fn run_project(app: AxiomSync, args: ProjectArgs) -> Result<()> {
    match args.command {
        ProjectCommand::Rebuild { dry_run } => {
            let plan = app.plan_replay()?;
            if dry_run {
                print_json(&serde_json::to_value(plan)?)?;
            } else {
                print_json(&serde_json::json!({
                    "plan": plan,
                    "applied": app.apply_replay(&plan)?,
                }))?;
            }
        }
        ProjectCommand::Purge {
            connector,
            workspace_id,
            dry_run,
        } => {
            let plan = app.plan_purge(connector.as_deref(), workspace_id.as_deref())?;
            if dry_run {
                print_json(&serde_json::to_value(plan)?)?;
            } else {
                print_json(&serde_json::json!({
                    "plan": plan,
                    "applied": app.apply_purge(&plan)?,
                }))?;
            }
        }
        ProjectCommand::Doctor => {
            print_json(&serde_json::to_value(app.doctor()?)?)?;
        }
        ProjectCommand::AuthGrant {
            workspace_root,
            token,
            dry_run,
        } => {
            let plan = app.plan_workspace_token_grant(&workspace_root, &token)?;
            if dry_run {
                print_json(&serde_json::to_value(plan)?)?;
            } else {
                print_json(&serde_json::json!({
                    "plan": plan,
                    "applied": app.apply_workspace_token_grant(&plan)?,
                }))?;
            }
        }
    }
    Ok(())
}

fn run_derive(app: AxiomSync, args: DeriveArgs) -> Result<()> {
    let plan = app.plan_derivation()?;
    if args.dry_run {
        print_json(&serde_json::to_value(plan)?)?;
    } else {
        print_json(&serde_json::json!({
            "plan": plan,
            "applied": app.apply_derivation(&plan)?,
        }))?;
    }
    Ok(())
}

fn run_search(app: AxiomSync, args: SearchArgs) -> Result<()> {
    if args.commands {
        return Ok(print_json(&serde_json::to_value(
            app.search_commands(&args.query, args.limit)?,
        )?)?);
    }
    let rows = app.search_episodes(SearchEpisodesRequest {
        query: args.query,
        limit: args.limit,
        filter: SearchEpisodesFilter {
            connector: args.connector,
            workspace_id: args.workspace_id,
            status: args
                .status
                .as_deref()
                .map(EpisodeStatus::parse)
                .transpose()?,
        },
    })?;
    Ok(print_json(&serde_json::to_value(rows)?)?)
}
