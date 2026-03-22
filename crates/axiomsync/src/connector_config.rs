use std::fs;
use std::path::PathBuf;

use crate::domain::ConnectorsConfig;
use crate::error::{AxiomError, Result};
use crate::ports::ConnectorConfigPort;

#[derive(Debug, Clone)]
pub struct FileConnectorConfigStore {
    root: PathBuf,
}

impl FileConnectorConfigStore {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    #[must_use]
    pub fn path(&self) -> PathBuf {
        self.root.join("connectors.toml")
    }

    pub fn ensure_default(&self) -> Result<()> {
        let path = self.path();
        if !path.exists() {
            fs::write(path, include_str!("../assets/connectors.example.toml"))?;
        }
        Ok(())
    }

    pub fn load(&self) -> Result<ConnectorsConfig> {
        let path = self.path();
        if !path.exists() {
            return Ok(ConnectorsConfig::default());
        }
        toml::from_str(&fs::read_to_string(path)?)
            .map_err(|error| AxiomError::Internal(format!("invalid connectors.toml: {error}")))
    }
}

impl ConnectorConfigPort for FileConnectorConfigStore {
    fn path(&self) -> PathBuf {
        self.path()
    }

    fn ensure_default(&self) -> Result<()> {
        self.ensure_default()
    }

    fn load(&self) -> Result<ConnectorsConfig> {
        self.load()
    }
}
