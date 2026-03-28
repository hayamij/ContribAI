//! MCP stdio server implementation.
//!
//! Implements the Model Context Protocol (JSON-RPC over stdio)
//! to expose ContribAI tools to Claude Desktop.
//!
//! Tools exposed:
//! - search_repos
//! - get_repo_info
//! - get_file_tree
//! - get_file_content
//! - get_open_issues
//! - fork_repo
//! - create_branch
//! - push_file_change
//! - create_pr
//! - close_pr
//! - get_stats
//! - patrol_prs
//! - check_ai_policy
//! - check_duplicate_pr

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use tracing::{error, info};

use crate::github::client::GitHubClient;
use crate::orchestrator::memory::Memory;

/// JSON-RPC request.
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    #[serde(default)]
    params: Value,
    id: Option<Value>,
}

/// JSON-RPC response.
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
    id: Value,
}

impl JsonRpcResponse {
    fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            result: Some(result),
            error: None,
            id,
        }
    }

    fn error(id: Value, code: i64, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            result: None,
            error: Some(json!({
                "code": code,
                "message": message,
            })),
            id,
        }
    }
}

/// MCP tool definition.
#[derive(Debug, Serialize)]
struct ToolDef {
    name: String,
    description: String,
    #[serde(rename = "inputSchema")]
    input_schema: Value,
}

/// Get all tool definitions.
fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "search_repos".into(),
            description: "Search GitHub for open-source repositories by language and star range".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "language": {"type": "string", "description": "e.g. python, javascript"},
                    "stars_min": {"type": "integer", "default": 50},
                    "stars_max": {"type": "integer", "default": 10000},
                    "limit": {"type": "integer", "default": 10}
                },
                "required": ["language"]
            }),
        },
        ToolDef {
            name: "get_repo_info".into(),
            description: "Get metadata for a GitHub repository".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "owner": {"type": "string"},
                    "repo": {"type": "string"}
                },
                "required": ["owner", "repo"]
            }),
        },
        ToolDef {
            name: "get_file_tree".into(),
            description: "List files in a repository (recursive)".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "owner": {"type": "string"},
                    "repo": {"type": "string"},
                    "max_files": {"type": "integer", "default": 200}
                },
                "required": ["owner", "repo"]
            }),
        },
        ToolDef {
            name: "get_file_content".into(),
            description: "Get the content of a specific file from a repository".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "owner": {"type": "string"},
                    "repo": {"type": "string"},
                    "path": {"type": "string"},
                    "ref": {"type": "string", "description": "Branch or commit SHA (optional)"}
                },
                "required": ["owner", "repo", "path"]
            }),
        },
        ToolDef {
            name: "get_open_issues".into(),
            description: "List open issues in a repository".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "owner": {"type": "string"},
                    "repo": {"type": "string"},
                    "limit": {"type": "integer", "default": 20}
                },
                "required": ["owner", "repo"]
            }),
        },
        ToolDef {
            name: "fork_repo".into(),
            description: "Fork a repository to the authenticated user's account".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "owner": {"type": "string"},
                    "repo": {"type": "string"}
                },
                "required": ["owner", "repo"]
            }),
        },
        ToolDef {
            name: "create_branch".into(),
            description: "Create a new branch on a repository".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "fork_owner": {"type": "string"},
                    "repo": {"type": "string"},
                    "branch_name": {"type": "string"},
                    "from_branch": {"type": "string", "description": "Source branch (defaults to repo default)"}
                },
                "required": ["fork_owner", "repo", "branch_name"]
            }),
        },
        ToolDef {
            name: "push_file_change".into(),
            description: "Push a file change to a branch. For updates, sha (blob SHA) is required.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "fork_owner": {"type": "string"},
                    "repo": {"type": "string"},
                    "branch": {"type": "string"},
                    "path": {"type": "string"},
                    "content": {"type": "string"},
                    "commit_msg": {"type": "string"},
                    "sha": {"type": "string", "description": "Blob SHA of existing file (required for updates)"}
                },
                "required": ["fork_owner", "repo", "branch", "path", "content", "commit_msg"]
            }),
        },
        ToolDef {
            name: "create_pr".into(),
            description: "Create a pull request from a fork branch to the upstream repo".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "owner": {"type": "string"},
                    "repo": {"type": "string"},
                    "title": {"type": "string"},
                    "body": {"type": "string"},
                    "head_branch": {"type": "string", "description": "fork_owner:branch"},
                    "base_branch": {"type": "string", "description": "Target branch (defaults to default branch)"}
                },
                "required": ["owner", "repo", "title", "body", "head_branch"]
            }),
        },
        ToolDef {
            name: "get_stats".into(),
            description: "Get ContribAI contribution statistics".into(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
    ]
}

