// compile-oriented skeleton, not build-verified

use std::path::PathBuf;
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct SinkArgs {
    #[command(subcommand)]
    pub command: SinkCommand,
}

#[derive(Debug, Subcommand)]
pub enum SinkCommand {
    PlanAppendRawEvents {
        #[arg(long)]
        file: PathBuf,
    },
    ApplyIngestPlan {
        #[arg(long)]
        file: PathBuf,
    },
    PlanUpsertSourceCursor {
        #[arg(long)]
        file: PathBuf,
    },
    ApplySourceCursorPlan {
        #[arg(long)]
        file: PathBuf,
    },
}

pub fn run_sink_command(app: &AxiomSync, args: SinkArgs) -> anyhow::Result<()> {
    match args.command {
        SinkCommand::PlanAppendRawEvents { file } => {
            let input: ConnectorBatchInput = serde_json::from_slice(&std::fs::read(file)?)?;
            let plan = app.plan_append_raw_events(&input)?;
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
        SinkCommand::ApplyIngestPlan { file } => {
            let plan: IngestPlan = serde_json::from_slice(&std::fs::read(file)?)?;
            let applied = app.apply_ingest_plan(&plan)?;
            println!("{}", serde_json::to_string_pretty(&applied)?);
        }
        SinkCommand::PlanUpsertSourceCursor { file } => {
            let input: SourceCursorInput = serde_json::from_slice(&std::fs::read(file)?)?;
            let plan = app.plan_upsert_source_cursor(&input)?;
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
        SinkCommand::ApplySourceCursorPlan { file } => {
            let plan: SourceCursorUpsertPlan = serde_json::from_slice(&std::fs::read(file)?)?;
            let applied = app.apply_source_cursor_plan(&plan)?;
            println!("{}", serde_json::to_string_pretty(&applied)?);
        }
    }
    Ok(())
}
