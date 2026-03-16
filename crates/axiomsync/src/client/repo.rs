use std::ffi::OsStr;
use std::io::Read;
use std::path::Path;

use chrono::Utc;
use walkdir::WalkDir;

use crate::error::{AxiomError, Result};
use crate::models::{
    AddResourceRequest, IngestProfile, RepoMountReport, RepoMountRequest, ResourceRecord,
    UpsertResource,
};

use super::AxiomSync;

pub(super) struct RepoService<'a> {
    app: &'a AxiomSync,
}

impl<'a> RepoService<'a> {
    pub(super) fn new(app: &'a AxiomSync) -> Self {
        Self { app }
    }

    pub(super) fn mount_repo(&self, req: RepoMountRequest) -> Result<RepoMountReport> {
        let source_path = Path::new(&req.source_path);
        if !source_path.exists() || !source_path.is_dir() {
            return Err(AxiomError::Validation(format!(
                "repo mount source must be an existing directory: {}",
                req.source_path
            )));
        }

        let add_result = self
            .app
            .add_resource_with_ingest_options(AddResourceRequest {
                source: req.source_path.clone(),
                target: Some(req.target_uri.to_string()),
                wait: req.wait,
                timeout_secs: None,
                wait_mode: crate::models::AddResourceWaitMode::Relaxed,
                ingest_options: crate::models::AddResourceIngestOptions::default(),
            })?;

        let now = Utc::now().timestamp();
        let resource_id = Self::resource_id_for_uri(&req.target_uri);
        let repo_object_uri =
            self.write_resource_mount_object(&req.target_uri, &req.namespace, &req.kind, &req)?;
        let resource = ResourceRecord {
            resource_id,
            uri: req.target_uri.clone(),
            namespace: req.namespace,
            kind: req.kind,
            title: req.title.clone().or_else(|| {
                source_path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .map(ToString::to_string)
            }),
            mime: None,
            tags: req.tags,
            attrs: req.attrs,
            object_uri: Some(repo_object_uri),
            excerpt_text: req.title,
            content_hash: compute_repo_tree_digest(source_path)?,
            tombstoned_at: None,
            created_at: now,
            updated_at: now,
        };
        self.app.state.persist_resource(UpsertResource {
            resource_id: resource.resource_id.clone(),
            uri: resource.uri.clone(),
            namespace: resource.namespace.clone(),
            kind: resource.kind.clone(),
            title: resource.title.clone(),
            mime: resource.mime.clone(),
            tags: resource.tags.clone(),
            attrs: resource.attrs.clone(),
            object_uri: resource.object_uri.clone(),
            excerpt_text: resource.excerpt_text.clone(),
            content_hash: resource.content_hash.clone(),
            tombstoned_at: None,
            created_at: resource.created_at,
            updated_at: resource.updated_at,
        })?;
        self.persist_resource_and_sync_index(&resource)?;

        Ok(RepoMountReport {
            root_uri: req.target_uri,
            resource,
            queued: add_result.queued,
        })
    }

    fn persist_resource_and_sync_index(&self, resource: &ResourceRecord) -> Result<()> {
        let profile = IngestProfile::for_kind(&resource.kind);
        self.app
            .state
            .persist_resource_search_document(resource, &profile)?;
        self.sync_runtime_index(&resource.uri.to_string())
    }

    fn sync_runtime_index(&self, uri: &str) -> Result<()> {
        let record = self.app.state.get_search_document(uri)?;
        let mut index = self
            .app
            .index
            .write()
            .map_err(|_| AxiomError::lock_poisoned("index"))?;
        index.remove(uri);
        if let Some(record) = record {
            index.upsert(record);
        }
        Ok(())
    }

    fn write_resource_mount_object(
        &self,
        target_uri: &crate::AxiomUri,
        namespace: &crate::models::NamespaceKey,
        kind: &crate::models::Kind,
        req: &RepoMountRequest,
    ) -> Result<crate::AxiomUri> {
        let resource_id = Self::resource_id_for_uri(target_uri);
        let object_uri = self
            .resource_object_uri(namespace, kind, &resource_id)
            .map_err(|err| {
                AxiomError::Internal(format!("failed to resolve resource object uri: {err}"))
            })?;
        let payload = serde_json::to_vec_pretty(&serde_json::json!({
            "source_path": req.source_path,
            "target_uri": target_uri.to_string(),
            "namespace": namespace.as_path(),
            "kind": kind.as_str(),
            "title": req.title,
            "tags": req.tags,
            "attrs": req.attrs,
        }))?;
        self.app.fs.write_bytes(&object_uri, &payload, true)?;
        Ok(object_uri)
    }

