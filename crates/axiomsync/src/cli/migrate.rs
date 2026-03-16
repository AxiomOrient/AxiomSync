use std::path::PathBuf;

use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct MigrateArgs {
    #[command(subcommand)]
    pub command: MigrateCommand,
}

#[derive(Debug, Subcommand)]
pub enum MigrateCommand {
    Inspect {
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    Apply {
        #[arg(long)]
        backup_dir: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
}
