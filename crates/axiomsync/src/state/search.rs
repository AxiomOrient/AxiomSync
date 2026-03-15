use chrono::Utc;
use rusqlite::{OptionalExtension, params, types::Type};

use crate::error::Result;
use crate::mime::infer_mime;
use crate::models::{
    EventRecord, IndexPolicy, IndexRecord, IngestProfile, Kind, NamespaceKey, ResourceRecord,
};

use super::SqliteStateStore;
use super::db::{normalize_tags, push_namespace_range_filter};

struct SearchProjectionMeta {
    namespace: Option<String>,
    kind: Option<String>,
    event_time: Option<i64>,
    source_weight: f32,
    freshness_bucket: i64,
    mime: Option<String>,
}

impl SqliteStateStore {
    pub fn search_documents_fts(&self, query: &str, limit: usize) -> Result<Vec<String>> {
        let query = query.trim();
        if query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                r"
                SELECT d.uri
                FROM search_docs_fts
                JOIN search_docs d ON d.id = search_docs_fts.rowid
                WHERE search_docs_fts MATCH ?1
                ORDER BY bm25(search_docs_fts) ASC, d.uri ASC
                LIMIT ?2
                ",
            )?;
            let rows = stmt.query_map(params![query, usize_to_i64_saturating(limit)], |row| {
                row.get::<_, String>(0)
            })?;

