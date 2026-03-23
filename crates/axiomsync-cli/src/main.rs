use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = axiomsync_cli::Cli::parse();
    axiomsync_cli::run_with(cli, |root| {
        let repo = Arc::new(axiomsync_store_sqlite::ContextDb::open(root.clone())?)
            as axiomsync_kernel::ports::SharedRepositoryPort;
        let auth = Arc::new(AuthStore::open(root)?) as axiomsync_kernel::ports::SharedAuthStorePort;
        Ok(axiomsync_kernel::AxiomSync::new(
            repo,
            auth,
            Arc::new(NoopLlm) as axiomsync_kernel::ports::SharedLlmExtractionPort,
        ))
    })
}

#[derive(Debug, Clone)]
struct AuthStore {
    root: PathBuf,
}

impl AuthStore {
    fn open(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    fn path(&self) -> PathBuf {
        self.root.join("auth.json")
    }

    fn read_snapshot(&self) -> Result<axiomsync_domain::domain::AuthSnapshot> {
        match fs::read(self.path()) {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
            Err(error) if error.kind() == ErrorKind::NotFound => {
                Ok(axiomsync_domain::domain::AuthSnapshot::empty())
            }
            Err(error) => Err(error.into()),
        }
    }

    fn write_snapshot(&self, snapshot: &axiomsync_domain::domain::AuthSnapshot) -> Result<()> {
        fs::write(self.path(), serde_json::to_vec_pretty(snapshot)?)?;
        Ok(())
    }
}

impl axiomsync_kernel::ports::AuthStorePort for AuthStore {
    fn root(&self) -> &Path {
        &self.root
    }

    fn path(&self) -> PathBuf {
        self.path()
    }

    fn read(&self) -> axiomsync_domain::Result<axiomsync_domain::domain::AuthSnapshot> {
        self.read_snapshot()
            .map_err(|error| axiomsync_domain::AxiomError::Internal(error.to_string()))
    }

    fn write(
        &self,
        snapshot: &axiomsync_domain::domain::AuthSnapshot,
    ) -> axiomsync_domain::Result<()> {
        self.write_snapshot(snapshot)
            .map_err(|error| axiomsync_domain::AxiomError::Internal(error.to_string()))
    }
}

#[derive(Debug)]
struct NoopLlm;

impl axiomsync_kernel::ports::LlmExtractionPort for NoopLlm {}
