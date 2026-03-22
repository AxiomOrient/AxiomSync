use super::*;

impl ContextDb {
    pub fn get_thread(&self, session_id: &str) -> Result<ThreadView> {
        let session = self
            .load_sessions()?
            .into_iter()
            .find(|row| row.stable_id == session_id)
            .ok_or_else(|| AxiomError::NotFound(format!("thread {session_id}")))?;
        let turns = self
            .load_turns()?
            .into_iter()
            .filter(|turn| turn.session_id == session_id)
            .collect::<Vec<_>>();
        let items = self.load_items()?;
        let artifacts = self.load_artifacts()?;
        let mut turn_views = Vec::new();
        for turn in turns {
            let item_views = items
                .iter()
                .filter(|item| item.turn_id == turn.stable_id)
                .cloned()
                .map(|item| ThreadItemView {
                    artifacts: artifacts
                        .iter()
                        .filter(|artifact| artifact.item_id == item.stable_id)
                        .cloned()
                        .collect(),
                    item,
                })
                .collect();
            turn_views.push(ThreadTurnView {
                turn,
                items: item_views,
            });
        }
        Ok(ThreadView {
            session,
            turns: turn_views,
        })
    }

    pub fn get_evidence(&self, evidence_id: &str) -> Result<crate::domain::EvidenceView> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select evidence_anchor.stable_id, conv_item.stable_id, evidence_anchor.selector_type, evidence_anchor.selector_json, evidence_anchor.quoted_text,
                    conv_item.turn_id, conv_item.item_type, conv_item.tool_name, conv_item.body_text, conv_item.payload_json
             from evidence_anchor
             join conv_item on conv_item.id = evidence_anchor.item_id
             where evidence_anchor.stable_id = ?1",
        ).map_db_err()?;
        stmt.query_row([evidence_id], |row| {
            Ok(crate::domain::EvidenceView {
                evidence: EvidenceAnchorRow {
                    stable_id: row.get(0)?,
                    item_id: row.get(1)?,
                    selector_type: crate::context_db::codec::parse_selector_type(row.get(2)?)?,
                    selector_json: row.get(3)?,
                    quoted_text: row.get(4)?,
                },
                item: ConvItemRow {
                    stable_id: row.get(1)?,
                    turn_id: row.get(5)?,
                    item_type: crate::context_db::codec::parse_item_type(row.get(6)?)?,
                    tool_name: row.get(7)?,
                    body_text: row.get(8)?,
                    payload_json: row.get(9)?,
                },
            })
        })
        .map_err(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => {
                AxiomError::NotFound(format!("evidence {evidence_id}"))
            }
            other => sqlite_error(other),
        })
    }

    pub fn load_episode_connectors(&self) -> Result<Vec<EpisodeConnectorRow>> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "select episode.stable_id, conv_session.connector, conv_turn.turn_index
             from episode
             join episode_member on episode_member.episode_id = episode.id
             join conv_turn on conv_turn.id = episode_member.turn_id
             join conv_session on conv_session.id = conv_turn.session_id
             order by episode.stable_id asc, conv_turn.turn_index asc",
            )
            .map_db_err()?;
        let rows = stmt
            .query_map([], |row| {
                Ok(EpisodeConnectorRow {
                    episode_id: row.get(0)?,
                    connector: row.get(1)?,
                    turn_index: row.get(2)?,
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }

    pub fn episode_workspace_id(&self, episode_id: &str) -> Result<Option<String>> {
        let conn = self.connect()?;
        conn.query_row(
            "select workspace.stable_id
             from episode
             left join workspace on workspace.id = episode.workspace_id
             where episode.stable_id = ?1",
            [episode_id],
            |row| row.get(0),
        )
        .optional()
        .map_db_err()
    }

    pub fn thread_workspace_id(&self, thread_id: &str) -> Result<Option<String>> {
        let conn = self.connect()?;
        conn.query_row(
            "select workspace.stable_id
             from conv_session
             left join workspace on workspace.id = conv_session.workspace_id
             where conv_session.stable_id = ?1",
            [thread_id],
            |row| row.get(0),
        )
        .optional()
        .map_db_err()
    }

    pub fn evidence_workspace_id(&self, evidence_id: &str) -> Result<Option<String>> {
        let conn = self.connect()?;
        conn.query_row(
            "select workspace.stable_id
             from evidence_anchor
             join conv_item on conv_item.id = evidence_anchor.item_id
             join conv_turn on conv_turn.id = conv_item.turn_id
             join conv_session on conv_session.id = conv_turn.session_id
             left join workspace on workspace.id = conv_session.workspace_id
             where evidence_anchor.stable_id = ?1",
            [evidence_id],
            |row| row.get(0),
        )
        .optional()
        .map_db_err()
    }

    pub fn load_episode_search_fts_rows(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchEpisodeFtsRow>> {
        let Some(normalized_query) = crate::domain::normalize_fts_query(query) else {
            return Ok(Vec::new());
        };
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select episode_id, workspace_id, connector, status, matched_kind, matched_summary, pass_boost
             from (
               select episode.stable_id as episode_id,
                      workspace.stable_id as workspace_id,
                      conv_session.connector as connector,
                      episode.status as status,
                      insight.kind as matched_kind,
                      insight.summary as matched_summary,
                      max(case when verification.status = 'pass' then 1 else 0 end) as pass_boost
               from insight_fts
               join insight on insight.id = insight_fts.rowid
               join episode on episode.id = insight.episode_id
               left join workspace on workspace.id = episode.workspace_id
               left join verification on verification.episode_id = episode.id
               left join episode_member on episode_member.episode_id = episode.id
               left join conv_turn on conv_turn.id = episode_member.turn_id
               left join conv_session on conv_session.id = conv_turn.session_id
               where insight_fts match ?1
               group by episode.id, workspace.stable_id, conv_session.connector, episode.status, insight.id
               union all
               select episode.stable_id as episode_id,
                      workspace.stable_id as workspace_id,
                      conv_session.connector as connector,
                      episode.status as status,
                      null as matched_kind,
                      search_doc_redacted.body as matched_summary,
                      max(case when verification.status = 'pass' then 1 else 0 end) as pass_boost
               from search_doc_redacted_fts
               join search_doc_redacted on search_doc_redacted.id = search_doc_redacted_fts.rowid
               join episode on episode.id = search_doc_redacted.episode_id
               left join workspace on workspace.id = episode.workspace_id
               left join verification on verification.episode_id = episode.id
               left join episode_member on episode_member.episode_id = episode.id
               left join conv_turn on conv_turn.id = episode_member.turn_id
               left join conv_session on conv_session.id = conv_turn.session_id
               where search_doc_redacted_fts match ?1
               group by episode.id, workspace.stable_id, conv_session.connector, episode.status, search_doc_redacted.id
             )
             order by pass_boost desc, episode_id asc
             limit ?2",
        ).map_db_err()?;
        let rows = stmt
            .query_map(params![normalized_query, limit as i64], |row| {
                Ok(SearchEpisodeFtsRow {
                    episode_id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    connector: row.get(2)?,
                    status: crate::context_db::codec::parse_episode_status(row.get(3)?)?,
                    matched_kind: row
                        .get::<_, Option<String>>(4)?
                        .map(crate::context_db::codec::parse_insight_kind)
                        .transpose()?,
                    matched_summary: row.get(5)?,
                    pass_boost: row.get::<_, i64>(6)? != 0,
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }

    pub fn load_command_search_candidates(&self) -> Result<Vec<SearchCommandCandidateRow>> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "select episode.stable_id, workspace.stable_id, insight.summary
             from insight
             join episode on episode.id = insight.episode_id
             left join workspace on workspace.id = episode.workspace_id
             where insight.kind = 'command'
             order by episode.stable_id asc, insight.summary asc",
            )
            .map_db_err()?;
        let rows = stmt
            .query_map([], |row| {
                Ok(SearchCommandCandidateRow {
                    episode_id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    command: row.get(2)?,
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }

    pub fn load_episode_evidence_search_rows(&self) -> Result<Vec<EpisodeEvidenceSearchRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select episode.stable_id,
                    workspace.stable_id,
                    conv_session.connector,
                    episode.status,
                    evidence_anchor.stable_id,
                    evidence_anchor.quoted_text,
                    conv_item.body_text,
                    max(case when verification.status = 'pass' then 1 else 0 end) as pass_boost
             from episode
             join episode_member on episode_member.episode_id = episode.id
             join conv_turn on conv_turn.id = episode_member.turn_id
             join conv_session on conv_session.id = conv_turn.session_id
             join conv_item on conv_item.turn_id = conv_turn.id
             join evidence_anchor on evidence_anchor.item_id = conv_item.id
             left join workspace on workspace.id = episode.workspace_id
             left join verification on verification.episode_id = episode.id
             group by episode.id, workspace.stable_id, conv_session.connector, episode.status, evidence_anchor.id, conv_item.id
             order by episode.stable_id asc, evidence_anchor.stable_id asc",
        ).map_db_err()?;
        let rows = stmt
            .query_map([], |row| {
                Ok(EpisodeEvidenceSearchRow {
                    episode_id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    connector: row.get(2)?,
                    status: crate::context_db::codec::parse_episode_status(row.get(3)?)?,
                    evidence_id: row.get(4)?,
                    quoted_text: row.get(5)?,
                    body_text: row.get(6)?,
                    pass_boost: row.get::<_, i64>(7)? != 0,
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }
}
