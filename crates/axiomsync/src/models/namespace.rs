use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{AxiomError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct NamespaceKey {
    pub segments: Vec<String>,
}

impl NamespaceKey {
    pub fn new<I, S>(segments: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut normalized = Vec::new();
        for segment in segments {
            normalized.push(normalize_namespace_segment(&segment.into())?);
        }
        if normalized.is_empty() {
            return Err(AxiomError::Validation(
                "namespace must contain at least one segment".to_string(),
            ));
        }
        Ok(Self {
            segments: normalized,
        })
    }

    pub fn parse(raw: &str) -> Result<Self> {
        let segments = raw
            .split('/')
            .map(str::trim)
            .filter(|segment| !segment.is_empty())
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        Self::new(segments)
    }

    #[must_use]
    pub fn as_path(&self) -> String {
        self.segments.join("/")
    }
}

impl Display for NamespaceKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.as_path())
    }
}

impl FromStr for NamespaceKey {
    type Err = AxiomError;

    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}

fn normalize_namespace_segment(raw: &str) -> Result<String> {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(AxiomError::Validation(
            "namespace segment must not be empty".to_string(),
        ));
    }
    if normalized == "." || normalized == ".." {
        return Err(AxiomError::Validation(format!(
            "namespace segment is invalid: {raw}"
        )));
    }
    if !normalized
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Err(AxiomError::Validation(format!(
            "namespace segment contains unsupported characters: {raw}"
        )));
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespace_normalizes_segments() {
        let namespace = NamespaceKey::parse("Org/API.Team").expect("namespace");
        assert_eq!(namespace.as_path(), "org/api.team");
    }

    #[test]
    fn namespace_rejects_empty_input() {
        let err = NamespaceKey::parse(" / ").expect_err("must fail");
        assert!(matches!(err, AxiomError::Validation(_)));
    }
}