    fn resource_id_for_uri(target_uri: &crate::AxiomUri) -> String {
        blake3::hash(target_uri.to_string().as_bytes())
            .to_hex()
            .to_string()
    }

    fn resource_object_uri(
        &self,
        namespace: &crate::models::NamespaceKey,
        kind: &crate::models::Kind,
        resource_id: &str,
    ) -> std::result::Result<crate::AxiomUri, crate::AxiomError> {
        let mut uri = crate::AxiomUri::root(crate::Scope::Resources)
            .join("_objects")?
            .join(&namespace.as_path())?
            .join(kind.as_str())?;
        uri = uri.join(&format!("{resource_id}.json"))?;
        Ok(uri)
    }
}

impl AxiomSync {
    pub fn mount_repo(&self, req: RepoMountRequest) -> Result<RepoMountReport> {
        self.repo_service().mount_repo(req)
    }
}

/// Directories excluded from the repository tree digest.
///
/// VCS metadata, AxiomSync runtime state, and common build artifact directories are not
/// stable repository content. Excluding them ensures `content_hash` reflects only the
/// authored working-tree files, making repo identity stable across Git operations and
/// local build state changes.
const IGNORED_DIR_NAMES: &[&str] = &[
    ".git",
    ".axiomsync",
    "target",
    "node_modules",
    ".hg",
    ".svn",
];

/// Tier-generated files excluded from the repository tree digest.
const EXCLUDED_FILE_NAMES: &[&str] = &[".abstract.md", ".overview.md"];

/// Pure predicate: returns true if `name` is an ignored directory name.
fn is_ignored_name(name: &OsStr) -> bool {
    IGNORED_DIR_NAMES.iter().any(|&s| name == s)
}

/// Returns true if this directory entry should be pruned from the tree walk.
fn is_ignored_dir(entry: &walkdir::DirEntry) -> bool {
    entry.file_type().is_dir() && is_ignored_name(entry.file_name())
}

