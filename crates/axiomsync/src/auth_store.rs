use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::domain::AuthSnapshot;
use crate::error::Result;
use crate::ports::AuthStorePort;

#[cfg(unix)]
use std::fs::OpenOptions;
#[cfg(unix)]
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

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
        write_auth_snapshot(&tmp, snapshot)?;
        fs::rename(tmp, path)?;
        #[cfg(unix)]
        fs::set_permissions(self.path(), fs::Permissions::from_mode(0o600))?;
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

#[cfg(unix)]
fn write_auth_snapshot(path: &Path, snapshot: &AuthSnapshot) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(&serde_json::to_vec_pretty(snapshot)?)?;
    file.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn write_auth_snapshot(path: &Path, snapshot: &AuthSnapshot) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(snapshot)?)?;
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn auth_snapshot_reads_legacy_file_without_admin_tokens() {
        let temp = tempdir().expect("tempdir");
        let store = AuthStore::open(temp.path()).expect("store");
        fs::write(
            store.path(),
            r#"{"schema_version":"renewal-sqlite-v1","grants":[{"workspace_id":"ws_1","token_sha256":"hash"}]}"#,
        )
        .expect("write legacy snapshot");

        let snapshot = store.read().expect("read");

        assert_eq!(snapshot.grants.len(), 1);
        assert!(snapshot.admin_tokens.is_empty());
    }

    #[test]
    fn auth_snapshot_is_written_with_owner_only_permissions() {
        let temp = tempdir().expect("tempdir");
        let store = AuthStore::open(temp.path()).expect("store");

        store.write(&AuthSnapshot::empty()).expect("write");

        let mode = fs::metadata(store.path())
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }
}
