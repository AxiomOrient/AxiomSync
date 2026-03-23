pub use axiomsync_cli::Cli;

pub fn run(cli: Cli) -> anyhow::Result<()> {
    axiomsync_cli::run_with(cli, |root| Ok(crate::open(root)?))
}
