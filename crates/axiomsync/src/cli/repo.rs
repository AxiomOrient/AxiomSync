use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct RepoArgs {
    #[command(subcommand)]
    pub command: RepoCommand,
}

#[derive(Debug, Subcommand)]
pub enum RepoCommand {
    Mount {
        source_path: String,
        #[arg(long)]
        target_uri: String,
        #[arg(long)]
        namespace: String,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long = "tag")]
        tags: Vec<String>,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        wait: bool,
    },
}
