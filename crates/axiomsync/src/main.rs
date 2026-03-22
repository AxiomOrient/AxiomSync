use anyhow::Result;
use clap::Parser;

use axiomsync::command_line::{Cli, run};

fn main() -> Result<()> {
    let cli = Cli::parse();
    run(cli)
}
