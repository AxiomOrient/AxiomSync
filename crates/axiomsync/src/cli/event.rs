use std::path::PathBuf;

use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct EventArgs {
    #[command(subcommand)]
    pub command: EventCommand,
}

#[derive(Debug, Subcommand)]
pub enum EventCommand {
    Add {
        #[arg(long)]
        event_id: String,
        #[arg(long)]
        uri: String,
        #[arg(long)]
        namespace: String,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        event_time: i64,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        summary: Option<String>,
        #[arg(long)]
        severity: Option<String>,
        #[arg(long)]
        run_id: Option<String>,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long = "tag")]
        tags: Vec<String>,
    },
    Import {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        namespace: String,
        #[arg(long)]
        kind: String,
    },
    Archive {
        #[command(subcommand)]
        command: EventArchiveCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum EventArchiveCommand {
    Plan {
        #[arg(long)]
        archive_id: String,
        #[arg(long)]
        namespace: Option<String>,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        start_time: Option<i64>,
        #[arg(long)]
        end_time: Option<i64>,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long)]
        archive_reason: Option<String>,
        #[arg(long)]
        archived_by: Option<String>,
    },
    Execute {
        #[arg(long)]
        plan_file: PathBuf,
    },
}
