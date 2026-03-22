use super::*;

impl AxiomSync {
    pub fn mcp_resources(&self) -> Result<Value> {
        Ok(serde_json::json!([
            {"uri": "axiom://episode/{id}", "name": "Episode runbook"},
            {"uri": "axiom://thread/{id}", "name": "Conversation thread"},
            {"uri": "axiom://evidence/{id}", "name": "Evidence anchor"}
        ]))
    }

    pub fn mcp_roots(&self, bound_workspace_id: Option<&str>) -> Result<Value> {
        let mut roots = self
            .repo
            .load_workspaces()?
            .into_iter()
            .filter(|workspace| bound_workspace_id.is_none_or(|bound| workspace.stable_id == bound))
            .map(|workspace| {
                serde_json::json!({
                    "uri": workspace.canonical_root,
                    "name": workspace.canonical_root,
                    "workspace_id": workspace.stable_id,
                })
            })
            .collect::<Vec<_>>();
        roots.sort_by(|left, right| {
            left.get("workspace_id")
                .and_then(Value::as_str)
                .cmp(&right.get("workspace_id").and_then(Value::as_str))
        });
        Ok(Value::Array(roots))
    }
}

impl McpResourcePort for AxiomSync {
    fn mcp_resources(&self) -> Result<Value> {
        AxiomSync::mcp_resources(self)
    }

    fn mcp_roots(&self, bound_workspace_id: Option<&str>) -> Result<Value> {
        AxiomSync::mcp_roots(self, bound_workspace_id)
    }

    fn read_mcp_resource(&self, uri: &str) -> Result<Value> {
        match parse_resource_uri(uri)? {
            ResourceUri::Episode(id) => {
                serde_json::to_value(self.get_runbook(&id)?).map_err(Into::into)
            }
            ResourceUri::Thread(id) => {
                serde_json::to_value(self.get_thread(&id)?).map_err(Into::into)
            }
            ResourceUri::Evidence(id) => {
                serde_json::to_value(self.get_evidence(&id)?).map_err(Into::into)
            }
        }
    }

    fn resource_workspace_requirement(&self, uri: &str) -> Result<Option<String>> {
        match parse_resource_uri(uri)? {
            ResourceUri::Episode(id) => self.runbook_workspace_id(&id),
            ResourceUri::Thread(id) => self.thread_workspace_id(&id),
            ResourceUri::Evidence(id) => self.evidence_workspace_id(&id),
        }
    }
}

impl McpToolPort for AxiomSync {
    fn mcp_tools(&self) -> Result<Value> {
        Ok(serde_json::json!([
            {"name": "search_episodes"},
            {"name": "get_runbook"},
            {"name": "get_thread"},
            {"name": "get_evidence"},
            {"name": "search_commands"}
        ]))
    }

    fn call_mcp_tool(
        &self,
        name: &str,
        arguments: &Value,
        bound_workspace_id: Option<&str>,
    ) -> Result<Value> {
        match name {
            "search_episodes" => serde_json::to_value(
                self.search_episodes(crate::domain::SearchEpisodesRequest {
                    query: required_arg(arguments, "query")?.to_string(),
                    limit: arguments.get("limit").and_then(Value::as_u64).unwrap_or(10) as usize,
                    filter: crate::domain::SearchEpisodesFilter {
                        connector: arguments
                            .get("filters")
                            .and_then(|filters| filters.get("connector"))
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        workspace_id: self
                            .tool_workspace_requirement(name, arguments)?
                            .or_else(|| bound_workspace_id.map(ToOwned::to_owned)),
                        status: arguments
                            .get("filters")
                            .and_then(|filters| filters.get("status"))
                            .and_then(Value::as_str)
                            .map(crate::domain::EpisodeStatus::parse)
                            .transpose()?,
                    },
                })?,
            )
            .map_err(Into::into),
            "get_runbook" => {
                serde_json::to_value(self.get_runbook(required_arg(arguments, "episode_id")?)?)
                    .map_err(Into::into)
            }
            "get_thread" => {
                serde_json::to_value(self.get_thread(required_arg(arguments, "thread_id")?)?)
                    .map_err(Into::into)
            }
            "get_evidence" => {
                serde_json::to_value(self.get_evidence(required_arg(arguments, "evidence_id")?)?)
                    .map_err(Into::into)
            }
            "search_commands" => {
                let query = required_arg(arguments, "query")?;
                let limit = arguments.get("limit").and_then(Value::as_u64).unwrap_or(5) as usize;
                let workspace_id = arguments
                    .get("workspace_id")
                    .and_then(Value::as_str)
                    .or(bound_workspace_id);
                let result = if let Some(ws) = workspace_id {
                    self.search_commands_in_workspace(query, limit, ws)?
                } else {
                    self.search_commands(query, limit)?
                };
                serde_json::to_value(result).map_err(Into::into)
            }
            other => Err(AxiomError::Validation(format!("unknown tool {other}"))),
        }
    }

    fn tool_workspace_requirement(&self, name: &str, arguments: &Value) -> Result<Option<String>> {
        match name {
            "get_runbook" => self.runbook_workspace_id(required_arg(arguments, "episode_id")?),
            "get_thread" => self.thread_workspace_id(required_arg(arguments, "thread_id")?),
            "get_evidence" => self.evidence_workspace_id(required_arg(arguments, "evidence_id")?),
            "search_episodes" => Ok(arguments
                .get("filters")
                .and_then(|filters| filters.get("workspace_id"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)),
            "search_commands" => Ok(arguments
                .get("workspace_id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)),
            _ => Ok(None),
        }
    }
}

enum ResourceUri {
    Episode(String),
    Thread(String),
    Evidence(String),
}

fn parse_resource_uri(uri: &str) -> Result<ResourceUri> {
    if let Some(id) = uri.strip_prefix("axiom://episode/") {
        return Ok(ResourceUri::Episode(id.to_string()));
    }
    if let Some(id) = uri.strip_prefix("axiom://thread/") {
        return Ok(ResourceUri::Thread(id.to_string()));
    }
    if let Some(id) = uri.strip_prefix("axiom://evidence/") {
        return Ok(ResourceUri::Evidence(id.to_string()));
    }
    Err(AxiomError::Validation(format!(
        "invalid resource uri {uri}"
    )))
}

fn required_arg<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| AxiomError::Validation(format!("missing argument {key}")))
}
