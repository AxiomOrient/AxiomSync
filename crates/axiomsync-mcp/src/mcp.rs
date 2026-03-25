use std::io::{BufRead, Write};

use serde_json::{Value, json};

use axiomsync_domain::error::{AxiomError, Result};
use axiomsync_kernel::AxiomSync;
use axiomsync_kernel::ports::{McpResourcePort, McpToolPort};

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

pub async fn serve_stdio(app: AxiomSync, workspace_id: Option<&str>) -> Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut writer = stdout.lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(request) => {
                let id = rpc_id(&request);
                match parse_request(&request) {
                    Ok(parsed) => match handle_parsed_request(&app, parsed, workspace_id) {
                        Ok(value) => value,
                        Err(error) => error_response(id, &error),
                    },
                    Err(error) => error_response(id, &error),
                }
            }
            Err(error) => json_rpc_error(
                Value::Null,
                JSON_RPC_INVALID_REQUEST,
                format!("invalid request: {error}"),
            ),
        };
        writeln!(writer, "{}", serde_json::to_string(&response)?)?;
        writer.flush()?;
    }
    Ok(())
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
    match request.method.as_str() {
        "resources/read" => {
            let uri = request
                .params
                .get("uri")
                .and_then(Value::as_str)
                .ok_or_else(|| AxiomError::Validation("missing resource uri".to_string()))?;
            app.resource_workspace_requirement(uri)
        }
        "tools/call" => {
            let name = request
                .params
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| AxiomError::Validation("missing tool name".to_string()))?;
            let args = request
                .params
                .get("arguments")
                .cloned()
                .unwrap_or(Value::Null);
            app.tool_workspace_requirement(name, &args)
        }
        _ => Ok(None),
    }
}

pub fn handle_request(
    app: &AxiomSync,
    request: Value,
    bound_workspace_id: Option<&str>,
) -> Result<Value> {
    let id = rpc_id(&request);
    let parsed = match parse_request(&request) {
        Ok(parsed) => parsed,
        Err(error) => return Ok(error_response(id, &error)),
    };
    Ok(
        match handle_parsed_request(app, parsed, bound_workspace_id) {
            Ok(response) => response,
            Err(error) => error_response(id, &error),
        },
    )
}

pub fn handle_parsed_request(
    app: &AxiomSync,
    request: ParsedRequest,
    bound_workspace_id: Option<&str>,
) -> Result<Value> {
    let required_workspace = workspace_requirement(app, &request)?;
    if let Some(bound) = bound_workspace_id
        && let Some(required) = required_workspace.as_deref()
        && required != bound
    {
        return Err(AxiomError::PermissionDenied(
            "resource is outside bound workspace".to_string(),
        ));
    }

    let result = match request.method.as_str() {
        "initialize" => json!({
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
        "roots/list" => app.mcp_roots(bound_workspace_id)?,
        "resources/list" => app.mcp_resources()?,
        "resources/read" => {
            let uri = request
                .params
                .get("uri")
                .and_then(Value::as_str)
                .ok_or_else(|| AxiomError::Validation("missing resource uri".to_string()))?;
            app.read_mcp_resource(uri)?
        }
        "tools/list" => app.mcp_tools()?,
        "tools/call" => {
            let name = request
                .params
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| AxiomError::Validation("missing tool name".to_string()))?;
            let args = request
                .params
                .get("arguments")
                .cloned()
                .unwrap_or(Value::Null);
            match app.call_mcp_tool(name, &args, bound_workspace_id) {
                Ok(value) => value,
                Err(AxiomError::Validation(message)) if message.starts_with("unknown tool ") => {
                    return Ok(json_rpc_error(
                        request.id,
                        JSON_RPC_METHOD_NOT_FOUND,
                        message,
                    ));
                }
                Err(error) => return Err(error),
            }
        }
        other => {
            return Ok(json_rpc_error(
                request.id,
                JSON_RPC_METHOD_NOT_FOUND,
                format!("unknown method {other}"),
            ));
        }
    };
    Ok(json!({
        "jsonrpc": "2.0",
        "id": request.id,
        "result": result,
    }))
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
