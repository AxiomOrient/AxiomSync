use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::domain::AuthSnapshot;
use crate::error::Result;
use crate::ports::AuthStorePort;

#[derive(Debug, Clone)]
pub struct AuthStore {
    root: PathBuf,
}

impl AuthStore {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    #[must_use]
    pub fn path(&self) -> PathBuf {
        self.root.join("auth.json")
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn read(&self) -> Result<AuthSnapshot> {
        match fs::read(self.path()) {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(AuthSnapshot::empty()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn write(&self, snapshot: &AuthSnapshot) -> Result<()> {
        let path = self.path();
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_vec_pretty(snapshot)?)?;
        fs::rename(tmp, path)?;
        Ok(())
    }
}

impl AuthStorePort for AuthStore {
    fn root(&self) -> &Path {
        self.root()
    }

    fn path(&self) -> PathBuf {
        self.path()
    }

    fn read(&self) -> Result<AuthSnapshot> {
        self.read()
    }

    fn write(&self, snapshot: &AuthSnapshot) -> Result<()> {
        self.write(snapshot)
    }
}
