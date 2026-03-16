//! Thin orchestration boundary for `AxiomSync`.
//!
//! Cross-service entrypoints stay on `AxiomSync`, while domain-specific behavior
//! lives in dedicated modules such as `event`, `link`, `repo`, `archive`,
//! `search`, and `release`.

use super::{
    AxiomSync, archive::ArchiveService, event::EventService, link::LinkService, repo::RepoService,
};

impl AxiomSync {
    pub(super) fn event_service(&self) -> EventService<'_> {
        EventService::new(self)
    }

    pub(super) fn repo_service(&self) -> RepoService<'_> {
        RepoService::new(self)
    }

    pub(super) fn link_service(&self) -> LinkService<'_> {
        LinkService::new(self)
    }

    pub(super) fn archive_service(&self) -> ArchiveService<'_> {
        ArchiveService::new(self)
    }
}