/// Computes a stable blake3 digest of the repository tree rooted at `source_path`.
///
/// The digest is derived from the sorted set of (relative path, file content) pairs,
/// making it stable for identical content regardless of absolute path and deterministic
/// across mounts with the same files.
///
/// Excluded from the digest:
/// - VCS metadata directories (`.git/`, `.hg/`, `.svn/`)
/// - AxiomSync runtime state (`.axiomsync/`)
/// - Build artifact directories (`target/`, `node_modules/`)
/// - Tier-generated files (`.abstract.md`, `.overview.md`)
fn compute_repo_tree_digest(source_path: &Path) -> Result<String> {
    // Pass 1: collect (rel_path, abs_path, file_len) triples sorted by rel_path.
    // file_len is taken from WalkDir's cached DirEntry metadata to avoid a
    // redundant fstat per file in Pass 2.
    let mut entries: Vec<(String, std::path::PathBuf, u64)> = Vec::new();

    for entry in WalkDir::new(source_path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_ignored_dir(e))
    {
        let entry =
            entry.map_err(|e| AxiomError::Validation(format!("repo tree walk error: {e}")))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name();
        if EXCLUDED_FILE_NAMES.iter().any(|&s| name == s) {
            continue;
        }
        let file_len = entry
            .metadata()
            .map_err(|e| AxiomError::Internal(format!("repo tree digest metadata error: {e}")))?
            .len();
        let rel_path = entry
            .path()
            .strip_prefix(source_path)
            .map_err(|e| AxiomError::Internal(format!("repo tree digest path error: {e}")))?
            .to_string_lossy()
            .into_owned();
        entries.push((rel_path, entry.path().to_owned(), file_len));
    }

    entries.sort_unstable_by(|a, b| a.0.cmp(&b.0));

    // Pass 2: stream each file through the hasher in sorted path order.
    // Read directly into a single stack buffer; no BufReader needed with a 64 KiB buffer.
    let mut hasher = blake3::Hasher::new();
    let mut buf = [0u8; 65536];
    for (rel_path, abs_path, file_len) in &entries {
        let mut file = std::fs::File::open(abs_path)
            .map_err(|e| AxiomError::Internal(format!("repo tree digest open error: {e}")))?;
        hasher.update(rel_path.as_bytes());
        hasher.update(b"\x00");
        hasher.update(&file_len.to_le_bytes());
        loop {
            let n = file
                .read(&mut buf)
                .map_err(|e| AxiomError::Internal(format!("repo tree digest read error: {e}")))?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
    }
    Ok(hasher.finalize().to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::compute_repo_tree_digest;

    #[test]
    fn repo_tree_digest_is_stable_for_identical_content() {
        let temp = tempdir().expect("tempdir");
        let repo_a = temp.path().join("repo-a");
        let repo_b = temp.path().join("repo-b");
        fs::create_dir_all(&repo_a).expect("mkdir a");
        fs::create_dir_all(&repo_b).expect("mkdir b");
        fs::write(repo_a.join("README.md"), "# Same Content").expect("write a");
        fs::write(repo_b.join("README.md"), "# Same Content").expect("write b");

        let hash_a = compute_repo_tree_digest(&repo_a).expect("digest a");
        let hash_b = compute_repo_tree_digest(&repo_b).expect("digest b");

        assert_eq!(hash_a, hash_b, "identical content must yield identical digest");
    }

    #[test]
    fn repo_tree_digest_changes_when_content_mutates() {
        let temp = tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        fs::create_dir_all(&repo).expect("mkdir");
        fs::write(repo.join("README.md"), "# Version 1").expect("write v1");

        let hash_v1 = compute_repo_tree_digest(&repo).expect("digest v1");

        fs::write(repo.join("README.md"), "# Version 2").expect("write v2");

        let hash_v2 = compute_repo_tree_digest(&repo).expect("digest v2");

        assert_ne!(hash_v1, hash_v2, "content change must change the digest");
    }

    #[test]
    fn repo_tree_digest_changes_when_file_added() {
        let temp = tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        fs::create_dir_all(&repo).expect("mkdir");
        fs::write(repo.join("a.md"), "# A").expect("write a");

        let hash_before = compute_repo_tree_digest(&repo).expect("digest before");

        fs::write(repo.join("b.md"), "# B").expect("write b");

        let hash_after = compute_repo_tree_digest(&repo).expect("digest after");

        assert_ne!(
            hash_before, hash_after,
            "adding a file must change the digest"
        );
    }

    #[test]
    fn repo_tree_digest_is_not_path_string_hash() {
        let temp = tempdir().expect("tempdir");
        let repo = temp.path().join("my-repo");
        fs::create_dir_all(&repo).expect("mkdir");
        fs::write(repo.join("README.md"), "# Hello").expect("write");

        let tree_digest = compute_repo_tree_digest(&repo).expect("tree digest");
        let path_string_hash = blake3::hash(repo.to_string_lossy().as_bytes())
            .to_hex()
            .to_string();

        assert_ne!(
            tree_digest, path_string_hash,
            "tree digest must differ from path-string hash"
        );
    }

    #[test]
    fn repo_tree_digest_excludes_git_metadata() {
        let temp = tempdir().expect("tempdir");
        let repo_a = temp.path().join("repo-a");
        let repo_b = temp.path().join("repo-b");
        fs::create_dir_all(&repo_a).expect("mkdir a");
        fs::create_dir_all(&repo_b).expect("mkdir b");
        fs::write(repo_a.join("README.md"), "# Same").expect("write a");
        fs::write(repo_b.join("README.md"), "# Same").expect("write b");

        // repo_b has a .git/ directory with internal state — must not affect the digest.
        let git_dir = repo_b.join(".git");
        fs::create_dir_all(&git_dir).expect("mkdir .git");
        fs::write(git_dir.join("HEAD"), "ref: refs/heads/main").expect("write HEAD");
        fs::write(git_dir.join("ORIG_HEAD"), "abc123").expect("write ORIG_HEAD");

        let hash_a = compute_repo_tree_digest(&repo_a).expect("digest a");
        let hash_b = compute_repo_tree_digest(&repo_b).expect("digest b");
        assert_eq!(hash_a, hash_b, ".git/ contents must not affect the digest");
    }

    #[test]
    fn repo_tree_digest_excludes_axiomsync_state() {
        let temp = tempdir().expect("tempdir");
        let repo_a = temp.path().join("repo-a");
        let repo_b = temp.path().join("repo-b");
        fs::create_dir_all(&repo_a).expect("mkdir a");
        fs::create_dir_all(&repo_b).expect("mkdir b");
        fs::write(repo_a.join("doc.md"), "content").expect("write a");
        fs::write(repo_b.join("doc.md"), "content").expect("write b");

        // repo_b has an .axiomsync/ runtime state directory — must not affect the digest.
        let state_dir = repo_b.join(".axiomsync");
        fs::create_dir_all(&state_dir).expect("mkdir .axiomsync");
        fs::write(state_dir.join("context.db"), "runtime state").expect("write db");

        let hash_a = compute_repo_tree_digest(&repo_a).expect("digest a");
        let hash_b = compute_repo_tree_digest(&repo_b).expect("digest b");
        assert_eq!(
            hash_a, hash_b,
            ".axiomsync/ contents must not affect the digest"
        );
    }
}
