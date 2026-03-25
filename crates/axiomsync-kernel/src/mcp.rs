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
        match parse_resource_uri(uri)? {
            ResourceTarget::Case(id) => Ok(serde_json::to_value(self.get_case(id)?)?),
            ResourceTarget::Thread(id) => Ok(serde_json::to_value(self.get_thread(id)?)?),
            ResourceTarget::Run(id) => Ok(serde_json::to_value(self.get_run(id)?)?),
            ResourceTarget::Task(id) => Ok(serde_json::to_value(self.get_task(id)?)?),
            ResourceTarget::Document(id) => Ok(serde_json::to_value(self.get_document(id)?)?),
            ResourceTarget::Evidence(id) => Ok(serde_json::to_value(self.get_evidence(id)?)?),
        }
    }

    fn resource_workspace_requirement(&self, uri: &str) -> crate::Result<Option<String>> {
        match parse_resource_uri(uri)? {
            ResourceTarget::Case(id) => self.case_workspace_id(id),
            ResourceTarget::Thread(id) => self.session_workspace_id(id),
            ResourceTarget::Run(id) => self.run_workspace_id(id),
            ResourceTarget::Task(id) => self.task_workspace_id(id),
            ResourceTarget::Document(id) => self.document_workspace_id(id),
            ResourceTarget::Evidence(id) => self.evidence_workspace_id(id),
        }
    }
}

impl McpToolPort for AxiomSync {
    fn mcp_tools(&self) -> crate::Result<Value> {
        Ok(json!({
            "tools": [
                {"name": "search_cases", "inputSchema": search_cases_schema()},
                {"name": "get_case", "inputSchema": id_schema()},
                {"name": "get_thread", "inputSchema": id_schema()},
                {"name": "get_run", "inputSchema": id_schema()},
                {"name": "get_task", "inputSchema": id_schema()},
                {"name": "get_document", "inputSchema": id_schema()},
                {"name": "get_evidence", "inputSchema": id_schema()},
                {"name": "list_runs", "inputSchema": workspace_root_schema(false)},
                {"name": "list_documents", "inputSchema": workspace_root_schema(true)}
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
                self.list_runs(Some(required_workspace_root_arg(arguments)?))?,
            )?),
            "list_documents" => Ok(serde_json::to_value(self.list_documents(
                Some(required_workspace_root_arg(arguments)?),
                arguments.get("kind").and_then(Value::as_str),
            )?)?),
            _ => Err(crate::AxiomError::Validation(format!(
                "{}{name}",
                crate::UNKNOWN_TOOL_ERROR_PREFIX
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
            "list_runs" | "list_documents" => Ok(Some(
                axiomsync_domain::workspace_stable_id(required_workspace_root_arg(arguments)?),
            )),
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
    Ok(Some(
        arguments
            .get("filter")
            .and_then(|filter| filter.get("workspace_root"))
            .and_then(Value::as_str)
            .map(axiomsync_domain::workspace_stable_id)
            .ok_or_else(|| {
                crate::AxiomError::Validation("workspace_root filter is required".to_string())
            })?,
    ))
}

fn required_workspace_root_arg(arguments: &Value) -> crate::Result<&str> {
    arguments
        .get("workspace_root")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| crate::AxiomError::Validation("workspace_root is required".to_string()))
}

#[derive(Clone, Copy)]
enum ResourceTarget<'a> {
    Case(&'a str),
    Thread(&'a str),
    Run(&'a str),
    Task(&'a str),
    Document(&'a str),
    Evidence(&'a str),
}

fn parse_resource_uri(uri: &str) -> crate::Result<ResourceTarget<'_>> {
    fn non_empty<'a>(id: &'a str, uri: &str) -> crate::Result<&'a str> {
        if id.is_empty() {
            Err(crate::AxiomError::Validation(format!(
                "invalid resource uri: {uri}"
            )))
        } else {
            Ok(id)
        }
    }
    if let Some(id) = uri.strip_prefix("axiom://cases/") {
        return Ok(ResourceTarget::Case(non_empty(id, uri)?));
    }
    if let Some(id) = uri.strip_prefix("axiom://threads/") {
        return Ok(ResourceTarget::Thread(non_empty(id, uri)?));
    }
    if let Some(id) = uri.strip_prefix("axiom://runs/") {
        return Ok(ResourceTarget::Run(non_empty(id, uri)?));
    }
    if let Some(id) = uri.strip_prefix("axiom://tasks/") {
        return Ok(ResourceTarget::Task(non_empty(id, uri)?));
    }
    if let Some(id) = uri.strip_prefix("axiom://documents/") {
        return Ok(ResourceTarget::Document(non_empty(id, uri)?));
    }
    if let Some(id) = uri.strip_prefix("axiom://evidence/") {
        return Ok(ResourceTarget::Evidence(non_empty(id, uri)?));
    }
    Err(crate::AxiomError::Validation(format!(
        "unknown resource {uri}"
    )))
}

fn id_schema() -> Value {
    json!({
        "type": "object",
        "required": ["id"],
        "properties": {
            "id": { "type": "string", "minLength": 1 }
        }
    })
}

fn search_cases_schema() -> Value {
    json!({
        "type": "object",
        "required": ["query", "filter"],
        "properties": {
            "query": { "type": "string" },
            "limit": { "type": "integer", "minimum": 0 },
            "filter": {
                "type": "object",
                "required": ["workspace_root"],
                "properties": {
                    "workspace_root": { "type": "string", "minLength": 1 }
                }
            }
        }
    })
}

fn workspace_root_schema(include_kind: bool) -> Value {
    let mut properties = serde_json::Map::from_iter([(
        "workspace_root".to_string(),
        json!({ "type": "string", "minLength": 1 }),
    )]);
    if include_kind {
        properties.insert(
            "kind".to_string(),
            json!({ "type": "string", "minLength": 1 }),
        );
    }
    Value::Object(serde_json::Map::from_iter([
        ("type".to_string(), json!("object")),
        ("required".to_string(), json!(["workspace_root"])),
        ("properties".to_string(), Value::Object(properties)),
    ]))
}
