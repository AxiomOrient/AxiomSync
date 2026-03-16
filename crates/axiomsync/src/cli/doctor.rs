use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct DoctorArgs {
    #[command(subcommand)]
    pub command: DoctorCommand,
}

#[derive(Debug, Subcommand)]
pub enum DoctorCommand {
    Storage {
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    Retrieval {
        #[arg(long, default_value_t = false)]
        json: bool,
    },
}
