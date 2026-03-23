use serde_json::{Value, json};

use crate::kernel::AxiomSync;
use crate::ports::{McpResourcePort, McpToolPort};

impl McpResourcePort for AxiomSync {
    fn mcp_resources(&self) -> crate::Result<Value> {
        Ok(json!({
            "resources": [
                {"uri": "session://{id}", "name": "Session (canonical)"},
                {"uri": "episode://{id}", "name": "Episode (canonical)"},
                {"uri": "insight://{id}", "name": "Insight (canonical)"},
                {"uri": "procedure://{id}", "name": "Procedure (canonical)"},
                {"uri": "axiom://sessions/{id}", "name": "Session"},
                {"uri": "axiom://entries/{id}", "name": "Entry"},
                {"uri": "axiom://artifacts/{id}", "name": "Artifact"},
                {"uri": "axiom://anchors/{id}", "name": "Anchor"},
                {"uri": "axiom://episodes/{id}", "name": "Episode"},
                {"uri": "axiom://claims/{id}", "name": "Claim"},
                {"uri": "axiom://insights/{id}", "name": "Insight"},
                {"uri": "axiom://procedures/{id}", "name": "Procedure"},
                {"uri": "axiom://cases/{id}", "name": "Case (compat)"},
                {"uri": "axiom://threads/{id}", "name": "Thread (compat)"},
                {"uri": "axiom://runbooks/{id}", "name": "Runbook (compat)"},
                {"uri": "axiom://tasks/{id}", "name": "Task (compat)"}
            ]
        }))
    }

    fn mcp_roots(&self, bound_workspace_id: Option<&str>) -> crate::Result<Value> {
        Ok(json!({
            "roots": [{
                "uri": format!("file://{}", self.root().display()),
                "name": bound_workspace_id.unwrap_or("axiomsync")
            }]
        }))
    }

    fn read_mcp_resource(&self, uri: &str) -> crate::Result<Value> {
        if let Some(id) = strip_one_of(uri, &["session://", "axiom://sessions/"]) {
            return Ok(serde_json::to_value(self.get_session(id)?)?);
        }
        if let Some(id) = strip_one_of(uri, &["episode://", "axiom://episodes/"]) {
            return Ok(serde_json::to_value(self.get_episode(id)?)?);
        }
        if let Some(id) = strip_one_of(uri, &["insight://", "axiom://insights/"]) {
            return Ok(serde_json::to_value(
                self.get_evidence_bundle("insight", id)?,
            )?);
        }
        if let Some(id) = strip_one_of(uri, &["procedure://", "axiom://procedures/"]) {
            return Ok(serde_json::to_value(self.get_procedure(id)?)?);
        }
        if let Some(id) = uri.strip_prefix("axiom://sessions/") {
            return Ok(serde_json::to_value(self.get_session(id)?)?);
        }
        if let Some(id) = uri.strip_prefix("axiom://entries/") {
            return Ok(serde_json::to_value(self.get_entry(id)?)?);
        }
        if let Some(id) = uri.strip_prefix("axiom://artifacts/") {
            return Ok(serde_json::to_value(self.get_artifact(id)?)?);
        }
        if let Some(id) = uri.strip_prefix("axiom://anchors/") {
            return Ok(serde_json::to_value(self.get_anchor(id)?)?);
        }
        if let Some(id) = uri.strip_prefix("axiom://episodes/") {
            return Ok(serde_json::to_value(self.get_episode(id)?)?);
        }
        if let Some(id) = uri.strip_prefix("axiom://claims/") {
            return Ok(serde_json::to_value(self.get_claim(id)?)?);
        }
        if let Some(id) = uri.strip_prefix("axiom://cases/") {
            return Ok(serde_json::to_value(self.get_case(id)?)?);
        }
        if let Some(id) = uri.strip_prefix("axiom://threads/") {
            return Ok(serde_json::to_value(self.get_thread(id)?)?);
        }
        if let Some(id) = uri.strip_prefix("axiom://runbooks/") {
            return Ok(serde_json::to_value(self.get_runbook(id)?)?);
        }
        if let Some(id) = uri.strip_prefix("axiom://tasks/") {
            return Ok(serde_json::to_value(self.get_task(id)?)?);
        }
        Err(crate::AxiomError::Validation(format!(
            "unknown resource {uri}"
        )))
    }

