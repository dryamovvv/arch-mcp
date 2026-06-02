use crate::error::ToolError;
use crate::platform::is_arch_linux;
use crate::protocol::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::ToolHandler;

// --- Query File Ownership ---

pub struct QueryFileOwnership;

#[async_trait]
impl ToolHandler for QueryFileOwnership {
    fn info(&self) -> Tool {
        Tool {
            name: "query_file_ownership".into(),
            description: "Query which package owns a file, list files in a package, or search by filename".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "File path or package name or filename"
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["file_to_package", "package_to_files", "filename_search"],
                        "default": "file_to_package"
                    }
                },
                "required": ["query", "mode"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        if !is_arch_linux() {
            return Err(ToolError::platform_not_arch());
        }
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_argument("Missing: query"))?;
        let mode = args
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("file_to_package");

        match mode {
            "file_to_package" => {
                let result = crate::command::run("pacman", &["-Qo", query]).await?;
                Ok(serde_json::json!({ "mode": "file_to_package", "result": result.stdout }))
            }
            "package_to_files" => {
                let result = crate::command::run("pacman", &["-Ql", query]).await?;
                Ok(serde_json::json!({ "mode": "package_to_files", "result": result.stdout }))
            }
            "filename_search" => {
                let result = crate::command::run("pacman", &["-Fs", query]).await?;
                Ok(serde_json::json!({ "mode": "filename_search", "result": result.stdout }))
            }
            _ => Err(ToolError::invalid_argument("Unknown mode")),
        }
    }
}

// --- Manage Groups ---

pub struct ManageGroups;

#[async_trait]
impl ToolHandler for ManageGroups {
    fn info(&self) -> Tool {
        Tool {
            name: "manage_groups".into(),
            description: "List package groups or packages within a group".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["list_groups", "list_packages_in_group"]
                    },
                    "group_name": {
                        "type": "string",
                        "description": "Group name (required for list_packages_in_group)"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        if !is_arch_linux() {
            return Err(ToolError::platform_not_arch());
        }
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_argument("Missing: action"))?;

        match action {
            "list_groups" => {
                let result = crate::command::run("pacman", &["-Sg"]).await?;
                let groups: Vec<&str> = result.stdout.lines().collect();
                Ok(serde_json::json!({ "groups": groups, "count": groups.len() }))
            }
            "list_packages_in_group" => {
                let group = args
                    .get("group_name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::invalid_argument("Missing: group_name"))?;
                let result = crate::command::run("pacman", &["-Sg", group]).await?;
                let packages: Vec<&str> = result.stdout.lines().collect();
                Ok(serde_json::json!({
                    "group": group,
                    "packages": packages,
                    "count": packages.len()
                }))
            }
            _ => Err(ToolError::invalid_argument("Unknown action")),
        }
    }
}
