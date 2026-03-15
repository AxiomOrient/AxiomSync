use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod args;
mod benchmark;
mod document;
mod eval;
mod event;
mod link;
mod ontology;
mod parsers;
mod queue;
mod relation;
mod release;
mod repo;
mod security;
mod session;
mod trace;

#[cfg(test)]
mod tests;

pub use args::{
    AddArgs, AddWaitModeArg, ExportArgs, FindArgs, GlobArgs, ImportArgs, ListArgs, MoveArgs,
    ReconcileArgs, RemoveArgs, SearchArgs, UriArg, WebArgs,
};
pub use benchmark::{BenchmarkArgs, BenchmarkCommand, BenchmarkFixtureCommand};
pub use document::{DocumentArgs, DocumentCommand, DocumentMode};
pub use eval::{EvalArgs, EvalCommand, EvalGoldenCommand};
pub use event::{EventArgs, EventCommand};
pub use link::{LinkArgs, LinkCommand};
pub use ontology::{OntologyArgs, OntologyCommand};
pub use queue::{QueueArgs, QueueCommand};
pub use relation::{RelationArgs, RelationCommand};
pub use release::{ReleaseArgs, ReleaseCommand, ReleaseSecurityAuditModeArg};
pub use repo::{RepoArgs, RepoCommand};
pub use security::{SecurityArgs, SecurityAuditModeArg, SecurityCommand};
pub use session::{SessionArgs, SessionCommand};
pub use trace::{TraceArgs, TraceCommand};

#[derive(Debug, Parser)]
#[command(name = "axiomsync")]
#[command(about = "Personal AxiomSync context database", version)]
pub struct Cli {
    #[arg(long, default_value = ".axiomsync")]
    pub root: PathBuf,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Init,
    Add(AddArgs),
    Ls(ListArgs),
    Glob(GlobArgs),
    Read(UriArg),
    Abstract(UriArg),
    Overview(UriArg),
    Mkdir(UriArg),
    Rm(RemoveArgs),
    Mv(MoveArgs),
    Tree(UriArg),
    Document(DocumentArgs),
    Find(FindArgs),
    Search(SearchArgs),
    Repo(RepoArgs),
    Event(EventArgs),
    Link(LinkArgs),
    Backend,
    Queue(QueueArgs),
    Trace(TraceArgs),
    Eval(EvalArgs),
    Ontology(OntologyArgs),
    Relation(RelationArgs),
    Benchmark(BenchmarkArgs),
    Security(SecurityArgs),
    Release(ReleaseArgs),
    Reconcile(ReconcileArgs),
    Session(SessionArgs),
    ExportOvpack(ExportArgs),
    ImportOvpack(ImportArgs),
    Web(WebArgs),
}
