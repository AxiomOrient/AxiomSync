use serde_json::{Value, json};

use crate::kernel::AxiomSync;
use crate::ports::{McpResourcePort, McpToolPort};

impl McpResourcePort for AxiomSync {
    fn mcp_resources(&self) -> crate::Result<Value> {
        Ok(json!({
            "resources": [
                {"uri": "axiom://cases/{id}", "name": "Case"},
                {"uri": "axiom://threads/{id}", "name": "Thread"},
                {"uri": "axiom://runs/{id}", "name": "Run"},
                {"uri": "axiom://tasks/{id}", "name": "Task"},
                {"uri": "axiom://documents/{id}", "name": "Document"},
                {"uri": "axiom://evidence/{id}", "name": "Evidence"}
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
        if let Some(id) = uri.strip_prefix("axiom://cases/") {
            return Ok(serde_json::to_value(self.get_case(id)?)?);
        }
        if let Some(id) = uri.strip_prefix("axiom://threads/") {
            return Ok(serde_json::to_value(self.get_thread(id)?)?);
        }
        if let Some(id) = uri.strip_prefix("axiom://runs/") {
            return Ok(serde_json::to_value(self.get_run(id)?)?);
        }
        if let Some(id) = uri.strip_prefix("axiom://tasks/") {
            return Ok(serde_json::to_value(self.get_task(id)?)?);
        }
        if let Some(id) = uri.strip_prefix("axiom://documents/") {
            return Ok(serde_json::to_value(self.get_document(id)?)?);
        }
        if let Some(id) = uri.strip_prefix("axiom://evidence/") {
            return Ok(serde_json::to_value(self.get_evidence(id)?)?);
        }
        Err(crate::AxiomError::Validation(format!(
            "unknown resource {uri}"
        )))
    }

    fn resource_workspace_requirement(&self, uri: &str) -> crate::Result<Option<String>> {
        if let Some(id) = uri.strip_prefix("axiom://cases/") {
            return self.case_workspace_id(id);
        }
        if let Some(id) = uri.strip_prefix("axiom://threads/") {
            return self.session_workspace_id(id);
        }
        if let Some(id) = uri.strip_prefix("axiom://runs/") {
            return self.run_workspace_id(id);
        }
        if let Some(id) = uri.strip_prefix("axiom://tasks/") {
            return self.task_workspace_id(id);
        }
        if let Some(id) = uri.strip_prefix("axiom://documents/") {
            return self.document_workspace_id(id);
        }
        if let Some(id) = uri.strip_prefix("axiom://evidence/") {
            return self.evidence_workspace_id(id);
        }
        Ok(None)
    }
}

impl McpToolPort for AxiomSync {
    fn mcp_tools(&self) -> crate::Result<Value> {
        Ok(json!({
            "tools": [
                {"name": "search_cases"},
                {"name": "get_case"},
                {"name": "get_thread"},
                {"name": "get_run"},
                {"name": "get_task"},
                {"name": "get_document"},
                {"name": "get_evidence"},
                {"name": "list_runs"},
                {"name": "list_documents"}
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
            "search_cases" => Ok(serde_json::to_value(
                self.search_cases(serde_json::from_value(arguments.clone())?)?,
            )?),
            "get_case" => Ok(serde_json::to_value(self.get_case(id_arg(arguments)?)?)?),
            "get_thread" => Ok(serde_json::to_value(self.get_thread(id_arg(arguments)?)?)?),
            "get_run" => Ok(serde_json::to_value(self.get_run(id_arg(arguments)?)?)?),
            "get_task" => Ok(serde_json::to_value(self.get_task(id_arg(arguments)?)?)?),
            "get_document" => Ok(serde_json::to_value(
                self.get_document(id_arg(arguments)?)?,
            )?),
            "get_evidence" => Ok(serde_json::to_value(
                self.get_evidence(id_arg(arguments)?)?,
            )?),
            "list_runs" => Ok(serde_json::to_value(
                self.list_runs(arguments.get("workspace_root").and_then(Value::as_str))?,
            )?),
            "list_documents" => Ok(serde_json::to_value(self.list_documents(
                arguments.get("workspace_root").and_then(Value::as_str),
                arguments.get("kind").and_then(Value::as_str),
            )?)?),
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
            "search_cases" => workspace_filter_arg(arguments),
            "get_case" => self.case_workspace_id(id_arg(arguments)?),
            "get_thread" => self.session_workspace_id(id_arg(arguments)?),
            "get_run" => self.run_workspace_id(id_arg(arguments)?),
            "get_task" => self.task_workspace_id(id_arg(arguments)?),
            "get_document" => self.document_workspace_id(id_arg(arguments)?),
            "get_evidence" => self.evidence_workspace_id(id_arg(arguments)?),
            "list_runs" | "list_documents" => Ok(arguments
                .get("workspace_root")
                .and_then(Value::as_str)
                .map(axiomsync_domain::workspace_stable_id)),
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

fn workspace_filter_arg(arguments: &Value) -> crate::Result<Option<String>> {
    Ok(arguments
        .get("filter")
        .and_then(|filter| filter.get("workspace_root"))
        .and_then(Value::as_str)
        .map(axiomsync_domain::workspace_stable_id))
}
