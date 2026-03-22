use super::*;

impl ContextDb {
    pub fn load_episodes(&self) -> Result<Vec<EpisodeRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select episode.stable_id, workspace.stable_id, episode.problem_signature, episode.status, episode.opened_at_ms, episode.closed_at_ms
             from episode
             left join workspace on workspace.id = episode.workspace_id
             order by episode.opened_at_ms, episode.stable_id",
        ).map_db_err()?;
        let rows = stmt
            .query_map([], |row| {
                Ok(EpisodeRow {
                    stable_id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    problem_signature: row.get(2)?,
                    status: crate::context_db::codec::parse_episode_status(row.get(3)?)?,
                    opened_at_ms: row.get(4)?,
                    closed_at_ms: row.get(5)?,
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }

    pub fn load_episode_members(&self) -> Result<Vec<EpisodeMemberRow>> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "select episode.stable_id, conv_turn.stable_id
             from episode_member
             join episode on episode.id = episode_member.episode_id
             join conv_turn on conv_turn.id = episode_member.turn_id",
            )
            .map_db_err()?;
        let rows = stmt
            .query_map([], |row| {
                Ok(EpisodeMemberRow {
                    episode_id: row.get(0)?,
                    turn_id: row.get(1)?,
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }

    pub fn load_insights(&self) -> Result<Vec<InsightRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select insight.stable_id, episode.stable_id, insight.kind, insight.summary, insight.normalized_text,
                    insight.extractor_version, insight.confidence, insight.stale
             from insight
             join episode on episode.id = insight.episode_id
             order by insight.stable_id",
        ).map_db_err()?;
        let rows = stmt
            .query_map([], |row| {
                Ok(InsightRow {
                    stable_id: row.get(0)?,
                    episode_id: row.get(1)?,
                    kind: crate::context_db::codec::parse_insight_kind(row.get(2)?)?,
                    summary: row.get(3)?,
                    normalized_text: row.get(4)?,
                    extractor_version: row.get(5)?,
                    confidence: row.get(6)?,
                    stale: row.get::<_, i64>(7)? != 0,
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }

    pub fn load_insight_anchors(&self) -> Result<Vec<InsightAnchorRow>> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "select insight.stable_id, evidence_anchor.stable_id
             from insight_anchor
             join insight on insight.id = insight_anchor.insight_id
             join evidence_anchor on evidence_anchor.id = insight_anchor.anchor_id
             order by insight.stable_id, evidence_anchor.stable_id",
            )
            .map_db_err()?;
        let rows = stmt
            .query_map([], |row| {
                Ok(InsightAnchorRow {
                    insight_id: row.get(0)?,
                    anchor_id: row.get(1)?,
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }

    pub fn load_verifications(&self) -> Result<Vec<VerificationRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select verification.stable_id, episode.stable_id, verification.kind, verification.status, verification.summary, evidence_anchor.stable_id
             from verification
             join episode on episode.id = verification.episode_id
             left join evidence_anchor on evidence_anchor.id = verification.evidence_id
             order by verification.stable_id",
        ).map_db_err()?;
        let rows = stmt
            .query_map([], |row| {
                Ok(VerificationRow {
                    stable_id: row.get(0)?,
                    episode_id: row.get(1)?,
                    kind: crate::context_db::codec::parse_verification_kind(row.get(2)?)?,
                    status: crate::context_db::codec::parse_verification_status(row.get(3)?)?,
                    summary: row.get(4)?,
                    evidence_id: row.get(5)?,
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }

    pub fn load_search_docs_redacted(&self) -> Result<Vec<SearchDocRedactedRow>> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "select search_doc_redacted.stable_id, episode.stable_id, search_doc_redacted.body
             from search_doc_redacted
             join episode on episode.id = search_doc_redacted.episode_id
             order by search_doc_redacted.stable_id",
            )
            .map_db_err()?;
        let rows = stmt
            .query_map([], |row| {
                Ok(SearchDocRedactedRow {
                    stable_id: row.get(0)?,
                    episode_id: row.get(1)?,
                    body: row.get(2)?,
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }
}
