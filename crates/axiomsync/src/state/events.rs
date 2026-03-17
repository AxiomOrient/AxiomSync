use rusqlite::{params, params_from_iter, types::Value};

use crate::error::Result;
use crate::models::{EventQuery, EventRecord};
use crate::uri::AxiomUri;

use super::SqliteStateStore;
use super::db::{
    normalize_tags, parse_json_column, parse_kind, parse_namespace, parse_optional_uri, parse_uri,
    push_namespace_range_filter,
};

impl SqliteStateStore {
    pub fn append_events(&self, batch: &[EventRecord]) -> Result<usize> {
        if batch.is_empty() {
            return Ok(0);
        }

        self.with_tx(|tx| {
            let mut stmt = tx.prepare(
                r"
                INSERT INTO events(
                    event_id, uri, namespace, kind, event_time, title, summary_text, severity,
                    actor_uri, subject_uri, run_id, session_id, tags_json, attrs_json,
                    object_uri, content_hash, tombstoned_at, created_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
                ON CONFLICT(event_id) DO UPDATE SET
                  uri = excluded.uri,
                  namespace = excluded.namespace,
                  kind = excluded.kind,
                  event_time = excluded.event_time,
                  title = excluded.title,
                  summary_text = excluded.summary_text,
                  severity = excluded.severity,
                  actor_uri = excluded.actor_uri,
                  subject_uri = excluded.subject_uri,
                  run_id = excluded.run_id,
                  session_id = excluded.session_id,
                  tags_json = excluded.tags_json,
                  attrs_json = excluded.attrs_json,
                  object_uri = excluded.object_uri,
                  content_hash = excluded.content_hash,
                  tombstoned_at = excluded.tombstoned_at,
                  created_at = excluded.created_at
                ",
            )?;

            batch.iter().try_fold(0usize, |acc, event| {
                let count = stmt.execute(params![
                    event.event_id,
                    event.uri.to_string(),
                    event.namespace.as_path(),
                    event.kind.as_str(),
                    event.event_time,
                    event.title,
                    event.summary_text,
                    event.severity,
                    event.actor_uri.as_ref().map(ToString::to_string),
                    event.subject_uri.as_ref().map(ToString::to_string),
                    event.run_id,
                    event.session_id,
                    serde_json::to_string(&normalize_tags(&event.tags))?,
                    serde_json::to_string(&event.attrs)?,
                    event.object_uri.as_ref().map(ToString::to_string),
                    event.content_hash,
                    event.tombstoned_at,
                    event.created_at,
                ])?;
                Ok(acc.saturating_add(count))
            })
        })
    }

    pub fn tombstone_event(&self, uri: &AxiomUri, tombstoned_at: i64) -> Result<bool> {
        self.with_conn(|conn| {
            let affected = conn.execute(
                "UPDATE events SET tombstoned_at = ?2 WHERE uri = ?1",
                params![uri.to_string(), tombstoned_at],
            )?;
            Ok(affected > 0)
        })
    }

    pub fn query_events(&self, query: EventQuery) -> Result<Vec<EventRecord>> {
        self.with_conn(|conn| {
            let mut sql = String::from(
                r"
                SELECT event_id, uri, namespace, kind, event_time, title, summary_text, severity,
                       actor_uri, subject_uri, run_id, session_id, tags_json, attrs_json,
                       object_uri, content_hash, tombstoned_at, created_at
                FROM events
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
            if let Some(start) = query.start_time {
                sql.push_str(" AND event_time >= ?");
                params.push(Value::Integer(start));
            }
            if let Some(end) = query.end_time {
                sql.push_str(" AND event_time <= ?");
                params.push(Value::Integer(end));
            }

            sql.push_str(" ORDER BY event_time DESC, uri ASC");
            if let Some(limit) = query.limit {
                sql.push_str(" LIMIT ?");
                params.push(Value::Integer(i64::try_from(limit).unwrap_or(i64::MAX)));
            }

            let mut stmt = conn.prepare(&sql)?;
            stmt.query_map(params_from_iter(params), map_event_record)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(Into::into)
        })
    }

    pub fn compact_events_into_archive(
        &self,
        event_ids: &[String],
        archive_id: &str,
        archive_object_uri: &AxiomUri,
        archived_at: i64,
    ) -> Result<usize> {
        if event_ids.is_empty() {
            return Ok(0);
        }

        self.with_tx(|tx| {
            let mut stmt = tx.prepare(
                r"
                UPDATE events
                SET attrs_json   = ?2,
                    object_uri   = ?3,
                    content_hash = NULL
                WHERE event_id = ?1
                ",
            )?;

            event_ids.iter().try_fold(0usize, |acc, event_id| {
                let archived_attrs = serde_json::json!({
                    "archived": {
                        "archive_id": archive_id,
                        "object_uri": archive_object_uri.to_string(),
                        "archived_at": archived_at,
                    }
                });
                let count = stmt.execute(params![
                    event_id,
                    serde_json::to_string(&archived_attrs)?,
                    archive_object_uri.to_string(),
                ])?;
                Ok(acc.saturating_add(count))
            })
        })
    }
}

fn map_event_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<EventRecord> {
    Ok(EventRecord {
        event_id: row.get(0)?,
        uri: parse_uri(row.get::<_, String>(1)?)?,
        namespace: parse_namespace(row.get::<_, String>(2)?)?,
        kind: parse_kind(row.get::<_, String>(3)?)?,
        event_time: row.get(4)?,
        title: row.get(5)?,
        summary_text: row.get(6)?,
        severity: row.get(7)?,
        actor_uri: parse_optional_uri(row.get(8)?)?,
        subject_uri: parse_optional_uri(row.get(9)?)?,
        run_id: row.get(10)?,
        session_id: row.get(11)?,
        tags: parse_json_column(row.get::<_, String>(12)?)?,
        attrs: parse_json_column(row.get::<_, String>(13)?)?,
        object_uri: parse_optional_uri(row.get(14)?)?,
        content_hash: row.get(15)?,
        tombstoned_at: row.get(16)?,
        created_at: row.get(17)?,
    })
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::models::{EventQuery, EventRecord, NamespaceKey};
    use crate::uri::AxiomUri;

    use super::SqliteStateStore;

    #[test]
    fn append_and_query_events_by_namespace_kind_and_time() {
        let temp = tempdir().expect("tempdir");
        let store = SqliteStateStore::open(temp.path().join("state.db")).expect("open");

        let batch = vec![
            EventRecord {
                event_id: "evt-1".to_string(),
                uri: AxiomUri::parse("axiom://events/acme/incidents/1").expect("uri"),
                namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
                kind: "incident".parse().expect("kind"),
                event_time: 100,
                title: Some("incident".to_string()),
                summary_text: Some("oauth outage".to_string()),
                severity: Some("high".to_string()),
                actor_uri: None,
                subject_uri: None,
                run_id: Some("run-1".to_string()),
                session_id: None,
                tags: vec!["OAuth".to_string()],
                attrs: serde_json::json!({"env": "prod"}),
                object_uri: None,
                content_hash: Some("hash-evt-1".to_string()),
                tombstoned_at: None,
                created_at: 101,
            },
            EventRecord {
                event_id: "evt-2".to_string(),
                uri: AxiomUri::parse("axiom://events/acme/deploys/2").expect("uri"),
                namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
                kind: "deploy".parse().expect("kind"),
                event_time: 120,
                title: Some("deploy".to_string()),
                summary_text: Some("deploy finished".to_string()),
                severity: None,
                actor_uri: None,
                subject_uri: None,
                run_id: Some("run-2".to_string()),
                session_id: None,
                tags: vec!["release".to_string()],
                attrs: serde_json::json!({}),
                object_uri: None,
                content_hash: None,
                tombstoned_at: None,
                created_at: 121,
            },
        ];

        let inserted = store.append_events(&batch).expect("append");
        assert_eq!(inserted, 2);

        let results = store
            .query_events(EventQuery {
                namespace_prefix: Some(NamespaceKey::parse("acme").expect("namespace")),
                kind: Some("incident".parse().expect("kind")),
                start_time: Some(90),
                end_time: Some(110),
                limit: Some(5),
                include_tombstoned: false,
            })
            .expect("query");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].event_id, "evt-1");
        assert_eq!(results[0].tags, vec!["oauth".to_string()]);
    }