/// Run the MCP server on stdio (JSON-RPC over stdin/stdout).
pub async fn run_stdio_server(
    github: &GitHubClient,
    memory: &Memory,
) -> anyhow::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();

    info!("MCP server started on stdio");

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) if l.is_empty() => continue,
            Ok(l) => l,
            Err(_) => break,
        };

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                error!(error = %e, "Invalid JSON-RPC request");
                continue;
            }
        };

        let id = request.id.clone().unwrap_or(Value::Null);

        let response = match request.method.as_str() {
            "initialize" => {
                JsonRpcResponse::success(
                    id,
                    json!({
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {}
                        },
                        "serverInfo": {
                            "name": "contribai",
                            "version": crate::VERSION
                        }
                    }),
                )
            }

            "tools/list" => {
                let tools = tool_definitions();
                JsonRpcResponse::success(id, json!({ "tools": tools }))
            }

            "tools/call" => {
                let tool_name = request.params.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let arguments = request.params.get("arguments")
                    .cloned()
                    .unwrap_or(json!({}));

                match handle_tool_call(tool_name, &arguments, github, memory).await {
                    Ok(result) => JsonRpcResponse::success(
                        id,
                        json!({
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string(&result).unwrap_or_default()
                            }]
                        }),
                    ),
                    Err(e) => JsonRpcResponse::success(
                        id,
                        json!({
                            "content": [{
                                "type": "text",
                                "text": json!({"error": e.to_string()}).to_string()
                            }],
                            "isError": true
                        }),
                    ),
                }
            }

            "notifications/initialized" | "ping" => {
                // No response needed for notifications
                continue;
            }

            _ => {
                JsonRpcResponse::error(id, -32601, &format!("Method not found: {}", request.method))
            }
        };

        let response_json = serde_json::to_string(&response)?;
        let mut out = stdout.lock();
        writeln!(out, "{}", response_json)?;
        out.flush()?;
    }

    info!("MCP server stopped");
    Ok(())
}

/// Handle a tool call by name.
async fn handle_tool_call(
    name: &str,
    args: &Value,
    github: &GitHubClient,
    memory: &Memory,
) -> anyhow::Result<Value> {
    match name {
        "search_repos" => {
            let language = args["language"].as_str().unwrap_or("python");
            let stars_min = args["stars_min"].as_i64().unwrap_or(50);
            let stars_max = args["stars_max"].as_i64().unwrap_or(10000);
            let limit = args["limit"].as_u64().unwrap_or(10) as usize;

            let query = format!(
                "language:{} stars:{}..{} sort:updated",
                language, stars_min, stars_max
            );

            let repos = github.search_repositories(&query, "updated", limit as u32).await?;
            Ok(json!(repos))
        }

        "get_repo_info" => {
            let owner = args["owner"].as_str().unwrap_or("");
            let repo = args["repo"].as_str().unwrap_or("");
            let info = github.get_repo_details(owner, repo).await?;
            Ok(json!(info))
        }

        "get_file_tree" => {
            let owner = args["owner"].as_str().unwrap_or("");
            let repo = args["repo"].as_str().unwrap_or("");
            let tree = github.get_file_tree(owner, repo, None).await?;
            Ok(json!(tree))
        }

        "get_file_content" => {
            let owner = args["owner"].as_str().unwrap_or("");
            let repo = args["repo"].as_str().unwrap_or("");
            let path = args["path"].as_str().unwrap_or("");
            let git_ref = args["ref"].as_str();
            let content = github.get_file_content(owner, repo, path, git_ref).await?;
            Ok(json!({"path": path, "content": content}))
        }

        "get_open_issues" => {
            let owner = args["owner"].as_str().unwrap_or("");
            let repo = args["repo"].as_str().unwrap_or("");
            let issues = github.get_open_issues(owner, repo, 20).await?;
            Ok(json!(issues))
        }

        "fork_repo" => {
            let owner = args["owner"].as_str().unwrap_or("");
            let repo = args["repo"].as_str().unwrap_or("");
            let fork = github.fork_repository(owner, repo).await?;
            Ok(json!(fork))
        }

        "create_branch" => {
            let fork_owner = args["fork_owner"].as_str().unwrap_or("");
            let repo = args["repo"].as_str().unwrap_or("");
            let branch_name = args["branch_name"].as_str().unwrap_or("");
            let from_branch = args["from_branch"].as_str();
            github
                .create_branch(fork_owner, repo, branch_name, from_branch)
                .await?;
            Ok(json!({"status": "created", "branch": branch_name}))
        }

        "push_file_change" => {
            let fork_owner = args["fork_owner"].as_str().unwrap_or("");
            let repo = args["repo"].as_str().unwrap_or("");
            let branch = args["branch"].as_str().unwrap_or("");
            let path = args["path"].as_str().unwrap_or("");
            let content = args["content"].as_str().unwrap_or("");
            let commit_msg = args["commit_msg"].as_str().unwrap_or("");
            let sha = args["sha"].as_str();

            github
                .create_or_update_file(
                    fork_owner, repo, path, content, commit_msg, branch, sha, None,
                )
                .await?;
            Ok(json!({"status": "pushed", "path": path}))
        }

        "create_pr" => {
            let owner = args["owner"].as_str().unwrap_or("");
            let repo = args["repo"].as_str().unwrap_or("");
            let title = args["title"].as_str().unwrap_or("");
            let body = args["body"].as_str().unwrap_or("");
            let head = args["head_branch"].as_str().unwrap_or("");
            let base = args["base_branch"].as_str();

            let pr = github
                .create_pull_request(owner, repo, title, body, head, base)
                .await?;
            Ok(json!(pr))
        }

        "get_stats" => {
            let stats = memory.get_stats()?;
            Ok(json!(stats))
        }

        _ => {
            anyhow::bail!("Unknown tool: {}", name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_definitions_complete() {
        let tools = tool_definitions();
        assert!(tools.len() >= 10);

        let names: Vec<_> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"search_repos"));
        assert!(names.contains(&"get_file_content"));
        assert!(names.contains(&"create_pr"));
        assert!(names.contains(&"get_stats"));
    }

    #[test]
    fn test_jsonrpc_response_success() {
        let resp = JsonRpcResponse::success(json!(1), json!({"ok": true}));
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"ok\":true"));
        assert!(!json.contains("error"));
    }

    #[test]
    fn test_jsonrpc_response_error() {
        let resp = JsonRpcResponse::error(json!(2), -32601, "Method not found");
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("-32601"));
        assert!(json.contains("Method not found"));
    }
}
