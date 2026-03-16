use serde::{Deserialize, Serialize};

use super::Kind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IndexPolicy {
    #[default]
    FullText,
    SummaryOnly,
    MetadataOnly,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RetentionClass {
    #[default]
    LongLived,
    Operational,
    Ephemeral,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IngestProfile {
    pub kind: Kind,
    pub index_policy: IndexPolicy,
    pub retention: RetentionClass,
}

impl IngestProfile {
    #[must_use]
    pub fn for_kind(kind: &Kind) -> Self {
        let (index_policy, retention) = match kind.as_str() {
            "contract" | "adr" | "runbook" => (IndexPolicy::FullText, RetentionClass::LongLived),
            "incident" | "run" | "deploy" => {
                (IndexPolicy::SummaryOnly, RetentionClass::Operational)
            }
            "log" | "trace" => (IndexPolicy::MetadataOnly, RetentionClass::Ephemeral),
            _ => (IndexPolicy::FullText, RetentionClass::LongLived),
        };
        Self {
            kind: kind.clone(),
            index_policy,
            retention,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profiles_match_v3_policy() {
        let contract = IngestProfile::for_kind(&Kind::new("contract").expect("kind"));
        let incident = IngestProfile::for_kind(&Kind::new("incident").expect("kind"));
        let trace = IngestProfile::for_kind(&Kind::new("trace").expect("kind"));

        assert_eq!(contract.index_policy, IndexPolicy::FullText);
        assert_eq!(contract.retention, RetentionClass::LongLived);
        assert_eq!(incident.index_policy, IndexPolicy::SummaryOnly);
        assert_eq!(incident.retention, RetentionClass::Operational);
        assert_eq!(trace.index_policy, IndexPolicy::MetadataOnly);
        assert_eq!(trace.retention, RetentionClass::Ephemeral);
    }
}