    fn resource_workspace_requirement(&self, uri: &str) -> crate::Result<Option<String>> {
        if let Some(id) = strip_one_of(uri, &["session://", "axiom://sessions/"]) {
            return self.session_workspace_id(id);
        }
        if let Some(id) = strip_one_of(uri, &["episode://", "axiom://episodes/"]) {
            return self.episode_workspace_id(id);
        }
        if let Some(id) = strip_one_of(uri, &["insight://", "axiom://insights/"]) {
            return self.insight_workspace_id(id);
        }
        if let Some(id) = strip_one_of(uri, &["procedure://", "axiom://procedures/"]) {
            return self.procedure_workspace_id(id);
        }
        if let Some(id) = uri.strip_prefix("axiom://sessions/") {
            return self.session_workspace_id(id);
        }
        if let Some(id) = uri.strip_prefix("axiom://artifacts/") {
            return self.artifact_workspace_id(id);
        }
        if let Some(id) = uri.strip_prefix("axiom://anchors/") {
            return self.anchor_workspace_id(id);
        }
        if let Some(id) = uri.strip_prefix("axiom://episodes/") {
            return self.episode_workspace_id(id);
        }
        if let Some(id) = uri.strip_prefix("axiom://cases/") {
            return self.episode_workspace_id(id);
        }
        if let Some(id) = uri.strip_prefix("axiom://threads/") {
            return self.session_workspace_id(id);
        }
        if let Some(id) = uri.strip_prefix("axiom://runbooks/") {
            return self.episode_workspace_id(id);
        }
        if let Some(id) = uri.strip_prefix("axiom://tasks/") {
            return self.task_workspace_id(id);
        }
        Ok(None)
    }
}

impl McpToolPort for AxiomSync {
    fn mcp_tools(&self) -> crate::Result<Value> {
        Ok(json!({
            "tools": [
                {"name": "search_entries"},
                {"name": "search_episodes"},
                {"name": "search_docs"},
                {"name": "search_insights"},
                {"name": "search_claims"},
                {"name": "search_procedures"},
                {"name": "find_fix"},
                {"name": "find_decision"},
                {"name": "find_runbook"},
                {"name": "get_evidence_bundle"},
                {"name": "get_session"},
                {"name": "get_entry"},
                {"name": "get_artifact"},
                {"name": "get_anchor"},
                {"name": "get_case"},
                {"name": "get_thread"},
                {"name": "get_runbook"},
                {"name": "get_task"}
            ]
        }))
    }

