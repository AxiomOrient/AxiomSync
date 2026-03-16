use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct LinkArgs {
    #[command(subcommand)]
    pub command: LinkCommand,
}

#[derive(Debug, Subcommand)]
pub enum LinkCommand {
    Add {
        #[arg(long)]
        link_id: String,
        #[arg(long)]
        namespace: String,
        #[arg(long)]
        from_uri: String,
        #[arg(long)]
        relation: String,
        #[arg(long)]
        to_uri: String,
        #[arg(long, default_value_t = 1.0)]
        weight: f32,
    },
    List {
        #[arg(long)]
        namespace: Option<String>,
        #[arg(long)]
        from_uri: Option<String>,
        #[arg(long)]
        to_uri: Option<String>,
        #[arg(long)]
        relation: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
    },
}