            let mut uris = Vec::new();
            for row in rows {
                uris.push(row?);
            }
            Ok(uris)
        })
    }

    pub fn search_documents_fts_filtered(
        &self,
        query: &str,
        namespace_prefix: Option<&NamespaceKey>,
        kind: Option<&Kind>,
        min_event_time: Option<i64>,
        limit: usize,
    ) -> Result<Vec<String>> {
        let query = query.trim();
        if query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        self.with_conn(|conn| {
            let mut sql = String::from(
                r"
                SELECT d.uri
                FROM search_docs_fts
                JOIN search_docs d ON d.id = search_docs_fts.rowid
                WHERE search_docs_fts MATCH ?1
                ",
            );
            let mut params = vec![rusqlite::types::Value::Text(query.to_string())];

            if let Some(namespace) = namespace_prefix {
                push_namespace_range_filter(
                    &mut sql,
                    &mut params,
                    "d.namespace",
                    &namespace.as_path(),
                );
            }
            if let Some(kind) = kind {
                sql.push_str(" AND d.kind = ?");
                params.push(rusqlite::types::Value::Text(kind.as_str().to_string()));
            }
            if let Some(min_event_time) = min_event_time {
                sql.push_str(" AND COALESCE(d.event_time, 0) >= ?");
                params.push(rusqlite::types::Value::Integer(min_event_time));
            }

            sql.push_str(
                " ORDER BY bm25(search_docs_fts) ASC, d.event_time DESC, d.uri ASC LIMIT ?",
            );
            params.push(rusqlite::types::Value::Integer(usize_to_i64_saturating(
                limit,
            )));

            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
                row.get::<_, String>(0)
            })?;

            let mut uris = Vec::new();
            for row in rows {
                uris.push(row?);
            }
            Ok(uris)
        })
    }

    pub fn list_search_documents(&self) -> Result<Vec<IndexRecord>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                r"
                SELECT
                  d.uri,
                  d.parent_uri,
                  d.is_leaf,
                  d.context_type,
                  d.name,
                  d.abstract_text,
                  d.content,
                  d.updated_at,
                  d.depth,
                  COALESCE(
                    (
                        SELECT group_concat(tag, ' ')
                        FROM search_doc_tags t
                        WHERE t.doc_id = d.id
                    ),
                    d.tags_text,
                    ''
                  ) AS tags_text
                FROM search_docs d
                ORDER BY d.depth ASC, d.uri ASC
                ",
            )?;
            let rows = stmt.query_map([], |row| {
                let uri = row.get::<_, String>(0)?;
                let updated_raw = row.get::<_, String>(7)?;
                let updated_at = parse_required_rfc3339(7, &updated_raw)?;
                let tags = row
                    .get::<_, String>(9)?
                    .split_whitespace()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();

                Ok(IndexRecord {
                    id: blake3::hash(uri.as_bytes()).to_hex().to_string(),
                    uri,
                    parent_uri: row.get(1)?,
                    is_leaf: row.get::<_, i64>(2)? != 0,
                    context_type: row.get(3)?,
                    name: row.get(4)?,
                    abstract_text: row.get(5)?,
                    content: row.get(6)?,
                    tags,
                    updated_at,
                    depth: i64_to_usize_saturating(row.get::<_, i64>(8)?),
                })
            })?;

            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })
    }

    pub fn clear_search_index(&self) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM search_doc_tags", [])?;
            conn.execute("DELETE FROM search_docs", [])?;
            Ok(())
        })
    }

    pub fn persist_search_document(&self, record: &IndexRecord) -> Result<()> {
        let (mut namespace, mut kind) = (None::<&str>, None::<&str>);
        for tag in &record.tags {
            if namespace.is_none()
                && let Some(v) = tag.strip_prefix("ns:")
            {
                namespace = Some(v);
            }
            if kind.is_none()
                && let Some(v) = tag.strip_prefix("kind:")
            {
                kind = Some(v);
            }
            if namespace.is_some() && kind.is_some() {
                break;
            }
        }
        let meta = SearchProjectionMeta {
            namespace: namespace.map(ToOwned::to_owned),
            kind: kind.map(ToOwned::to_owned),
            event_time: None,
            source_weight: 0.0,
            freshness_bucket: 0,
            mime: infer_mime(record).map(ToString::to_string),
        };
        self.persist_projection(record, &meta)
    }

    pub fn persist_resource_search_document(
        &self,
        record: &ResourceRecord,
        profile: &IngestProfile,
    ) -> Result<bool> {
        let Some((index_record, meta)) = build_search_doc_from_resource(record, profile) else {
            self.remove_search_document(&record.uri.to_string())?;
            return Ok(false);
        };
        self.persist_projection(&index_record, &meta)?;
        Ok(true)
    }

    pub fn persist_event_search_document(
        &self,
        record: &EventRecord,
        profile: &IngestProfile,
    ) -> Result<bool> {
        let Some((index_record, meta)) = build_search_doc_from_event(record, profile) else {
            self.remove_search_document(&record.uri.to_string())?;
            return Ok(false);
        };
        self.persist_projection(&index_record, &meta)?;
        Ok(true)
    }

    pub fn get_search_document(&self, uri: &str) -> Result<Option<IndexRecord>> {
        self.with_conn(|conn| {
            let result = conn
                .query_row(
                    r"
                    SELECT
                      d.uri,
                      d.parent_uri,
                      d.is_leaf,
                      d.context_type,
                      d.name,
                      d.abstract_text,
                      d.content,
                      d.updated_at,
                      d.depth,
                      COALESCE(
                        (
                            SELECT group_concat(tag, ' ')
                            FROM search_doc_tags t
                            WHERE t.doc_id = d.id
                        ),
                        d.tags_text,
                        ''
                      ) AS tags_text
                    FROM search_docs d
                    WHERE d.uri = ?1
                    ",
                    rusqlite::params![uri],
                    |row| {
                        let uri = row.get::<_, String>(0)?;
                        let updated_raw = row.get::<_, String>(7)?;
                        let updated_at = parse_required_rfc3339(7, &updated_raw)?;
                        let tags = row
                            .get::<_, String>(9)?
                            .split_whitespace()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>();
                        Ok(IndexRecord {
                            id: blake3::hash(uri.as_bytes()).to_hex().to_string(),
                            uri,
                            parent_uri: row.get(1)?,
                            is_leaf: row.get::<_, i64>(2)? != 0,
                            context_type: row.get(3)?,
                            name: row.get(4)?,
                            abstract_text: row.get(5)?,
                            content: row.get(6)?,
                            tags,
                            updated_at,
                            depth: i64_to_usize_saturating(row.get::<_, i64>(8)?),
                        })
                    },
                )
                .optional()?;
            Ok(result)
        })
    }

    pub fn get_search_documents_batch(
        &self,
        uris: &[String],
    ) -> Result<std::collections::HashMap<String, IndexRecord>> {
        if uris.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let placeholders = (1..=uris.len())
            .map(|i| format!("?{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            r"
            SELECT
              d.uri, d.parent_uri, d.is_leaf, d.context_type, d.name,
              d.abstract_text, d.content, d.updated_at, d.depth,
              COALESCE(
                (SELECT group_concat(tag, ' ') FROM search_doc_tags t WHERE t.doc_id = d.id),
                d.tags_text, ''
              ) AS tags_text
            FROM search_docs d
            WHERE d.uri IN ({placeholders})
            "
        );
        self.with_conn(|conn| {
            let params = rusqlite::params_from_iter(uris.iter().map(String::as_str));
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params, |row| {
                let uri = row.get::<_, String>(0)?;
                let updated_raw = row.get::<_, String>(7)?;
                let updated_at = parse_required_rfc3339(7, &updated_raw)?;
                let tags = row
                    .get::<_, String>(9)?
                    .split_whitespace()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                Ok(IndexRecord {
                    id: blake3::hash(uri.as_bytes()).to_hex().to_string(),
                    uri,
                    parent_uri: row.get(1)?,
                    is_leaf: row.get::<_, i64>(2)? != 0,
                    context_type: row.get(3)?,
                    name: row.get(4)?,
                    abstract_text: row.get(5)?,
                    content: row.get(6)?,
                    tags,
                    updated_at,
                    depth: i64_to_usize_saturating(row.get::<_, i64>(8)?),
                })
            })?;
            let mut out = std::collections::HashMap::new();
            for row in rows {
                let record = row?;
                out.insert(record.uri.clone(), record);
            }
            Ok(out)
        })
    }

    pub fn remove_search_document(&self, uri: &str) -> Result<()> {
        self.with_tx(|tx| {
            let doc_id = tx
                .query_row(
                    "SELECT id FROM search_docs WHERE uri = ?1",
                    params![uri],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?;

            if let Some(doc_id) = doc_id {
                tx.execute(
                    "DELETE FROM search_doc_tags WHERE doc_id = ?1",
                    params![doc_id],
                )?;
                tx.execute("DELETE FROM search_docs WHERE id = ?1", params![doc_id])?;
            }

            Ok(())
        })
    }

    pub fn remove_search_documents_with_prefix(&self, uri_prefix: &str) -> Result<()> {
        let like_pattern = format!("{uri_prefix}/%");
        self.with_tx(|tx| {
            tx.execute(
                r"
                DELETE FROM search_doc_tags
                WHERE doc_id IN (
                    SELECT id FROM search_docs
                    WHERE uri = ?1 OR uri LIKE ?2
                )
                ",
                params![uri_prefix, &like_pattern],
            )?;
            tx.execute(
                "DELETE FROM search_docs WHERE uri = ?1 OR uri LIKE ?2",
                params![uri_prefix, &like_pattern],
            )?;
            Ok(())
        })
    }
}

