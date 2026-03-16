//! Frozen ADR path inference rule for AxiomSync v3.
//! This module is intentionally narrow: path-only, deterministic, conservative.
//!
//! Precedence: explicit metadata > path inference > content heuristic.
//! See `plans/AxiomSync_16_ADR_Path_Inference_Spec.md` for full specification.

use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdrInferenceReason {
    StrongDirectoryMarker,
    StrongFilenameMarker,
    NumberedFilenameInsideAdrDirectory,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdrInference {
    pub kind: &'static str,
    pub confidence: f32,
    pub canonical_subpath: String,
    pub reason: AdrInferenceReason,
}

const SUPPORTED_EXTS: &[&str] = &["md", "mdx", "txt", "rst"];
const NEGATIVE_SEGMENTS: &[&str] = &["address", "addresses", "adapter", "adapters", "adrsync"];
const STRONG_DIR_PATTERNS: &[&[&str]] = &[
    &["adr"],
    &["docs", "adr"],
    &["docs", "decisions"],
    &["decisions", "adr"],
    &["decisions", "architecture"],
    &["architecture", "decisions"],
];

/// Infer whether a repo-relative path represents an ADR document.
///
/// Returns `None` if the path does not match any ADR inference rule.
/// When `Some`, the caller should store `inference_reason`, `inference_confidence`,
/// and `source_repo_path` in `attrs_json`.
pub fn infer_adr_from_repo_path(path: &Path) -> Option<AdrInference> {
    let norm = normalize_path(path);
    let segments: Vec<&str> = norm.split('/').filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        return None;
    }
    if segments.iter().any(|s| NEGATIVE_SEGMENTS.contains(s)) {
        return None;
    }

    let file_name = *segments.last()?;
    let (stem, ext) = split_stem_ext(file_name)?;
    if !SUPPORTED_EXTS.contains(&ext) {
        return None;
    }

    let has_strong_dir = contains_dir_marker(&segments);
    let strong_filename = is_strong_filename(stem);
    let numbered_in_adr_dir = has_strong_dir && is_numbered_filename(stem);

    let (confidence, reason) = if has_strong_dir {
        if numbered_in_adr_dir {
            (0.92, AdrInferenceReason::NumberedFilenameInsideAdrDirectory)
        } else {
            (0.95, AdrInferenceReason::StrongDirectoryMarker)
        }
    } else if strong_filename {
        (0.90, AdrInferenceReason::StrongFilenameMarker)
    } else {
        return None;
    };

    let original_name = path.file_name()?.to_string_lossy().to_string();
    Some(AdrInference {
        kind: "adr",
        confidence,
        canonical_subpath: format!("adr/{original_name}"),
        reason,
    })
}

fn normalize_path(path: &Path) -> String {
    let raw = path.to_string_lossy().replace('\\', "/");
    let raw = raw.trim_start_matches("./");
    raw.split('/')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("/")
}

fn split_stem_ext(file_name: &str) -> Option<(&str, &str)> {
    let (stem, ext) = file_name.rsplit_once('.')?;
    Some((stem, ext))
}

fn contains_dir_marker(segments: &[&str]) -> bool {
    STRONG_DIR_PATTERNS
        .iter()
        .any(|pattern| contains_contiguous_pattern(segments, pattern))
}

fn contains_contiguous_pattern(segments: &[&str], pattern: &[&str]) -> bool {
    if pattern.is_empty() || segments.len() < pattern.len() {
        return false;
    }
    segments.windows(pattern.len()).any(|w| w == pattern)
}

fn is_strong_filename(stem: &str) -> bool {
    let lower = stem.to_ascii_lowercase();
    if !lower.starts_with("adr") {
        return false;
    }
    let rest = lower.trim_start_matches("adr");
    let rest = rest
        .strip_prefix('-')
        .or_else(|| rest.strip_prefix('_'))
        .or_else(|| rest.strip_prefix(' '))
        .unwrap_or(rest);
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    !digits.is_empty() && digits.len() <= 4
}

fn is_numbered_filename(stem: &str) -> bool {
    let digits: String = stem.chars().take_while(|c| c.is_ascii_digit()).collect();
    !digits.is_empty() && digits.len() <= 4
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn must_match_examples() {
        let yes = [
            "docs/adr/ADR-001-system-boundary.md",
            "adr/ADR-002-search-ranking.md",
            "decisions/architecture/0007-auth.md",
            "architecture/decisions/0012-context-store.md",
            "docs/decisions/ADR-018-events-model.md",
        ];
        for item in yes {
            assert!(
                infer_adr_from_repo_path(&p(item)).is_some(),
                "expected match: {item}"
            );
        }
    }

    #[test]
    fn must_not_match_examples() {
        let no = [
            "docs/addressing.md",
            "src/adapter/adr.rs",
            "assets/adr-logo.png",
            "notes/decision-log.md",
            "docs/architecture/overview.md",
        ];
        for item in no {
            assert!(
                infer_adr_from_repo_path(&p(item)).is_none(),
                "expected no match: {item}"
            );
        }
    }

    #[test]
    fn strong_dir_marker_wins_over_plain_filename() {
        let out = infer_adr_from_repo_path(&p("docs/adr/overview.md")).unwrap();
        assert_eq!(out.kind, "adr");
        assert_eq!(out.reason, AdrInferenceReason::StrongDirectoryMarker);
        assert_eq!(out.confidence, 0.95);
    }

    #[test]
    fn numbered_filename_inside_adr_dir_uses_lower_confidence() {
        let out = infer_adr_from_repo_path(&p("decisions/architecture/0007-auth.md")).unwrap();
        assert_eq!(
            out.reason,
            AdrInferenceReason::NumberedFilenameInsideAdrDirectory
        );
        assert_eq!(out.confidence, 0.92);
    }

    #[test]
    fn canonical_subpath_preserves_original_filename_casing() {
        let out = infer_adr_from_repo_path(&p("docs/adr/ADR-004-cache-policy.md")).unwrap();
        assert_eq!(out.canonical_subpath, "adr/ADR-004-cache-policy.md");
    }

    #[test]
    fn strong_filename_marker_outside_adr_dir() {
        let out = infer_adr_from_repo_path(&p("ADR-007-caching.md")).unwrap();
        assert_eq!(out.reason, AdrInferenceReason::StrongFilenameMarker);
        assert_eq!(out.confidence, 0.90);
    }

    #[test]
    fn negative_segment_blocks_inference() {
        assert!(infer_adr_from_repo_path(&p("src/adapter/adr.rs")).is_none());
        assert!(infer_adr_from_repo_path(&p("docs/addresses/adr-001.md")).is_none());
    }

    #[test]
    fn unsupported_extension_blocks_inference() {
        assert!(infer_adr_from_repo_path(&p("adr/ADR-001.pdf")).is_none());
        assert!(infer_adr_from_repo_path(&p("adr/ADR-001.drawio")).is_none());
    }
}
