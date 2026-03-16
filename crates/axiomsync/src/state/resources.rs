use rusqlite::{Connection, OptionalExtension, params, params_from_iter, types::Value};

use crate::error::Result;
use crate::models::{ResourceQuery, ResourceRecord, UpsertResource};
use crate::uri::AxiomUri;

use super::SqliteStateStore;
use super::db::{
    normalize_tags, parse_json_column, parse_kind, parse_namespace, parse_optional_uri, parse_uri,
    push_namespace_range_filter,
};

impl SqliteStateStore {
    pub fn persist_resource(&self, cmd: UpsertResource) -> Result<()> {
        let tags_json = serde_json::to_string(&normalize_tags(&cmd.tags))?;
        let attrs_json = serde_json::to_string(&cmd.attrs)?;
        let namespace = cmd.namespace.as_path();
        let kind = cmd.kind.as_str().to_string();
        let uri = cmd.uri.to_string();
        let object_uri = cmd.object_uri.as_ref().map(ToString::to_string);

        self.with_conn(|conn| {
            conn.execute(
                r"
                INSERT INTO resources(
                    resource_id, uri, namespace, kind, title, mime, tags_json, attrs_json,
                    object_uri, excerpt_text, content_hash, tombstoned_at, created_at, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                ON CONFLICT(resource_id) DO UPDATE SET
                  uri = excluded.uri,
                  namespace = excluded.namespace,
                  kind = excluded.kind,
                  title = excluded.title,
                  mime = excluded.mime,
                  tags_json = excluded.tags_json,
                  attrs_json = excluded.attrs_json,
                  object_uri = excluded.object_uri,
                  excerpt_text = excluded.excerpt_text,
                  content_hash = excluded.content_hash,
                  tombstoned_at = excluded.tombstoned_at,
                  created_at = excluded.created_at,
                  updated_at = excluded.updated_at
                ",
                params![
                    cmd.resource_id,
                    uri,
                    namespace,
                    kind,
                    cmd.title,
                    cmd.mime,
                    tags_json,
                    attrs_json,
                    object_uri,
                    cmd.excerpt_text,
                    cmd.content_hash,
                    cmd.tombstoned_at,
                    cmd.created_at,
                    cmd.updated_at,
                ],
            )?;
            Ok(())
        })
    }

    pub fn tombstone_resource(&self, uri: &AxiomUri, tombstoned_at: i64) -> Result<bool> {
        self.with_conn(|conn| {
            let affected = conn.execute(
                "UPDATE resources SET tombstoned_at = ?2, updated_at = MAX(updated_at, ?2) WHERE uri = ?1",
                params![uri.to_string(), tombstoned_at],
            )?;
            Ok(affected > 0)
        })
    }

    pub fn get_resource(&self, uri: &AxiomUri) -> Result<Option<ResourceRecord>> {
        self.with_conn(|conn| query_resource(conn, uri).map_err(Into::into))
    }

    pub fn nearest_active_resource(&self, uri: &AxiomUri) -> Result<Option<ResourceRecord>> {
        self.with_conn(|conn| {
            for candidate in resource_ancestor_chain(uri) {
                if let Some(resource) = query_resource(conn, &candidate)?
                    && resource.tombstoned_at.is_none()
                {
                    return Ok(Some(resource));
                }
            }
            Ok(None)
        })
    }

    pub fn list_resources(&self, query: ResourceQuery) -> Result<Vec<ResourceRecord>> {
        self.with_conn(|conn| {
            let mut sql = String::from(
                r"
                SELECT resource_id, uri, namespace, kind, title, mime, tags_json, attrs_json,
                       object_uri, excerpt_text, content_hash, tombstoned_at, created_at, updated_at
                FROM resources
                WHERE 1 = 1
                ",
            );
            let mut params = Vec::<Value>::new();

            if !query.include_tombstoned {
                sql.push_str(" AND tombstoned_at IS NULL");
            }

            if let Some(namespace) = query.namespace_prefix {
                push_namespace_range_filter(
                    &mut sql,
                    &mut params,
                    "namespace",
                    &namespace.as_path(),
                );
            }

            if let Some(kind) = query.kind {
                sql.push_str(" AND kind = ?");
                params.push(Value::Text(kind.as_str().to_string()));
            }

            sql.push_str(" ORDER BY updated_at DESC, uri ASC");
            if let Some(limit) = query.limit {
                sql.push_str(" LIMIT ?");
                params.push(Value::Integer(i64::try_from(limit).unwrap_or(i64::MAX)));
            }

            let mut stmt = conn.prepare(&sql)?;
            stmt.query_map(params_from_iter(params), map_resource_record)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(Into::into)
        })
    }
}

fn map_resource_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<ResourceRecord> {
    Ok(ResourceRecord {
        resource_id: row.get(0)?,
        uri: parse_uri(row.get::<_, String>(1)?)?,
        namespace: parse_namespace(row.get::<_, String>(2)?)?,
        kind: parse_kind(row.get::<_, String>(3)?)?,
        title: row.get(4)?,
        mime: row.get(5)?,
        tags: parse_json_column(row.get::<_, String>(6)?)?,
        attrs: parse_json_column(row.get::<_, String>(7)?)?,
        object_uri: parse_optional_uri(row.get(8)?)?,
        excerpt_text: row.get(9)?,
        content_hash: row.get(10)?,
        tombstoned_at: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

fn query_resource(conn: &Connection, uri: &AxiomUri) -> rusqlite::Result<Option<ResourceRecord>> {
    conn.query_row(
        r"
        SELECT resource_id, uri, namespace, kind, title, mime, tags_json, attrs_json,
               object_uri, excerpt_text, content_hash, tombstoned_at, created_at, updated_at
        FROM resources
        WHERE uri = ?1
        ",
        params![uri.to_string()],
        map_resource_record,
    )
    .optional()
}

fn resource_ancestor_chain(uri: &AxiomUri) -> Vec<AxiomUri> {
    let mut chain = Vec::new();
    let mut cursor = Some(uri.clone());
    while let Some(candidate) = cursor {
        chain.push(candidate.clone());
        cursor = candidate.parent();
    }
    chain
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::models::{NamespaceKey, ResourceQuery, UpsertResource};
    use crate::uri::AxiomUri;

    use super::SqliteStateStore;

    #[test]
    fn resource_upsert_get_list_and_tombstone_roundtrip() {
        let temp = tempdir().expect("tempdir");
        let store = SqliteStateStore::open(temp.path().join("state.db")).expect("open");
        let now = 1_710_000_000_i64;

        store
            .persist_resource(UpsertResource {
                resource_id: "res-1".to_string(),
                uri: AxiomUri::parse("axiom://resources/acme/contracts/auth").expect("uri"),
                namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
                kind: "contract".parse().expect("kind"),
                title: Some("Auth Contract".to_string()),
                mime: Some("text/markdown".to_string()),
                tags: vec!["API".to_string(), "contract".to_string()],
                attrs: serde_json::json!({"owner": "platform"}),
                object_uri: None,
                excerpt_text: Some("Authentication contract".to_string()),
                content_hash: "hash-1".to_string(),
                tombstoned_at: None,
                created_at: now,
                updated_at: now + 10,
            })
            .expect("upsert");

        let uri = AxiomUri::parse("axiom://resources/acme/contracts/auth").expect("uri");
        let stored = store
            .get_resource(&uri)
            .expect("get")
            .expect("resource missing");
        assert_eq!(stored.namespace.as_path(), "acme/platform");
        assert_eq!(stored.kind.as_str(), "contract");
        assert_eq!(stored.tags, vec!["api".to_string(), "contract".to_string()]);

        let listed = store
            .list_resources(ResourceQuery {
                namespace_prefix: Some(NamespaceKey::parse("acme").expect("namespace")),
                kind: Some("contract".parse().expect("kind")),
                limit: Some(10),
                include_tombstoned: false,
            })
            .expect("list");
        assert_eq!(listed.len(), 1);

        assert!(store.tombstone_resource(&uri, now + 20).expect("tombstone"));
        let active = store
            .list_resources(ResourceQuery {
                namespace_prefix: Some(NamespaceKey::parse("acme").expect("namespace")),
                kind: None,
                limit: None,
                include_tombstoned: false,
            })
            .expect("list active");
        assert!(active.is_empty());
    }

    #[test]
    fn nearest_active_resource_walks_up_uri_tree() {
        let temp = tempdir().expect("tempdir");
        let store = SqliteStateStore::open(temp.path().join("state.db")).expect("open");
        let root_uri = AxiomUri::parse("axiom://resources/acme/repos/platform").expect("uri");

        store
            .persist_resource(UpsertResource {
                resource_id: "res-root".to_string(),
                uri: root_uri.clone(),
                namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
                kind: "repository".parse().expect("kind"),
                title: Some("Platform Repo".to_string()),
                mime: None,
                tags: vec!["repo".to_string()],
                attrs: serde_json::json!({}),
                object_uri: None,
                excerpt_text: None,
                content_hash: "hash-root".to_string(),
                tombstoned_at: None,
                created_at: 1_710_000_000,
                updated_at: 1_710_000_000,
            })
            .expect("persist root");

        let child_uri = AxiomUri::parse("axiom://resources/acme/repos/platform/runbooks/oauth.md")
            .expect("child uri");
        let resource = store
            .nearest_active_resource(&child_uri)
            .expect("lookup")
            .expect("resource");

        assert_eq!(resource.uri, root_uri);
        assert_eq!(resource.namespace.as_path(), "acme/platform");
    }
}
