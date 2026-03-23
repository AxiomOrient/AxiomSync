use super::*;

impl AxiomSync {
    pub fn mcp_resources(&self) -> Result<Value> {
        Ok(serde_json::json!([
            {"uri": "axiom://cases/{id}", "name": "Case"},
            {"uri": "axiom://threads/{id}", "name": "Thread"},
            {"uri": "axiom://runs/{id}", "name": "Run"},
            {"uri": "axiom://documents/{id}", "name": "Document"},
            {"uri": "axiom://evidence/{id}", "name": "Evidence"},
            {"uri": "axiom://episode/{id}", "name": "Episode runbook (legacy)"},
            {"uri": "axiom://thread/{id}", "name": "Conversation thread (legacy)"}
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
            ResourceUri::Case(id) => serde_json::to_value(self.get_case(&id)?).map_err(Into::into),
            ResourceUri::Episode(id) => {
                serde_json::to_value(self.get_runbook(&id)?).map_err(Into::into)
            }
            ResourceUri::Thread(id) => {
                serde_json::to_value(self.get_thread(&id)?).map_err(Into::into)
            }
            ResourceUri::Run(id) => serde_json::to_value(self.get_run(&id)?).map_err(Into::into),
            ResourceUri::Document(id) => {
                serde_json::to_value(self.get_document(&id)?).map_err(Into::into)
            }
            ResourceUri::Evidence(id) => {
                serde_json::to_value(self.get_evidence(&id)?).map_err(Into::into)
            }
        }
    }

    fn resource_workspace_requirement(&self, uri: &str) -> Result<Option<String>> {
        match parse_resource_uri(uri)? {
            ResourceUri::Case(id) => self.case_workspace_id(&id),
            ResourceUri::Episode(id) => self.runbook_workspace_id(&id),
            ResourceUri::Thread(id) => self.thread_workspace_id(&id),
            ResourceUri::Run(id) => self.run_workspace_id(&id),
            ResourceUri::Document(id) => self.document_workspace_id(&id),
            ResourceUri::Evidence(id) => self.evidence_workspace_id(&id),
        }
    }
}

impl McpToolPort for AxiomSync {
    fn mcp_tools(&self) -> Result<Value> {
        Ok(serde_json::json!([
            {"name": "search_cases"},
            {"name": "get_case"},
            {"name": "search_episodes"},
            {"name": "get_runbook"},
            {"name": "get_thread"},
            {"name": "get_evidence"},
            {"name": "search_commands"},
            {"name": "list_runs"},
            {"name": "get_run"},
            {"name": "get_task"},
            {"name": "list_documents"},
            {"name": "get_document"}
        ]))
    }

    fn call_mcp_tool(
        &self,
        name: &str,
        arguments: &Value,
        bound_workspace_id: Option<&str>,
    ) -> Result<Value> {
        match name {
            "search_cases" => serde_json::to_value(
                self.search_cases(crate::domain::SearchCasesRequest {
                    query: required_arg(arguments, "query")?.to_string(),
                    limit: arguments.get("limit").and_then(Value::as_u64).unwrap_or(10) as usize,
                    filter: crate::domain::SearchCasesFilter {
                        producer: arguments
                            .get("filters")
                            .and_then(|filters| {
                                filters
                                    .get("producer")
                                    .or_else(|| filters.get("source"))
                                    .or_else(|| filters.get("connector"))
                            })
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
            "get_case" => serde_json::to_value(
                self.get_case(required_arg_alias(arguments, &["case_id", "episode_id"])?)?,
            )
            .map_err(Into::into),
            "search_episodes" => serde_json::to_value(
                self.search_episodes(crate::domain::SearchEpisodesRequest {
                    query: required_arg(arguments, "query")?.to_string(),
                    limit: arguments.get("limit").and_then(Value::as_u64).unwrap_or(10) as usize,
                    filter: crate::domain::SearchEpisodesFilter {
                        source: arguments
                            .get("filters")
                            .and_then(|filters| {
                                filters.get("source").or_else(|| filters.get("connector"))
                            })
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
            "get_runbook" => serde_json::to_value(
                self.get_runbook(required_arg_alias(arguments, &["episode_id", "case_id"])?)?,
            )
            .map_err(Into::into),
            "get_thread" => serde_json::to_value(
                self.get_thread(required_arg_alias(arguments, &["thread_id", "session_id"])?)?,
            )
            .map_err(Into::into),
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
            "list_runs" => serde_json::to_value(
                self.list_runs(
                    arguments
                        .get("workspace_id")
                        .and_then(Value::as_str)
                        .or(bound_workspace_id),
                )?,
            )
            .map_err(Into::into),
            "get_run" => serde_json::to_value(self.get_run(required_arg(arguments, "run_id")?)?)
                .map_err(Into::into),
            "get_task" => serde_json::to_value(self.get_task(required_arg(arguments, "task_id")?)?)
                .map_err(Into::into),
            "list_documents" => serde_json::to_value(
                self.list_documents(
                    arguments
                        .get("workspace_id")
                        .and_then(Value::as_str)
                        .or(bound_workspace_id),
                    arguments.get("kind").and_then(Value::as_str),
                )?,
            )
            .map_err(Into::into),
            "get_document" => {
                serde_json::to_value(self.get_document(required_arg(arguments, "document_id")?)?)
                    .map_err(Into::into)
            }
            other => Err(AxiomError::Validation(format!("unknown tool {other}"))),
        }
    }

    fn tool_workspace_requirement(&self, name: &str, arguments: &Value) -> Result<Option<String>> {
        match name {
            "get_case" => {
                self.case_workspace_id(required_arg_alias(arguments, &["case_id", "episode_id"])?)
            }
            "get_runbook" => self
                .runbook_workspace_id(required_arg_alias(arguments, &["episode_id", "case_id"])?),
            "get_thread" => self
                .thread_workspace_id(required_arg_alias(arguments, &["thread_id", "session_id"])?),
            "get_evidence" => self.evidence_workspace_id(required_arg(arguments, "evidence_id")?),
            "get_run" => self.run_workspace_id(required_arg(arguments, "run_id")?),
            "get_task" => self.task_workspace_id(required_arg(arguments, "task_id")?),
            "get_document" => self.document_workspace_id(required_arg(arguments, "document_id")?),
            "search_cases" => Ok(arguments
                .get("filters")
                .and_then(|filters| filters.get("workspace_id"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)),
            "search_episodes" => Ok(arguments
                .get("filters")
                .and_then(|filters| filters.get("workspace_id"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)),
            "list_runs" | "list_documents" | "search_commands" => Ok(arguments
                .get("workspace_id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)),
            _ => Ok(None),
        }
    }
}

enum ResourceUri {
    Case(String),
    Episode(String),
    Thread(String),
    Run(String),
    Document(String),
    Evidence(String),
}

fn parse_resource_uri(uri: &str) -> Result<ResourceUri> {
    if let Some(id) = uri.strip_prefix("axiom://cases/") {
        return Ok(ResourceUri::Case(id.to_string()));
    }
    if let Some(id) = uri.strip_prefix("axiom://episode/") {
        return Ok(ResourceUri::Episode(id.to_string()));
    }
    if let Some(id) = uri.strip_prefix("axiom://threads/") {
        return Ok(ResourceUri::Thread(id.to_string()));
    }
    if let Some(id) = uri.strip_prefix("axiom://thread/") {
        return Ok(ResourceUri::Thread(id.to_string()));
    }
    if let Some(id) = uri.strip_prefix("axiom://runs/") {
        return Ok(ResourceUri::Run(id.to_string()));
    }
    if let Some(id) = uri.strip_prefix("axiom://documents/") {
        return Ok(ResourceUri::Document(id.to_string()));
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

fn required_arg_alias<'a>(value: &'a Value, keys: &[&str]) -> Result<&'a str> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .ok_or_else(|| AxiomError::Validation(format!("missing argument {}", keys.join(" or "))))
}