    #[test]
    fn compact_events_into_archive_rewrites_event_payload_to_archive_reference() {
        let temp = tempdir().expect("tempdir");
        let store = SqliteStateStore::open(temp.path().join("state.db")).expect("open");
        let archive_uri = AxiomUri::parse("axiom://events/_archive/acme/log/archive-1.jsonl")
            .expect("archive uri");
        let event = EventRecord {
            event_id: "evt-1".to_string(),
            uri: AxiomUri::parse("axiom://events/acme/logs/1").expect("uri"),
            namespace: NamespaceKey::parse("acme/platform").expect("namespace"),
            kind: "log".parse().expect("kind"),
            event_time: 100,
            title: Some("log".to_string()),
            summary_text: Some("oauth retry".to_string()),
            severity: None,
            actor_uri: None,
            subject_uri: None,
            run_id: Some("run-1".to_string()),
            session_id: None,
            tags: vec!["oauth".to_string()],
            attrs: serde_json::json!({"raw_payload": "line"}),
            object_uri: None,
            content_hash: None,
            tombstoned_at: None,
            created_at: 101,
        };

        store
            .append_events(std::slice::from_ref(&event))
            .expect("append event");
        let updated = store
            .compact_events_into_archive(
                std::slice::from_ref(&event.event_id),
                "archive-1",
                &archive_uri,
                200,
            )
            .expect("compact events");
        assert_eq!(updated, 1);

        let queried = store
            .query_events(EventQuery {
                namespace_prefix: Some(NamespaceKey::parse("acme").expect("namespace")),
                kind: Some("log".parse().expect("kind")),
                start_time: None,
                end_time: None,
                limit: Some(10),
                include_tombstoned: false,
            })
            .expect("query events");
        assert_eq!(queried.len(), 1);
        assert_eq!(queried[0].object_uri.as_ref(), Some(&archive_uri));
        assert_eq!(
            queried[0]
                .attrs
                .get("archived")
                .and_then(|value| value.get("archive_id"))
                .and_then(|value| value.as_str()),
            Some("archive-1")
        );
    }
}
