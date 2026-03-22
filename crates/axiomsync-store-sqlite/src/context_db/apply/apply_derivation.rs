use super::*;

impl ContextDb {
    pub(crate) fn apply_derivation_in_tx(
        tx: &rusqlite::Transaction<'_>,
        plan: &DerivePlan,
    ) -> Result<serde_json::Value> {
        plan.validate()?;
        tx.execute("delete from search_doc_redacted", [])
            .map_db_err()?;
        tx.execute("delete from insight_anchor", []).map_db_err()?;
        tx.execute("delete from verification", []).map_db_err()?;
        tx.execute("delete from insight", []).map_db_err()?;
        tx.execute("delete from episode_member", []).map_db_err()?;
        tx.execute("delete from episode", []).map_db_err()?;

        let workspace_ids = stable_id_map(tx, "workspace")?;
        let turn_ids = stable_id_map(tx, "conv_turn")?;
        let anchor_ids = stable_id_map(tx, "evidence_anchor")?;

        for episode in &plan.episodes {
            tx.execute(
                "insert into episode (stable_id, workspace_id, problem_signature, status, opened_at_ms, closed_at_ms)
                 values (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    episode.stable_id,
                    lookup_fk(&workspace_ids, episode.workspace_id.as_deref())?,
                    episode.problem_signature,
                    episode.status.as_str(),
                    episode.opened_at_ms,
                    episode.closed_at_ms
                ],
            )
            .map_db_err()?;
        }
        let episode_ids = stable_id_map(tx, "episode")?;

        for member in &plan.episode_members {
            tx.execute(
                "insert into episode_member (episode_id, turn_id) values (?1, ?2)",
                params![
                    lookup_fk(&episode_ids, Some(member.episode_id.as_str()))?,
                    lookup_fk(&turn_ids, Some(member.turn_id.as_str()))?,
                ],
            )
            .map_db_err()?;
        }

        for insight in &plan.insights {
            tx.execute(
                "insert into insight (stable_id, episode_id, kind, summary, normalized_text, extractor_version, confidence, stale)
                 values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    insight.stable_id,
                    lookup_fk(&episode_ids, Some(insight.episode_id.as_str()))?,
                    insight.kind.as_str(),
                    insight.summary,
                    insight.normalized_text,
                    insight.extractor_version,
                    insight.confidence,
                    if insight.stale { 1 } else { 0 }
                ],
            )
            .map_db_err()?;
        }
        let insight_ids = stable_id_map(tx, "insight")?;

        for link in &plan.insight_anchors {
            tx.execute(
                "insert into insight_anchor (insight_id, anchor_id) values (?1, ?2)",
                params![
                    lookup_fk(&insight_ids, Some(link.insight_id.as_str()))?,
                    lookup_fk(&anchor_ids, Some(link.anchor_id.as_str()))?,
                ],
            )
            .map_db_err()?;
        }

        for verification in &plan.verifications {
            tx.execute(
                "insert into verification (stable_id, episode_id, kind, status, summary, evidence_id)
                 values (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    verification.stable_id,
                    lookup_fk(&episode_ids, Some(verification.episode_id.as_str()))?,
                    verification.kind.as_str(),
                    verification.status.as_str(),
                    verification.summary,
                    lookup_fk(&anchor_ids, verification.evidence_id.as_deref())?,
                ],
            )
            .map_db_err()?;
        }
        for doc in &plan.search_docs_redacted {
            tx.execute(
                "insert into search_doc_redacted (stable_id, episode_id, body)
                 values (?1, ?2, ?3)",
                params![
                    doc.stable_id,
                    lookup_fk(&episode_ids, Some(doc.episode_id.as_str()))?,
                    doc.body,
                ],
            )
            .map_db_err()?;
        }
        tx.execute(
            "insert into insight_fts(insight_fts) values ('rebuild')",
            [],
        )
        .map_db_err()?;
        tx.execute(
            "insert into search_doc_redacted_fts(search_doc_redacted_fts) values ('rebuild')",
            [],
        )
        .map_db_err()?;
        Ok(serde_json::json!({
            "episodes": plan.episodes.len(),
            "episode_members": plan.episode_members.len(),
            "insights": plan.insights.len(),
            "insight_anchors": plan.insight_anchors.len(),
            "verifications": plan.verifications.len(),
            "search_docs_redacted": plan.search_docs_redacted.len(),
        }))
    }
}