    fn call_mcp_tool(
        &self,
        name: &str,
        arguments: &Value,
        _bound_workspace_id: Option<&str>,
    ) -> crate::Result<Value> {
        match name {
            "search_entries" => Ok(serde_json::to_value(
                self.search_entries(serde_json::from_value(arguments.clone())?)?,
            )?),
            "search_episodes" => Ok(serde_json::to_value(
                self.search_episodes(serde_json::from_value(arguments.clone())?)?,
            )?),
            "search_docs" => Ok(serde_json::to_value(
                self.search_docs(serde_json::from_value(arguments.clone())?)?,
            )?),
            "search_insights" => Ok(serde_json::to_value(
                self.search_insights(serde_json::from_value(arguments.clone())?)?,
            )?),
            "search_claims" => Ok(serde_json::to_value(
                self.search_claims(serde_json::from_value(arguments.clone())?)?,
            )?),
            "search_procedures" => Ok(serde_json::to_value(
                self.search_procedures(serde_json::from_value(arguments.clone())?)?,
            )?),
            "find_fix" => Ok(serde_json::to_value(
                self.find_fix(serde_json::from_value(arguments.clone())?)?,
            )?),
            "find_decision" => Ok(serde_json::to_value(
                self.find_decision(serde_json::from_value(arguments.clone())?)?,
            )?),
            "find_runbook" => Ok(serde_json::to_value(
                self.find_runbook(serde_json::from_value(arguments.clone())?)?,
            )?),
            "get_evidence_bundle" => Ok(serde_json::to_value(
                self.get_evidence_bundle(subject_kind_arg(arguments)?, subject_id_arg(arguments)?)?,
            )?),
            "get_session" => Ok(serde_json::to_value(self.get_session(id_arg(arguments)?)?)?),
            "get_entry" => Ok(serde_json::to_value(self.get_entry(id_arg(arguments)?)?)?),
            "get_artifact" => Ok(serde_json::to_value(
                self.get_artifact(id_arg(arguments)?)?,
            )?),
            "get_anchor" => Ok(serde_json::to_value(self.get_anchor(id_arg(arguments)?)?)?),
            "get_case" => Ok(serde_json::to_value(self.get_case(id_arg(arguments)?)?)?),
            "get_thread" => Ok(serde_json::to_value(self.get_thread(id_arg(arguments)?)?)?),
            "get_runbook" => Ok(serde_json::to_value(self.get_runbook(id_arg(arguments)?)?)?),
            "get_task" => Ok(serde_json::to_value(self.get_task(id_arg(arguments)?)?)?),
            _ => Err(crate::AxiomError::Validation(format!(
                "unknown tool {name}"
            ))),
        }
    }

    fn tool_workspace_requirement(
        &self,
        name: &str,
        arguments: &Value,
    ) -> crate::Result<Option<String>> {
        match name {
            "search_entries" | "search_episodes" | "search_docs" | "search_insights"
            | "search_claims" | "search_procedures" | "find_fix" | "find_decision"
            | "find_runbook" => workspace_filter_arg(arguments),
            "get_session" | "get_thread" => self.session_workspace_id(id_arg(arguments)?),
            "get_artifact" => self.artifact_workspace_id(id_arg(arguments)?),
            "get_anchor" => self.anchor_workspace_id(id_arg(arguments)?),
            "get_case" | "get_runbook" => self.episode_workspace_id(id_arg(arguments)?),
            "get_evidence_bundle" => match subject_kind_arg(arguments)? {
                "episode" => self.episode_workspace_id(subject_id_arg(arguments)?),
                "insight" => self.insight_workspace_id(subject_id_arg(arguments)?),
                "procedure" => self.procedure_workspace_id(subject_id_arg(arguments)?),
                _ => Ok(None),
            },
            "get_task" => self.task_workspace_id(id_arg(arguments)?),
            _ => Ok(None),
        }
    }
}

fn id_arg(arguments: &Value) -> crate::Result<&str> {
    arguments
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| crate::AxiomError::Validation("missing id".to_string()))
}

fn subject_kind_arg(arguments: &Value) -> crate::Result<&str> {
    arguments
        .get("subject_kind")
        .and_then(Value::as_str)
        .ok_or_else(|| crate::AxiomError::Validation("missing subject_kind".to_string()))
}

fn subject_id_arg(arguments: &Value) -> crate::Result<&str> {
    arguments
        .get("subject_id")
        .and_then(Value::as_str)
        .ok_or_else(|| crate::AxiomError::Validation("missing subject_id".to_string()))
}

fn strip_one_of<'a>(uri: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    prefixes.iter().find_map(|prefix| uri.strip_prefix(prefix))
}

fn workspace_filter_arg(arguments: &Value) -> crate::Result<Option<String>> {
    Ok(arguments
        .get("filter")
        .and_then(|filter| filter.get("workspace_root"))
        .and_then(Value::as_str)
        .map(axiomsync_domain::domain::workspace_stable_id))
}
