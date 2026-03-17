//! Thin orchestration boundary for `AxiomSync`.
//!
//! Cross-service entrypoints stay on `AxiomSync`, while domain-specific behavior
//! lives in dedicated modules such as `event`, `link`, `repo`, `archive`,
//! `search`, and `release`.

use super::{
    AxiomSync,
    archive::ArchiveService,
    event::EventService,
    link::LinkService,
    release::ReleaseVerificationService,
    repo::RepoService,
    resource::ResourceService,
    runtime::{RuntimeBootstrapService, SessionService},
    search::SearchService,
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

    pub(super) fn release_verification_service(&self) -> ReleaseVerificationService<'_> {
        ReleaseVerificationService::new(self)
    }

    pub(super) fn runtime_bootstrap_service(&self) -> RuntimeBootstrapService<'_> {
        RuntimeBootstrapService::new(self)
    }

    pub(super) fn resource_service(&self) -> ResourceService<'_> {
        ResourceService::new(self)
    }

    pub(super) fn search_service(&self) -> SearchService<'_> {
        SearchService::new(self)
    }

    pub(super) fn session_service(&self) -> SessionService<'_> {
        SessionService::new(self)
    }
}
