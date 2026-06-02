use crate::error::ToolError;
use crate::platform::is_arch_linux;
use crate::protocol::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::ToolHandler;

const PACMAN_LOG: &str = "/var/log/pacman.log";

pub struct QueryPackageHistory;

#[async_trait]
impl ToolHandler for QueryPackageHistory {
    fn info(&self) -> Tool {
        Tool {
            name: "query_package_history".into(),
            description: "Query pacman transaction history from /var/log/pacman.log. Supports 4 query types: all (recent transactions), package (filter by name), failures (failed operations), sync (database sync history).".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query_type": {
                        "type": "string",
                        "enum": ["all", "package", "failures", "sync"],
                        "description": "Type of history query"
                    },
                    "package_name": {
                        "type": "string",
                        "description": "Package name filter (required for query_type=package)"
                    },
                    "limit": {
                        "type": "integer",
                        "default": 50,
                        "description": "Max lines to return"
                    }
                },
                "required": ["query_type"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        if !is_arch_linux() {
            return Err(ToolError::platform_not_arch());
        }

        let query_type = args
            .get("query_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_argument("Missing: query_type"))?;

        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as usize;

        let content = std::fs::read_to_string(PACMAN_LOG)
            .map_err(|e| ToolError::parse_error(&format!("Cannot read {}: {}", PACMAN_LOG, e)))?;

        match query_type {
            "all" => Ok(query_all(&content, limit)),
            "package" => {
                let pkg = args
                    .get("package_name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::invalid_argument("Missing: package_name for query_type=package"))?;
                Ok(query_package(&content, pkg, limit))
            }
            "failures" => Ok(query_failures(&content, limit)),
            "sync" => Ok(query_sync(&content, limit)),
            _ => Err(ToolError::invalid_argument("Unknown query_type")),
        }
    }
}

fn parse_log_line(line: &str) -> Option<Value> {
    let re = regex::Regex::new(
        r"^\[(.+?)\]\s+\[(\w+(?:-SCRIPTLET)?)\]\s+(.+)$"
    ).ok()?;

    let caps = re.captures(line)?;
    Some(serde_json::json!({
        "timestamp": caps.get(1)?.as_str(),
        "action_type": caps.get(2)?.as_str(),
        "message": caps.get(3)?.as_str()
    }))
}

fn query_all(content: &str, limit: usize) -> Value {
    let entries: Vec<Value> = content
        .lines()
        .filter_map(parse_log_line)
        .rev()
        .take(limit)
        .collect();

    serde_json::json!({
        "query": "all",
        "count": entries.len(),
        "entries": entries
    })
}

fn query_package(content: &str, package: &str, limit: usize) -> Value {
    let pattern = format!(
        r"(installed|upgraded|removed|downgraded)\s+{}\s",
        regex::escape(package)
    );
    let re = regex::Regex::new(&pattern).ok();

    let entries: Vec<Value> = content
        .lines()
        .filter(|line| {
            if let Some(ref regex) = re {
                regex.is_match(line)
            } else {
                line.contains(package)
            }
        })
        .filter_map(parse_log_line)
        .rev()
        .take(limit)
        .collect();

    serde_json::json!({
        "query": "package",
        "package": package,
        "count": entries.len(),
        "entries": entries
    })
}

fn query_failures(content: &str, limit: usize) -> Value {
    let entries: Vec<Value> = content
        .lines()
        .filter(|l| {
            l.contains("error")
                || l.contains("failed")
                || l.contains("FAILED")
                || l.contains("ERROR")
        })
        .filter_map(parse_log_line)
        .rev()
        .take(limit)
        .collect();

    serde_json::json!({
        "query": "failures",
        "count": entries.len(),
        "entries": entries
    })
}

fn query_sync(content: &str, limit: usize) -> Value {
    let entries: Vec<Value> = content
        .lines()
        .filter(|l| {
            l.contains("synchronizing package lists")
                || l.contains("Running 'pacman -Sy")
                || l.contains("Running 'pacman -Syu")
                || l.contains("starting full system upgrade")
        })
        .filter_map(parse_log_line)
        .rev()
        .take(limit)
        .collect();

    serde_json::json!({
        "query": "sync",
        "count": entries.len(),
        "entries": entries
    })
}
