use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .map(PathBuf::from)
        .expect("workspace root")
}

fn should_skip_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| matches!(name, ".git" | "target" | ".axiomsync" | "logs"))
}

fn should_skip_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| name == "Cargo.lock")
}

fn blocked_word() -> String {
    String::from_utf8(vec![0x76, 0x69, 0x6b, 0x69, 0x6e, 0x67]).expect("blocked word")
}

fn blocked_prefix() -> String {
    let prefix = String::from_utf8(vec![0x6f, 0x70, 0x65, 0x6e]).expect("blocked prefix");
    format!("{prefix}{}", blocked_word())
}

fn blocked_scheme() -> String {
    format!("{}://", blocked_word())
}

fn contains_standalone_word(haystack: &str, needle: &str) -> bool {
    let bytes = haystack.as_bytes();
    let needle_bytes = needle.as_bytes();
    if needle_bytes.is_empty() || bytes.len() < needle_bytes.len() {
        return false;
    }

    bytes
        .windows(needle_bytes.len())
        .enumerate()
        .any(|(index, window)| {
            if window != needle_bytes {
                return false;
            }
            let left_ok = index == 0
                || (!bytes[index - 1].is_ascii_alphanumeric() && bytes[index - 1] != b'_');
            let right_index = index + needle_bytes.len();
            let right_ok = right_index == bytes.len()
                || (!bytes[right_index].is_ascii_alphanumeric() && bytes[right_index] != b'_');
            left_ok && right_ok
        })
}

fn line_matches_policy(line: &str) -> bool {
    let lowered = line.to_ascii_lowercase();
    let word = blocked_word();
    lowered.contains(&blocked_prefix())
        || lowered.contains(&blocked_scheme())
        || contains_standalone_word(&lowered, &word)
}

#[test]
fn repository_policy_disallows_blocked_tokens() {
    let mut hits = Vec::new();
    let root = workspace_root();

    for entry in WalkDir::new(&root)
        .into_iter()
        .filter_entry(|entry| !should_skip_dir(entry.path()))
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if entry.file_type().is_dir() || should_skip_file(path) {
            continue;
        }

        let Ok(raw) = fs::read_to_string(path) else {
            continue;
        };
        for (line_no, line) in raw.lines().enumerate() {
            if line_matches_policy(line) {
                let relative = path.strip_prefix(&root).unwrap_or(path);
                hits.push(format!("{}:{}", relative.display(), line_no + 1));
            }
        }
    }

    assert!(
        hits.is_empty(),
        "blocked token policy violations:\n{}",
        hits.join("\n")
    );
}
