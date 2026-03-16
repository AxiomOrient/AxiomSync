use crate::error::Result;
use crate::models::{
    EventQuery, EventRecord, IndexRecord, IngestProfile, LinkQuery, LinkRecord, ResourceQuery,
    ResourceRecord, SessionRecord, UpsertResource,
};
use crate::uri::AxiomUri;

use super::SqliteStateStore;

pub trait ResourceStore {
    fn persist_resource(&self, cmd: UpsertResource) -> Result<()>;
    fn tombstone_resource(&self, uri: &AxiomUri, tombstoned_at: i64) -> Result<bool>;
    fn get_resource(&self, uri: &AxiomUri) -> Result<Option<ResourceRecord>>;
    fn list_resources(&self, query: ResourceQuery) -> Result<Vec<ResourceRecord>>;
}

pub trait EventStore {
    fn append_events(&self, batch: &[EventRecord]) -> Result<usize>;
    fn query_events(&self, query: EventQuery) -> Result<Vec<EventRecord>>;
    fn tombstone_event(&self, uri: &AxiomUri, tombstoned_at: i64) -> Result<bool>;
}

pub trait SessionStore {
    fn save_session(&self, record: SessionRecord) -> Result<()>;
    fn load_session(&self, session_id: &str) -> Result<Option<SessionRecord>>;
    fn list_sessions(&self, limit: Option<usize>) -> Result<Vec<SessionRecord>>;
    fn delete_session(&self, session_id: &str) -> Result<bool>;
}

pub trait LinkStore {
    fn persist_link(&self, link: &LinkRecord) -> Result<()>;
    fn query_links(&self, query: LinkQuery) -> Result<Vec<LinkRecord>>;
}

pub trait SearchProjectionStore {
    fn list_search_documents(&self) -> Result<Vec<IndexRecord>>;
    fn get_search_document(&self, uri: &str) -> Result<Option<IndexRecord>>;
    fn persist_search_document(&self, record: &IndexRecord) -> Result<()>;
    fn persist_resource_search_document(
        &self,
        record: &ResourceRecord,
        profile: &IngestProfile,
    ) -> Result<bool>;
    fn persist_event_search_document(
        &self,
        record: &EventRecord,
        profile: &IngestProfile,
    ) -> Result<bool>;
    fn remove_search_document(&self, uri: &str) -> Result<()>;
}

pub trait ContextStore:
    ResourceStore + EventStore + LinkStore + SearchProjectionStore + SessionStore
{
}

impl<T> ContextStore for T where
    T: ResourceStore + EventStore + LinkStore + SearchProjectionStore + SessionStore
{
}

impl ResourceStore for SqliteStateStore {
    fn persist_resource(&self, cmd: UpsertResource) -> Result<()> {
        SqliteStateStore::persist_resource(self, cmd)
    }

    fn tombstone_resource(&self, uri: &AxiomUri, tombstoned_at: i64) -> Result<bool> {
        SqliteStateStore::tombstone_resource(self, uri, tombstoned_at)
    }

    fn get_resource(&self, uri: &AxiomUri) -> Result<Option<ResourceRecord>> {
        SqliteStateStore::get_resource(self, uri)
    }

    fn list_resources(&self, query: ResourceQuery) -> Result<Vec<ResourceRecord>> {
        SqliteStateStore::list_resources(self, query)
    }
}

impl EventStore for SqliteStateStore {
    fn append_events(&self, batch: &[EventRecord]) -> Result<usize> {
        SqliteStateStore::append_events(self, batch)
    }

    fn query_events(&self, query: EventQuery) -> Result<Vec<EventRecord>> {
        SqliteStateStore::query_events(self, query)
    }

    fn tombstone_event(&self, uri: &AxiomUri, tombstoned_at: i64) -> Result<bool> {
        SqliteStateStore::tombstone_event(self, uri, tombstoned_at)
    }
}

impl SessionStore for SqliteStateStore {
    fn save_session(&self, record: SessionRecord) -> Result<()> {
        SqliteStateStore::save_session(self, record)
    }

    fn load_session(&self, session_id: &str) -> Result<Option<SessionRecord>> {
        SqliteStateStore::load_session(self, session_id)
    }

