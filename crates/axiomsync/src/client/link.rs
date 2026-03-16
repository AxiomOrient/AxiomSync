use chrono::Utc;

use crate::error::Result;
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
        let record = LinkRecord {
            link_id: req.link_id,
            namespace: req.namespace,
            from_uri: req.from_uri,
            relation: req.relation.trim().to_ascii_lowercase(),
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
