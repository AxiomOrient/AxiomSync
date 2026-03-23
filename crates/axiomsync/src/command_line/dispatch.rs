use super::*;

pub fn run(cli: Cli) -> Result<()> {
    let app = crate::open(cli.root)?;
    match cli.command {
        Command::Init => print_json(&app.init()?)?,
        Command::Sink(args) => run_sink(app, args)?,
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

fn run_sink(app: AxiomSync, args: SinkArgs) -> Result<()> {
    match args.command {
        SinkCommand::PlanAppendRawEvents(args) => {
            let request = crate::sink::load_append_request(&args.file)?;
            let batch = app.build_append_batch(&request)?;
            let existing = app.load_existing_raw_event_keys()?;
            let plan = app.plan_ingest(&existing, &batch)?;
            print_json(&serde_json::to_value(plan)?)?;
        }
        SinkCommand::ApplyIngestPlan(args) => {
            let plan = crate::sink::load_json_file::<crate::domain::IngestPlan>(&args.file)?;
            print_json(&app.apply_ingest(&plan)?)?;
        }
        SinkCommand::PlanUpsertSourceCursor(args) => {
            let request = crate::sink::load_cursor_request(&args.file)?;
            let plan = app.plan_source_cursor_upsert(&request)?;
            print_json(&serde_json::to_value(plan)?)?;
        }
        SinkCommand::ApplySourceCursorPlan(args) => {
            let plan =
                crate::sink::load_json_file::<crate::domain::SourceCursorUpsertPlan>(&args.file)?;
            print_json(&app.apply_source_cursor_upsert(&plan)?)?;
        }
    }
    Ok(())
}

fn run_project(app: AxiomSync, args: ProjectArgs) -> Result<()> {
    match args.command {
        ProjectCommand::PlanRebuild => {
            let plan = crate::build_replay_plan(&app)?;
            print_json(&serde_json::to_value(plan)?)?;
        }
        ProjectCommand::ApplyReplayPlan { file } => {
            let plan = crate::sink::load_json_file::<crate::domain::ReplayPlan>(&file)?;
            print_json(&app.apply_replay(&plan)?)?;
        }
        ProjectCommand::PlanPurge {
            source,
            workspace_id,
        } => {
            let plan = crate::build_purge_plan(&app, source.as_deref(), workspace_id.as_deref())?;
            print_json(&serde_json::to_value(plan)?)?;
        }
        ProjectCommand::ApplyPurgePlan { file } => {
            let plan = crate::sink::load_json_file::<crate::domain::PurgePlan>(&file)?;
            print_json(&app.apply_purge(&plan)?)?;
        }
        ProjectCommand::Doctor => {
            print_json(&serde_json::to_value(app.doctor()?)?)?;
        }
        ProjectCommand::PlanAuthGrant {
            workspace_root,
            token,
        } => {
            let plan = app.plan_workspace_token_grant(&workspace_root, &token)?;
            print_json(&serde_json::to_value(plan)?)?;
        }
        ProjectCommand::PlanAdminGrant { token } => {
            let plan = app.plan_admin_token_grant(&token)?;
            print_json(&serde_json::to_value(plan)?)?;
        }
        ProjectCommand::ApplyAuthGrantPlan { file } => {
            let plan = crate::sink::load_json_file::<crate::domain::WorkspaceTokenPlan>(&file)?;
            print_json(&app.apply_workspace_token_grant(&plan)?)?;
        }
        ProjectCommand::ApplyAdminGrantPlan { file } => {
            let plan = crate::sink::load_json_file::<crate::domain::AdminTokenPlan>(&file)?;
            print_json(&app.apply_admin_token_grant(&plan)?)?;
        }
    }
    Ok(())
}

fn run_derive(app: AxiomSync, args: DeriveArgs) -> Result<()> {
    match args.command {
        DeriveCommand::Plan => {
            let plan = crate::build_derivation_plan(&app)?;
            print_json(&serde_json::to_value(plan)?)?;
        }
        DeriveCommand::ApplyPlan { file } => {
            let plan = crate::sink::load_json_file::<crate::domain::DerivePlan>(&file)?;
            print_json(&app.apply_derivation(&plan)?)?;
        }
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
            source: args.source,
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