    fn list_sessions(&self, limit: Option<usize>) -> Result<Vec<SessionRecord>> {
        SqliteStateStore::list_sessions(self, limit)
    }

    fn delete_session(&self, session_id: &str) -> Result<bool> {
        SqliteStateStore::delete_session(self, session_id)
    }
}

impl LinkStore for SqliteStateStore {
    fn persist_link(&self, link: &LinkRecord) -> Result<()> {
        SqliteStateStore::persist_link(self, link)
    }

    fn query_links(&self, query: LinkQuery) -> Result<Vec<LinkRecord>> {
        SqliteStateStore::query_links(self, query)
    }
}

impl SearchProjectionStore for SqliteStateStore {
    fn list_search_documents(&self) -> Result<Vec<IndexRecord>> {
        SqliteStateStore::list_search_documents(self)
    }

    fn get_search_document(&self, uri: &str) -> Result<Option<IndexRecord>> {
        SqliteStateStore::get_search_document(self, uri)
    }

    fn persist_search_document(&self, record: &IndexRecord) -> Result<()> {
        SqliteStateStore::persist_search_document(self, record)
    }

    fn persist_resource_search_document(
        &self,
        record: &ResourceRecord,
        profile: &IngestProfile,
    ) -> Result<bool> {
        SqliteStateStore::persist_resource_search_document(self, record, profile)
    }

    fn persist_event_search_document(
        &self,
        record: &EventRecord,
        profile: &IngestProfile,
    ) -> Result<bool> {
        SqliteStateStore::persist_event_search_document(self, record, profile)
    }

    fn remove_search_document(&self, uri: &str) -> Result<()> {
        SqliteStateStore::remove_search_document(self, uri)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::models::{
        EventQuery, IngestProfile, Kind, NamespaceKey, ResourceQuery, UpsertResource,
    };
    use crate::uri::AxiomUri;

    use super::{ContextStore, EventStore, ResourceStore, SearchProjectionStore, SqliteStateStore};

    fn assert_context_store<T: ContextStore>() {}

    #[test]
    fn sqlite_state_store_exposes_context_store_capability_seam() {
        assert_context_store::<SqliteStateStore>();
    }

    #[test]
    fn capability_traits_delegate_to_sqlite_store_hot_path() {
        let temp = tempdir().expect("tempdir");
        let store = SqliteStateStore::open(temp.path().join("state.db")).expect("open");
        let now = 1_710_000_000_i64;
        let resource_uri =
            AxiomUri::parse("axiom://resources/acme/contracts/auth").expect("resource uri");
        let resource = UpsertResource {
            resource_id: "res-1".to_string(),
            uri: resource_uri.clone(),
            namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
            kind: Kind::new("contract").expect("kind"),
            title: Some("Auth Contract".to_string()),
            mime: Some("text/markdown".to_string()),
            tags: vec!["contract".to_string()],
            attrs: serde_json::json!({"owner": "platform"}),
            object_uri: None,
            excerpt_text: Some("OAuth refresh contract".to_string()),
            content_hash: "hash-1".to_string(),
            tombstoned_at: None,
            created_at: now,
            updated_at: now,
        };

        ResourceStore::persist_resource(&store, resource).expect("persist resource");
        let stored = ResourceStore::get_resource(&store, &resource_uri)
            .expect("get resource")
            .expect("resource");
        assert_eq!(stored.kind.as_str(), "contract");

        let profile = IngestProfile::for_kind(&stored.kind);
        SearchProjectionStore::persist_resource_search_document(&store, &stored, &profile)
            .expect("persist projection");
        let docs = SearchProjectionStore::list_search_documents(&store).expect("list docs");
        assert!(docs.iter().any(|doc| doc.uri == resource_uri.to_string()));

        let events = EventStore::query_events(
            &store,
            EventQuery {
                namespace_prefix: Some(NamespaceKey::parse("acme").expect("namespace")),
                ..EventQuery::default()
            },
        )
        .expect("query events");
        assert!(events.is_empty());

        let resources = ResourceStore::list_resources(
            &store,
            ResourceQuery {
                namespace_prefix: Some(NamespaceKey::parse("acme").expect("namespace")),
                ..ResourceQuery::default()
            },
        )
        .expect("list resources");
        assert_eq!(resources.len(), 1);
    }
}