impl SqliteStateStore {
    fn persist_projection(&self, record: &IndexRecord, meta: &SearchProjectionMeta) -> Result<()> {
        let tags = normalize_tags(&record.tags);
        let tags_text = tags.join(" ");
        self.with_tx(|tx| {
            let doc_id: i64 = tx.query_row(
                r"
                INSERT INTO search_docs(
                    uri, parent_uri, is_leaf, context_type, name, abstract_text, content,
                    tags_text, mime, updated_at, depth, namespace, kind, event_time,
                    source_weight, freshness_bucket
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
                ON CONFLICT(uri) DO UPDATE SET
                  parent_uri=excluded.parent_uri,
                  is_leaf=excluded.is_leaf,
                  context_type=excluded.context_type,
                  name=excluded.name,
                  abstract_text=excluded.abstract_text,
                  content=excluded.content,
                  tags_text=excluded.tags_text,
                  mime=excluded.mime,
                  updated_at=excluded.updated_at,
                  depth=excluded.depth,
                  namespace=excluded.namespace,
                  kind=excluded.kind,
                  event_time=excluded.event_time,
                  source_weight=excluded.source_weight,
                  freshness_bucket=excluded.freshness_bucket
                RETURNING id
                ",
                params![
                    record.uri.as_str(),
                    record.parent_uri.as_deref(),
                    bool_to_i64(record.is_leaf),
                    record.context_type.as_str(),
                    record.name.as_str(),
                    record.abstract_text.as_str(),
                    record.content.as_str(),
                    tags_text,
                    meta.mime.as_deref(),
                    record.updated_at.to_rfc3339(),
                    usize_to_i64_saturating(record.depth),
                    meta.namespace.as_deref(),
                    meta.kind.as_deref(),
                    meta.event_time,
                    meta.source_weight,
                    meta.freshness_bucket,
                ],
                |row| row.get(0),
            )?;

            tx.execute(
                "DELETE FROM search_doc_tags WHERE doc_id = ?1",
                params![doc_id],
            )?;
            for tag in &tags {
                tx.execute(
                    "INSERT OR IGNORE INTO search_doc_tags(doc_id, tag) VALUES (?1, ?2)",
                    params![doc_id, tag],
                )?;
            }

            Ok(())
        })
    }
}

fn parse_required_rfc3339(idx: usize, raw: &str) -> rusqlite::Result<chrono::DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(raw)
        .map(|x| x.with_timezone(&Utc))
        .map_err(|err| rusqlite::Error::FromSqlConversionFailure(idx, Type::Text, Box::new(err)))
}

