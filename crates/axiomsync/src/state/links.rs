use rusqlite::{params, params_from_iter, types::Value};

use crate::error::{AxiomError, Result};
use crate::models::{LinkQuery, LinkRecord};

use super::SqliteStateStore;
use super::db::{parse_json_column, parse_namespace, parse_uri, push_namespace_range_filter};

impl SqliteStateStore {
    pub fn persist_link(&self, link: &LinkRecord) -> Result<()> {
        let relation = normalize_relation(&link.relation)?;
        self.with_conn(|conn| {
            conn.execute(
                r"
                INSERT INTO links(link_id, namespace, from_uri, relation, to_uri, weight, attrs_json, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ON CONFLICT(link_id) DO UPDATE SET
                  namespace = excluded.namespace,
                  from_uri = excluded.from_uri,
                  relation = excluded.relation,
                  to_uri = excluded.to_uri,
                  weight = excluded.weight,
                  attrs_json = excluded.attrs_json,
                  created_at = excluded.created_at
                ",
                params![
                    link.link_id,
                    link.namespace.as_path(),
                    link.from_uri.to_string(),
                    relation,
                    link.to_uri.to_string(),
                    link.weight,
                    serde_json::to_string(&link.attrs)?,
                    link.created_at,
                ],
            )?;
            Ok(())
        })
    }

    pub fn query_links(&self, query: LinkQuery) -> Result<Vec<LinkRecord>> {
        self.with_conn(|conn| {
            let mut sql = String::from(
                r"
                SELECT link_id, namespace, from_uri, relation, to_uri, weight, attrs_json, created_at
                FROM links
                WHERE 1 = 1
                ",
            );
            let mut params = Vec::<Value>::new();

            if let Some(namespace) = query.namespace_prefix {
                push_namespace_range_filter(&mut sql, &mut params, "namespace", &namespace.as_path());
            }
            if let Some(from_uri) = query.from_uri {
                sql.push_str(" AND from_uri = ?");
                params.push(Value::Text(from_uri.to_string()));
            }
            if let Some(to_uri) = query.to_uri {
                sql.push_str(" AND to_uri = ?");
                params.push(Value::Text(to_uri.to_string()));
            }
            if let Some(relation) = query.relation {
                sql.push_str(" AND relation = ?");
                params.push(Value::Text(normalize_relation(&relation)?));
            }

            sql.push_str(" ORDER BY created_at DESC, link_id ASC");
            if let Some(limit) = query.limit {
                sql.push_str(" LIMIT ?");
                params.push(Value::Integer(i64::try_from(limit).unwrap_or(i64::MAX)));
            }

            let mut stmt = conn.prepare(&sql)?;
            stmt.query_map(params_from_iter(params), map_link_record)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(Into::into)
        })
    }
}

fn map_link_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<LinkRecord> {
    Ok(LinkRecord {
        link_id: row.get(0)?,
        namespace: parse_namespace(row.get::<_, String>(1)?)?,
        from_uri: parse_uri(row.get::<_, String>(2)?)?,
        relation: row.get(3)?,
        to_uri: parse_uri(row.get::<_, String>(4)?)?,
        weight: row.get(5)?,
        attrs: parse_json_column(row.get::<_, String>(6)?)?,
        created_at: row.get(7)?,
    })
}

fn normalize_relation(raw: &str) -> Result<String> {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(AxiomError::Validation(
            "relation must not be empty".to_string(),
        ));
    }
    if !normalized
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        return Err(AxiomError::Validation(format!(
            "relation contains unsupported characters: {raw}"
        )));
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::models::{LinkQuery, LinkRecord, NamespaceKey};
    use crate::uri::AxiomUri;

    use super::SqliteStateStore;

    #[test]
    fn upsert_and_query_links() {
        let temp = tempdir().expect("tempdir");
        let store = SqliteStateStore::open(temp.path().join("state.db")).expect("open");

        store
            .persist_link(&LinkRecord {
                link_id: "link-1".to_string(),
                namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
                from_uri: AxiomUri::parse("axiom://events/acme/incidents/1").expect("from"),
                relation: "resolved_by".to_string(),
                to_uri: AxiomUri::parse("axiom://resources/acme/runbooks/auth").expect("to"),
                weight: 0.9,
                attrs: serde_json::json!({"source": "incident"}),
                created_at: 200,
            })
            .expect("persist");

        let links = store
            .query_links(LinkQuery {
                namespace_prefix: Some(NamespaceKey::parse("acme").expect("namespace")),
                from_uri: Some(AxiomUri::parse("axiom://events/acme/incidents/1").expect("from")),
                to_uri: None,
                relation: Some("resolved_by".to_string()),
                limit: Some(10),
            })
            .expect("query");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].relation, "resolved_by");
    }
}
