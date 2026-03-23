use std::io::{BufRead, Write};

use serde_json::{Value, json};

use axiomsync_kernel::ports::{McpResourcePort, McpToolPort};

use crate::error::{AxiomError, Result};
use crate::kernel::AxiomSync;

pub async fn serve_stdio(app: AxiomSync, workspace_id: Option<&str>) -> Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut writer = stdout.lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let request: Value = serde_json::from_str(&line)?;
        let response = match handle_request(&app, request, workspace_id) {
            Ok(value) => value,
            Err(error) => json_rpc_error(Value::Null, -32000, error.to_string()),
        };
        writeln!(writer, "{}", serde_json::to_string(&response)?)?;
        writer.flush()?;
    }
    Ok(())
}

pub fn workspace_requirement(app: &AxiomSync, request: &Value) -> Result<Option<String>> {
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let params = request.get("params").cloned().unwrap_or(Value::Null);
    match method {
        "resources/read" => {
            let uri = params
                .get("uri")
                .and_then(Value::as_str)
                .ok_or_else(|| AxiomError::Validation("missing resource uri".to_string()))?;
            app.resource_workspace_requirement(uri)
        }
        "tools/call" => {
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| AxiomError::Validation("missing tool name".to_string()))?;
            let args = params.get("arguments").cloned().unwrap_or(Value::Null);
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
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| AxiomError::Validation("missing method".to_string()))?;
    let params = request.get("params").cloned().unwrap_or(Value::Null);

    let required_workspace = workspace_requirement(app, &request)?;
    if let Some(bound) = bound_workspace_id
        && let Some(required) = required_workspace.as_deref()
        && required != bound
    {
        return Err(AxiomError::PermissionDenied(
            "resource is outside bound workspace".to_string(),
        ));
    }

    let result = match method {
        "initialize" => json!({
            "serverInfo": {
                "name": "axiomsync",
                "version": crate::domain::KERNEL_SCHEMA_VERSION,
            }
        }),
        "roots/list" => app.mcp_roots(bound_workspace_id)?,
        "resources/list" => app.mcp_resources()?,
        "resources/read" => {
            let uri = params
                .get("uri")
                .and_then(Value::as_str)
                .ok_or_else(|| AxiomError::Validation("missing resource uri".to_string()))?;
            app.read_mcp_resource(uri)?
        }
        "tools/list" => app.mcp_tools()?,
        "tools/call" => {
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| AxiomError::Validation("missing tool name".to_string()))?;
            let args = params.get("arguments").cloned().unwrap_or(Value::Null);
            match app.call_mcp_tool(name, &args, bound_workspace_id) {
                Ok(value) => value,
                Err(AxiomError::Validation(message)) if message.starts_with("unknown tool ") => {
                    return Ok(json_rpc_error(id, -32601, message));
                }
                Err(error) => return Err(error),
            }
        }
        other => {
            return Ok(json_rpc_error(
                id,
                -32601,
                format!("unknown method {other}"),
            ));
        }
    };
    Ok(json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    }))
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