const fn bool_to_i64(value: bool) -> i64 {
    if value { 1 } else { 0 }
}

fn usize_to_i64_saturating(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn i64_to_usize_saturating(value: i64) -> usize {
    if value <= 0 {
        0
    } else {
        usize::try_from(value).unwrap_or(usize::MAX)
    }
}

fn with_projection_tags(
    tags: &[String],
    namespace: &str,
    kind: &str,
    event_time: Option<i64>,
    source_weight: f32,
    freshness_bucket: i64,
) -> Vec<String> {
    let mut all = tags.to_vec();
    all.push(format!("ns:{namespace}"));
    all.push(format!("kind:{kind}"));
    if let Some(t) = event_time {
        all.push(format!("event_time:{t}"));
    }
    all.push(format!("source_weight:{source_weight:.2}"));
    all.push(format!("freshness_bucket:{freshness_bucket}"));
    normalize_tags(&all)
}

fn build_search_doc_from_resource(
    record: &ResourceRecord,
    profile: &IngestProfile,
) -> Option<(IndexRecord, SearchProjectionMeta)> {
    if matches!(profile.index_policy, IndexPolicy::None) || record.tombstoned_at.is_some() {
        return None;
    }

    let name = record
        .title
        .clone()
        .or_else(|| record.uri.last_segment().map(ToString::to_string))
        .unwrap_or_else(|| record.resource_id.clone());
    let abstract_text = record
        .excerpt_text
        .clone()
        .or_else(|| record.title.clone())
        .unwrap_or_else(|| record.kind.as_str().to_string());
    let content = projection_content(
        profile.index_policy,
        record.title.as_deref(),
        record.excerpt_text.as_deref(),
        &record.tags,
        &record.attrs,
    );

    let index_record = IndexRecord {
        id: record.resource_id.clone(),
        uri: record.uri.to_string(),
        parent_uri: record.uri.parent().map(|uri| uri.to_string()),
        is_leaf: true,
        context_type: "resource".to_string(),
        name,
        abstract_text,
        content,
        tags: with_projection_tags(
            &record.tags,
            &record.namespace.as_path(),
            record.kind.as_str(),
            None,
            1.0,
            0,
        ),
        updated_at: chrono_from_unix(record.updated_at),
        depth: record.uri.segments().len(),
    };
    let meta = SearchProjectionMeta {
        namespace: Some(record.namespace.as_path()),
        kind: Some(record.kind.as_str().to_string()),
        event_time: None,
        source_weight: 1.0,
        freshness_bucket: 0,
        mime: record.mime.clone(),
    };
    Some((index_record, meta))
}

fn build_search_doc_from_event(
    record: &EventRecord,
    profile: &IngestProfile,
) -> Option<(IndexRecord, SearchProjectionMeta)> {
    if matches!(profile.index_policy, IndexPolicy::None) || record.tombstoned_at.is_some() {
        return None;
    }

    let now = Utc::now().timestamp();
    let fb = freshness_bucket(record.event_time, now);

    let name = record
        .title
        .clone()
        .unwrap_or_else(|| record.event_id.clone());
    let abstract_text = record
        .summary_text
        .clone()
        .or_else(|| record.title.clone())
        .unwrap_or_else(|| record.kind.as_str().to_string());
    let content = projection_content(
        profile.index_policy,
        record.title.as_deref(),
        record.summary_text.as_deref(),
        &record.tags,
        &record.attrs,
    );

    let index_record = IndexRecord {
        id: record.event_id.clone(),
        uri: record.uri.to_string(),
        parent_uri: record.uri.parent().map(|uri| uri.to_string()),
        is_leaf: true,
        context_type: "event".to_string(),
        name,
        abstract_text,
        content,
        tags: with_projection_tags(
            &record.tags,
            &record.namespace.as_path(),
            record.kind.as_str(),
            Some(record.event_time),
            0.9,
            fb,
        ),
        updated_at: chrono_from_unix(record.created_at),
        depth: record.uri.segments().len(),
    };
    let meta = SearchProjectionMeta {
        namespace: Some(record.namespace.as_path()),
        kind: Some(record.kind.as_str().to_string()),
        event_time: Some(record.event_time),
        source_weight: 0.9,
        freshness_bucket: fb,
        mime: Some("application/json".to_string()),
    };
    Some((index_record, meta))
}

fn projection_content(
    policy: IndexPolicy,
    title: Option<&str>,
    summary: Option<&str>,
    tags: &[String],
    attrs: &serde_json::Value,
) -> String {
    let title = title.unwrap_or_default().trim();
    let summary = summary.unwrap_or_default().trim();
    let tags_text = normalize_tags(tags).join(" ");
    let attrs_text = flatten_attrs(attrs);

    match policy {
        IndexPolicy::FullText => [title, summary, tags_text.as_str(), attrs_text.as_str()]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
        IndexPolicy::SummaryOnly => [title, summary]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
        IndexPolicy::MetadataOnly => [title, tags_text.as_str(), attrs_text.as_str()]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
        IndexPolicy::None => String::new(),
    }
}

fn flatten_attrs(attrs: &serde_json::Value) -> String {
    match attrs {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(value) => value.clone(),
        _ => attrs.to_string(),
    }
}

fn chrono_from_unix(timestamp: i64) -> chrono::DateTime<Utc> {
    chrono::DateTime::<Utc>::from_timestamp(timestamp, 0).unwrap_or(chrono::DateTime::UNIX_EPOCH)
}

fn freshness_bucket(event_time: i64, now: i64) -> i64 {
    let age_secs = (now - event_time).max(0);
    match age_secs {
        0..=86_399 => 0,
        86_400..=604_799 => 1,
        604_800..=2_592_000 => 2,
        _ => 3,
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use rusqlite::params;
    use tempfile::tempdir;

    use crate::models::{EventRecord, IngestProfile, NamespaceKey, ResourceRecord};
    use crate::uri::AxiomUri;

    use super::SqliteStateStore;

    #[test]
    fn resource_projection_persists_v3_metadata_columns() {
        let temp = tempdir().expect("tempdir");
        let store = SqliteStateStore::open(temp.path().join("state.db")).expect("open");
        let resource = ResourceRecord {
            resource_id: "res-1".to_string(),
            uri: AxiomUri::parse("axiom://resources/acme/contracts/auth").expect("uri"),
            namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
            kind: "contract".parse().expect("kind"),
            title: Some("Auth Contract".to_string()),
            mime: Some("text/markdown".to_string()),
            tags: vec!["contract".to_string()],
            attrs: serde_json::json!({"owner": "platform"}),
            object_uri: None,
            excerpt_text: Some("Defines auth flows".to_string()),
            content_hash: "hash".to_string(),
            tombstoned_at: None,
            created_at: 1_710_000_000,
            updated_at: 1_710_000_100,
        };

        let indexed = store
            .persist_resource_search_document(&resource, &IngestProfile::for_kind(&resource.kind))
            .expect("upsert projection");
        assert!(indexed);

        let row = store
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT namespace, kind, event_time, source_weight FROM search_docs WHERE uri = ?1",
                    params![resource.uri.to_string()],
                    |row| {
                        Ok((
                            row.get::<_, Option<String>>(0)?,
                            row.get::<_, Option<String>>(1)?,
                            row.get::<_, Option<i64>>(2)?,
                            row.get::<_, f64>(3)?,
                        ))
                    },
                )
                .map_err(Into::into)
            })
            .expect("row");

        assert_eq!(row.0.as_deref(), Some("acme/platform"));
        assert_eq!(row.1.as_deref(), Some("contract"));
        assert_eq!(row.2, None);
        assert_eq!(row.3, 1.0);
    }

    #[test]
    fn event_projection_supports_filtered_fts_queries() {
        let temp = tempdir().expect("tempdir");
        let store = SqliteStateStore::open(temp.path().join("state.db")).expect("open");
        let now = Utc::now().timestamp();

        let incident = EventRecord {
            event_id: "evt-1".to_string(),
            uri: AxiomUri::parse("axiom://events/acme/incidents/1").expect("uri"),
            namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
            kind: "incident".parse().expect("kind"),
            event_time: now - 60,
            title: Some("OAuth outage".to_string()),
            summary_text: Some("oauth tokens failed".to_string()),
            severity: Some("high".to_string()),
            actor_uri: None,
            subject_uri: None,
            run_id: None,
            session_id: None,
            tags: vec!["oauth".to_string()],
            attrs: serde_json::json!({"env": "prod"}),
            object_uri: None,
            content_hash: Some("hash-1".to_string()),
            tombstoned_at: None,
            created_at: now - 60,
        };
        let deploy = EventRecord {
            event_id: "evt-2".to_string(),
            uri: AxiomUri::parse("axiom://events/acme/deploys/2").expect("uri"),
            namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
            kind: "deploy".parse().expect("kind"),
            event_time: now - 7_200,
            title: Some("Deploy done".to_string()),
            summary_text: Some("oauth service rollout".to_string()),
            severity: None,
            actor_uri: None,
            subject_uri: None,
            run_id: None,
            session_id: None,
            tags: vec!["release".to_string()],
            attrs: serde_json::json!({}),
            object_uri: None,
            content_hash: None,
            tombstoned_at: None,
            created_at: now - 7_200,
        };

        store
            .persist_event_search_document(&incident, &IngestProfile::for_kind(&incident.kind))
            .expect("incident projection");
        store
            .persist_event_search_document(&deploy, &IngestProfile::for_kind(&deploy.kind))
            .expect("deploy projection");

        let hits = store
            .search_documents_fts_filtered(
                "oauth",
                Some(&NamespaceKey::parse("acme").expect("namespace")),
                Some(&"incident".parse().expect("kind")),
                Some(now - 300),
                10,
            )
            .expect("search");
        assert_eq!(hits, vec!["axiom://events/acme/incidents/1".to_string()]);
    }
}
