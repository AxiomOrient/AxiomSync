use std::io::{BufRead, Write};

use serde_json::{Value, json};

use axiomsync_domain::error::{AxiomError, Result};
use axiomsync_kernel::ports::{McpResourcePort, McpToolPort};
use axiomsync_kernel::{AxiomSync, UNKNOWN_TOOL_ERROR_PREFIX};

const MCP_PROTOCOL_VERSION: &str = "2025-06-18";
const JSON_RPC_INVALID_REQUEST: i64 = -32600;
const JSON_RPC_METHOD_NOT_FOUND: i64 = -32601;
const JSON_RPC_INVALID_PARAMS: i64 = -32602;
const JSON_RPC_INTERNAL_ERROR: i64 = -32000;

#[derive(Debug, Clone)]
pub struct ParsedRequest {
    pub id: Value,
    pub method: String,
    pub params: Value,
}

#[derive(Debug, Clone, PartialEq)]
enum NormalizedMcpRequest {
    Initialize { id: Value },
    RootsList { id: Value },
    ResourcesList { id: Value },
    ResourceRead { id: Value, uri: String },
    ToolsList { id: Value },
    ToolCall { id: Value, name: String, arguments: Value },
    UnknownMethod { id: Value, method: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParsedTransportLine {
    Json(Value),
    InvalidJson(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NormalizedTransportLine {
    InvalidJson(String),
    Notification,
    Request { id: Value, request: Value },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum StdioPlan {
    Ignore,
    Respond(Value),
    Execute { id: Value, request: Value },
}

#[derive(Debug, Clone, PartialEq)]
enum RequestPlan {
    Respond(Value),
    Initialize { id: Value },
    RootsList { id: Value },
    ResourcesList { id: Value },
    ResourceRead { id: Value, uri: String },
    ToolsList { id: Value },
    ToolCall { id: Value, name: String, arguments: Value },
}

pub async fn serve_stdio(app: AxiomSync, workspace_id: Option<&str>) -> Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut writer = stdout.lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let parsed = parse_stdio_line(&line);
        let normalized = normalize_stdio_line(parsed);
        let plan = plan_stdio_line(normalized);
        if let Some(response) = apply_stdio_plan(&app, plan, workspace_id)? {
            writeln!(writer, "{}", serde_json::to_string(&response)?)?;
            writer.flush()?;
        }
    }
    Ok(())
}

fn parse_stdio_line(line: &str) -> ParsedTransportLine {
    match serde_json::from_str::<Value>(line) {
        Ok(request) => ParsedTransportLine::Json(request),
        Err(error) => ParsedTransportLine::InvalidJson(error.to_string()),
    }
}

fn normalize_stdio_line(parsed: ParsedTransportLine) -> NormalizedTransportLine {
    match parsed {
        ParsedTransportLine::InvalidJson(error) => NormalizedTransportLine::InvalidJson(error),
        ParsedTransportLine::Json(request) => normalize_transport_request(request),
    }
}

fn plan_stdio_line(normalized: NormalizedTransportLine) -> StdioPlan {
    match normalized {
        NormalizedTransportLine::InvalidJson(error) => StdioPlan::Respond(json_rpc_error(
            Value::Null,
            JSON_RPC_INVALID_REQUEST,
            format!("invalid request: {error}"),
        )),
        NormalizedTransportLine::Notification => StdioPlan::Ignore,
        NormalizedTransportLine::Request { id, request } => StdioPlan::Execute { id, request },
    }
}

fn normalize_transport_request(request: Value) -> NormalizedTransportLine {
    if request.as_object().is_some_and(|object| !object.contains_key("id")) {
        return NormalizedTransportLine::Notification;
    }
    let id = rpc_id(&request);
    NormalizedTransportLine::Request { id, request }
}

fn apply_stdio_plan(
    app: &AxiomSync,
    plan: StdioPlan,
    workspace_id: Option<&str>,
) -> Result<Option<Value>> {
    match plan {
        StdioPlan::Ignore => Ok(None),
        StdioPlan::Respond(response) => Ok(Some(response)),
        StdioPlan::Execute { id, request } => {
            let response = match parse_request(&request) {
                Ok(parsed) => match handle_parsed_request(app, parsed, workspace_id) {
                    Ok(value) => value,
                    Err(error) => error_response(id, &error),
                },
                Err(error) => request_parse_error_response(id, &error),
            };
            Ok(Some(response))
        }
    }
}

pub fn parse_request(request: &Value) -> Result<ParsedRequest> {
    let object = request
        .as_object()
        .ok_or_else(|| AxiomError::Validation("request must be a JSON object".to_string()))?;
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Err(AxiomError::Validation(
            "jsonrpc must be \"2.0\"".to_string(),
        ));
    }
    let method = object
        .get("method")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AxiomError::Validation("missing method".to_string()))?;
    Ok(ParsedRequest {
        id: rpc_id(request),
        method: method.to_string(),
        params: object.get("params").cloned().unwrap_or(Value::Null),
    })
}

pub fn workspace_requirement(app: &AxiomSync, request: &ParsedRequest) -> Result<Option<String>> {
    let normalized = normalize_request(request.clone())?;
    workspace_requirement_for(app, &normalized)
}

pub fn handle_request(
    app: &AxiomSync,
    request: Value,
    bound_workspace_id: Option<&str>,
) -> Result<Value> {
    let id = rpc_id(&request);
    let parsed = match parse_request(&request) {
        Ok(parsed) => parsed,
        Err(error) => return Ok(request_parse_error_response(id, &error)),
    };
    let normalized = match normalize_request(parsed) {
        Ok(normalized) => normalized,
        Err(error) => return Ok(error_response(id, &error)),
    };
    let plan = plan_request(normalized);
    apply_request_plan(app, plan, bound_workspace_id)
}

pub fn handle_parsed_request(
    app: &AxiomSync,
    request: ParsedRequest,
    bound_workspace_id: Option<&str>,
) -> Result<Value> {
    let id = request.id.clone();
    let normalized = match normalize_request(request) {
        Ok(normalized) => normalized,
        Err(error) => return Ok(error_response(id, &error)),
    };
    let plan = plan_request(normalized);
    apply_request_plan(app, plan, bound_workspace_id)
}

fn normalize_request(request: ParsedRequest) -> Result<NormalizedMcpRequest> {
    match request.method.as_str() {
        "initialize" => Ok(NormalizedMcpRequest::Initialize { id: request.id }),
        "roots/list" => Ok(NormalizedMcpRequest::RootsList { id: request.id }),
        "resources/list" => Ok(NormalizedMcpRequest::ResourcesList { id: request.id }),
        "resources/read" => Ok(NormalizedMcpRequest::ResourceRead {
            id: request.id,
            uri: required_string_arg(&request.params, "uri", "missing resource uri")?.to_string(),
        }),
        "tools/list" => Ok(NormalizedMcpRequest::ToolsList { id: request.id }),
        "tools/call" => Ok(NormalizedMcpRequest::ToolCall {
            id: request.id,
            name: required_string_arg(&request.params, "name", "missing tool name")?.to_string(),
            arguments: request
                .params
                .get("arguments")
                .cloned()
                .unwrap_or(Value::Null),
        }),
        other => Ok(NormalizedMcpRequest::UnknownMethod {
            id: request.id,
            method: other.to_string(),
        }),
    }
}

fn plan_request(request: NormalizedMcpRequest) -> RequestPlan {
    match request {
        NormalizedMcpRequest::Initialize { id } => RequestPlan::Initialize { id },
        NormalizedMcpRequest::RootsList { id } => RequestPlan::RootsList { id },
        NormalizedMcpRequest::ResourcesList { id } => RequestPlan::ResourcesList { id },
        NormalizedMcpRequest::ResourceRead { id, uri } => RequestPlan::ResourceRead { id, uri },
        NormalizedMcpRequest::ToolsList { id } => RequestPlan::ToolsList { id },
        NormalizedMcpRequest::ToolCall {
            id,
            name,
            arguments,
        } => RequestPlan::ToolCall {
            id,
            name,
            arguments,
        },
        NormalizedMcpRequest::UnknownMethod { id, method } => RequestPlan::Respond(json_rpc_error(
            id,
            JSON_RPC_METHOD_NOT_FOUND,
            format!("unknown method {method}"),
        )),
    }
}

fn workspace_requirement_for(
    app: &AxiomSync,
    request: &NormalizedMcpRequest,
) -> Result<Option<String>> {
    match request {
        NormalizedMcpRequest::ResourceRead { uri, .. } => app.resource_workspace_requirement(uri),
        NormalizedMcpRequest::ToolCall {
            name, arguments, ..
        } => app.tool_workspace_requirement(name, arguments),
        _ => Ok(None),
    }
}

fn apply_request_plan(
    app: &AxiomSync,
    plan: RequestPlan,
    bound_workspace_id: Option<&str>,
) -> Result<Value> {
    let plan_id = plan_id(&plan);
    let required_workspace = match workspace_requirement_for_plan(app, &plan) {
        Ok(required_workspace) => required_workspace,
        Err(error) => return Ok(error_response(plan_id, &error)),
    };
    if let Some(bound) = bound_workspace_id
        && let Some(required) = required_workspace.as_deref()
        && required != bound
    {
        return Err(AxiomError::PermissionDenied(
            "resource is outside bound workspace".to_string(),
        ));
    }

    match plan {
        RequestPlan::Respond(response) => Ok(response),
        RequestPlan::Initialize { id } => Ok(success_response(
            id,
            json!({
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {
                    "resources": { "listChanged": false },
                    "tools": { "listChanged": false },
                    "roots": { "listChanged": false }
                },
                "serverInfo": {
                    "name": "axiomsync",
                    "version": axiomsync_domain::KERNEL_SCHEMA_VERSION,
                }
            }),
        )),
        RequestPlan::RootsList { id } => {
            Ok(success_response(id, app.mcp_roots(bound_workspace_id)?))
        }
        RequestPlan::ResourcesList { id } => Ok(success_response(id, app.mcp_resources()?)),
        RequestPlan::ResourceRead { id, uri } => {
            Ok(success_response(id, app.read_mcp_resource(&uri)?))
        }
        RequestPlan::ToolsList { id } => Ok(success_response(id, app.mcp_tools()?)),
        RequestPlan::ToolCall {
            id,
            name,
            arguments,
        } => {
            let result = match app.call_mcp_tool(&name, &arguments, bound_workspace_id) {
                Ok(value) => value,
                Err(AxiomError::Validation(message))
                    if message.starts_with(UNKNOWN_TOOL_ERROR_PREFIX) =>
                {
                    return Ok(json_rpc_error(id, JSON_RPC_METHOD_NOT_FOUND, message));
                }
                Err(error) => return Err(error),
            };
            Ok(success_response(id, result))
        }
    }
}

fn plan_id(plan: &RequestPlan) -> Value {
    match plan {
        RequestPlan::Respond(response) => response.get("id").cloned().unwrap_or(Value::Null),
        RequestPlan::Initialize { id }
        | RequestPlan::RootsList { id }
        | RequestPlan::ResourcesList { id }
        | RequestPlan::ResourceRead { id, .. }
        | RequestPlan::ToolsList { id }
        | RequestPlan::ToolCall { id, .. } => id.clone(),
    }
}

fn workspace_requirement_for_plan(app: &AxiomSync, plan: &RequestPlan) -> Result<Option<String>> {
    match plan {
        RequestPlan::Respond(_)
        | RequestPlan::Initialize { .. }
        | RequestPlan::RootsList { .. }
        | RequestPlan::ResourcesList { .. }
        | RequestPlan::ToolsList { .. } => Ok(None),
        RequestPlan::ResourceRead { uri, .. } => app.resource_workspace_requirement(uri),
        RequestPlan::ToolCall {
            name,
            arguments,
            ..
        } => app.tool_workspace_requirement(name, arguments),
    }
}

fn required_string_arg<'a>(params: &'a Value, key: &str, message: &str) -> Result<&'a str> {
    params
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| AxiomError::Validation(message.to_string()))
}

