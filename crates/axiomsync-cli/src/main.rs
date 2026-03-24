use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = axiomsync_cli::Cli::parse();
    axiomsync_cli::run_with(cli, axiomsync_cli::open)
}
