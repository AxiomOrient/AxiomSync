use rusqlite::{OptionalExtension, params};

use crate::error::Result;
use crate::models::SessionRecord;

use super::SqliteStateStore;

impl SqliteStateStore {
    pub fn save_session(&self, record: SessionRecord) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                r"
                INSERT INTO sessions(session_id, uri, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4)
                ON CONFLICT(session_id) DO UPDATE SET
                  uri = excluded.uri,
                  created_at = excluded.created_at,
                  updated_at = excluded.updated_at
                ",
                params![
                    record.session_id,
                    record.uri,
                    record.created_at,
                    record.updated_at
                ],
            )?;
            Ok(())
        })
    }

    pub fn load_session(&self, session_id: &str) -> Result<Option<SessionRecord>> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT session_id, uri, created_at, updated_at FROM sessions WHERE session_id = ?1",
                params![session_id],
                map_session_record,
            )
            .optional()
            .map_err(Into::into)
        })
    }

    pub fn list_sessions(&self, limit: Option<usize>) -> Result<Vec<SessionRecord>> {
        self.with_conn(|conn| {
            let mut sql = String::from(
                "SELECT session_id, uri, created_at, updated_at FROM sessions ORDER BY updated_at DESC",
            );
            if let Some(limit) = limit {
                sql.push_str(" LIMIT ?1");
                let mut stmt = conn.prepare(&sql)?;
                let rows = stmt.query_map(
                    params![i64::try_from(limit).unwrap_or(i64::MAX)],
                    map_session_record,
                )?;
                rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
            } else {
                let mut stmt = conn.prepare(&sql)?;
                let rows = stmt.query_map([], map_session_record)?;
                rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
            }
        })
    }

    pub fn delete_session(&self, session_id: &str) -> Result<bool> {
        self.with_conn(|conn| {
            let affected = conn.execute(
                "DELETE FROM sessions WHERE session_id = ?1",
                params![session_id],
            )?;
            Ok(affected > 0)
        })
    }
}

fn map_session_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<SessionRecord> {
    Ok(SessionRecord {
        session_id: row.get(0)?,
        uri: row.get(1)?,
        created_at: row.get(2)?,
        updated_at: row.get(3)?,
    })
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::models::SessionRecord;

    use super::SqliteStateStore;

    #[test]
    fn save_load_delete_session_roundtrip() {
        let temp = tempdir().expect("tempdir");
        let store = SqliteStateStore::open(temp.path().join("state.db")).expect("open");

        let record = SessionRecord {
            session_id: "sess-1".to_string(),
            uri: "axiom://session/sess-1".to_string(),
            created_at: 1_710_000_000,
            updated_at: 1_710_000_100,
        };

        store.save_session(record.clone()).expect("save");

        let loaded = store.load_session("sess-1").expect("load").expect("exists");
        assert_eq!(loaded.session_id, "sess-1");
        assert_eq!(loaded.uri, "axiom://session/sess-1");
        assert_eq!(loaded.updated_at, 1_710_000_100);

        let deleted = store.delete_session("sess-1").expect("delete");
        assert!(deleted);

        assert!(
            store
                .load_session("sess-1")
                .expect("load after delete")
                .is_none()
        );
    }

    #[test]
    fn list_sessions_returns_all_ordered_by_updated_at() {
        let temp = tempdir().expect("tempdir");
        let store = SqliteStateStore::open(temp.path().join("state.db")).expect("open");

        store
            .save_session(SessionRecord {
                session_id: "sess-a".to_string(),
                uri: "axiom://session/sess-a".to_string(),
                created_at: 100,
                updated_at: 200,
            })
            .expect("save a");

        store
            .save_session(SessionRecord {
                session_id: "sess-b".to_string(),
                uri: "axiom://session/sess-b".to_string(),
                created_at: 100,
                updated_at: 300,
            })
            .expect("save b");

        let sessions = store.list_sessions(None).expect("list");
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].session_id, "sess-b");
        assert_eq!(sessions[1].session_id, "sess-a");
    }

    #[test]
    fn list_sessions_respects_limit() {
        let temp = tempdir().expect("tempdir");
        let store = SqliteStateStore::open(temp.path().join("state.db")).expect("open");

        for i in 0..5 {
            store
                .save_session(SessionRecord {
                    session_id: format!("sess-{i}"),
                    uri: format!("axiom://session/sess-{i}"),
                    created_at: i,
                    updated_at: i,
                })
                .expect("save");
        }

        let sessions = store.list_sessions(Some(2)).expect("list with limit");
        assert_eq!(sessions.len(), 2);
    }
}