fn success_response(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

pub fn rpc_id(request: &Value) -> Value {
    request.get("id").cloned().unwrap_or(Value::Null)
}

pub fn error_response(id: Value, error: &AxiomError) -> Value {
    let code = match error {
        AxiomError::Validation(_) => JSON_RPC_INVALID_PARAMS,
        _ => JSON_RPC_INTERNAL_ERROR,
    };
    json_rpc_error(id, code, error.to_string())
}

/// Error response for structural JSON-RPC request failures (not a valid Request object).
/// Uses -32600 (Invalid Request) per JSON-RPC 2.0 §5.1.
pub fn request_parse_error_response(id: Value, error: &AxiomError) -> Value {
    json_rpc_error(id, JSON_RPC_INVALID_REQUEST, error.to_string())
}

fn json_rpc_error(id: Value, code: i64, message: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_and_plan_tool_call_without_apply() {
        let parsed = ParsedRequest {
            id: json!(7),
            method: "tools/call".to_string(),
            params: json!({
                "name": "list_runs",
                "arguments": {
                    "workspace_root": "/workspace/http"
                }
            }),
        };

        let normalized = normalize_request(parsed).expect("normalized request");
        assert_eq!(
            normalized,
            NormalizedMcpRequest::ToolCall {
                id: json!(7),
                name: "list_runs".to_string(),
                arguments: json!({ "workspace_root": "/workspace/http" }),
            }
        );

        let plan = plan_request(normalized);
        assert_eq!(
            plan,
            RequestPlan::ToolCall {
                id: json!(7),
                name: "list_runs".to_string(),
                arguments: json!({ "workspace_root": "/workspace/http" }),
            }
        );
    }

    #[test]
    fn stdio_notification_returns_no_response_only_for_object_requests() {
        let app = axiomsync_cli::open(tempfile::tempdir().expect("tempdir").path()).expect("app");

        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "initialize"
        });
        let plan = plan_stdio_line(normalize_stdio_line(parse_stdio_line(
            &notification.to_string(),
        )));
        assert_eq!(plan, StdioPlan::Ignore);

        let batch = serde_json::json!([{
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize"
        }]);
        let plan = plan_stdio_line(normalize_stdio_line(parse_stdio_line(&batch.to_string())));
        match plan {
            StdioPlan::Execute { .. } => {}
            other => panic!("expected execute plan, got {other:?}"),
        }
        let response = apply_stdio_plan(&app, plan, None)
            .expect("batch apply")
            .expect("response");
        assert_eq!(response["error"]["code"].as_i64(), Some(JSON_RPC_INVALID_REQUEST));
    }
}
