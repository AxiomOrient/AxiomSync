use chrono::Utc;

use crate::error::{AxiomError, Result};
use crate::models::{LinkRecord, LinkRequest};

use super::AxiomSync;

pub(super) struct LinkService<'a> {
    app: &'a AxiomSync,
}

impl<'a> LinkService<'a> {
    pub(super) fn new(app: &'a AxiomSync) -> Self {
        Self { app }
    }

    pub(super) fn link_records(&self, req: LinkRequest) -> Result<LinkRecord> {
        let created_at = req.created_at.unwrap_or_else(|| Utc::now().timestamp());
        let relation = req.relation.trim().to_ascii_lowercase();
        validate_relation(&relation)?;
        let record = LinkRecord {
            link_id: req.link_id,
            namespace: req.namespace,
            from_uri: req.from_uri,
            relation,
            to_uri: req.to_uri,
            weight: req.weight,
            attrs: req.attrs,
            created_at,
        };
        self.app.state.persist_link(&record)?;
        Ok(record)
    }
}

impl AxiomSync {
    pub fn link_records(&self, req: LinkRequest) -> Result<LinkRecord> {
        self.link_service().link_records(req)
    }
}

fn validate_relation(relation: &str) -> Result<()> {
    if relation.is_empty() {
        return Err(AxiomError::Validation(
            "link relation must not be empty".to_string(),
        ));
    }
    if !relation
        .chars()
        .all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_' | '-'))
    {
        return Err(AxiomError::Validation(format!(
            "link relation contains invalid characters: {:?} (allowed: [a-z0-9_-])",
            relation
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::validate_relation;
    use crate::AxiomSync;
    use crate::error::AxiomError;
    use crate::models::{LinkRequest, NamespaceKey};
    use crate::uri::AxiomUri;

    #[test]
    fn validate_relation_accepts_valid_identifiers() {
        for valid in ["references", "depends-on", "blocks_work", "owns", "a1-b2_c3"] {
            validate_relation(valid).unwrap_or_else(|e| panic!("{valid:?} should be valid: {e}"));
        }
    }

    #[test]
    fn validate_relation_rejects_empty() {
        let err = validate_relation("").expect_err("empty must be rejected");
        assert!(matches!(err, AxiomError::Validation(_)));
    }

    #[test]
    fn validate_relation_rejects_uppercase() {
        let err = validate_relation("References").expect_err("uppercase must be rejected");
        assert!(matches!(err, AxiomError::Validation(_)));
    }

    #[test]
    fn validate_relation_rejects_spaces_and_special_chars() {
        for invalid in ["has ref", "dep/on", "link.to", "rel@2", "rel:v1"] {
            validate_relation(invalid)
                .expect_err(&format!("{invalid:?} should be rejected"));
        }
    }

    #[test]
    fn link_records_accepts_valid_relation_and_normalizes_case() {
        let temp = tempdir().expect("tempdir");
        let app = AxiomSync::new(temp.path()).expect("app");
        app.initialize().expect("init");

        let record = app
            .link_records(LinkRequest {
                link_id: "lnk-1".to_string(),
                namespace: NamespaceKey::parse("acme").expect("namespace"),
                from_uri: AxiomUri::parse("axiom://resources/doc-a").expect("uri"),
                relation: "  References  ".to_string(),
                to_uri: AxiomUri::parse("axiom://resources/doc-b").expect("uri"),
                weight: 1.0,
                attrs: serde_json::json!({}),
                created_at: None,
            })
            .expect("link ok");

        assert_eq!(record.relation, "references");
    }

    #[test]
    fn link_records_rejects_invalid_relation_before_persisting() {
        let temp = tempdir().expect("tempdir");
        let app = AxiomSync::new(temp.path()).expect("app");
        app.initialize().expect("init");

        let err = app
            .link_records(LinkRequest {
                link_id: "lnk-bad".to_string(),
                namespace: NamespaceKey::parse("acme").expect("namespace"),
                from_uri: AxiomUri::parse("axiom://resources/doc-a").expect("uri"),
                relation: "bad relation!".to_string(),
                to_uri: AxiomUri::parse("axiom://resources/doc-b").expect("uri"),
                weight: 1.0,
                attrs: serde_json::json!({}),
                created_at: None,
            })
            .expect_err("invalid relation must be rejected");

        assert!(matches!(err, AxiomError::Validation(_)));

        let stored = app
            .state
            .query_links(crate::models::LinkQuery::default())
            .expect("query links");
        assert!(stored.is_empty(), "invalid relation must not be persisted");
    }
}
