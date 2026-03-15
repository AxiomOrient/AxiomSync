use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{AxiomError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Kind(String);

impl Kind {
    pub fn new(raw: impl Into<String>) -> Result<Self> {
        let normalized = normalize_kind(&raw.into())?;
        Ok(Self(normalized))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for Kind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Kind {
    type Err = AxiomError;

    fn from_str(s: &str) -> Result<Self> {
        Self::new(s)
    }
}

fn normalize_kind(raw: &str) -> Result<String> {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(AxiomError::Validation("kind must not be empty".to_string()));
    }
    if !normalized
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        return Err(AxiomError::Validation(format!(
            "kind contains unsupported characters: {raw}"
        )));
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_normalizes_case() {
        let kind = Kind::new("RunBook").expect("kind");
        assert_eq!(kind.as_str(), "runbook");
    }

    #[test]
    fn kind_rejects_whitespace() {
        let err = Kind::new(" ").expect_err("must fail");
        assert!(matches!(err, AxiomError::Validation(_)));
    }
}
