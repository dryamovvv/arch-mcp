use crate::error::ToolError;
use crate::platform::is_arch_linux;
use crate::protocol::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::ToolHandler;

// --- Analyze BTRFS ---

pub struct AnalyzeBtrfs;

#[async_trait]
impl ToolHandler for AnalyzeBtrfs {
    fn info(&self) -> Tool {
        Tool {
            name: "analyze_btrfs".into(),
            description: "Analyze BTRFS filesystem: filesystem_info, filesystem_df, subvolumes, device_stats, scrub_status, snapshots, snapper_configs".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": [
                            "filesystem_info", "filesystem_df", "filesystem_usage",
                            "subvolumes", "subvolume_info", "device_stats",
                            "device_usage", "properties", "scrub_status",
                            "snapshots", "snapper_configs"
                        ],
                        "description": "BTRFS analysis action"
                    },
                    "path": { "type": "string", "description": "Mount point or device path", "default": "/" },
                    "config": { "type": "string", "description": "Snapper config name (for snapshots)" }
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
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("/");

        match action {
            "filesystem_info" => btrfs_cmd(&["filesystem", "show", path]).await,
            "filesystem_df" => btrfs_cmd(&["filesystem", "df", path]).await,
            "filesystem_usage" => btrfs_cmd(&["filesystem", "usage", path]).await,
            "subvolumes" => btrfs_cmd(&["subvolume", "list", path]).await,
            "subvolume_info" => btrfs_cmd(&["subvolume", "show", path]).await,
            "device_stats" => btrfs_cmd(&["device", "stats", path]).await,
            "device_usage" => btrfs_cmd(&["device", "usage", path]).await,
            "properties" => btrfs_cmd(&["property", "list", path]).await,
            "scrub_status" => btrfs_cmd(&["scrub", "status", path]).await,
            "snapshots" => {
                let config = args
                    .get("config")
                    .and_then(|v| v.as_str())
                    .unwrap_or("root");
                snapper_cmd(&["list", "-c", config]).await
            }
            "snapper_configs" => {
                let result = crate::command::run("snapper", &["list-configs"]).await?;
                Ok(serde_json::json!({ "configs": result.stdout }))
            }
            _ => Err(ToolError::invalid_argument("Unknown action")),
        }
    }
}

async fn btrfs_cmd(args: &[&str]) -> Result<Value, ToolError> {
    let result = crate::command::run("btrfs", args).await?;
    Ok(serde_json::json!({
        "command": format!("btrfs {}", args.join(" ")),
        "output": result.stdout,
        "exit_code": result.exit_code
    }))
}

async fn snapper_cmd(args: &[&str]) -> Result<Value, ToolError> {
    let result = crate::command::run("snapper", args).await?;
    Ok(serde_json::json!({
        "command": format!("snapper {}", args.join(" ")),
        "output": result.stdout,
        "exit_code": result.exit_code
    }))
}

// --- Manage BTRFS Snapshots ---

pub struct ManageBtrfsSnapshots;

#[async_trait]
impl ToolHandler for ManageBtrfsSnapshots {
    fn info(&self) -> Tool {
        Tool {
            name: "manage_btrfs_snapshots".into(),
            description: "Manage BTRFS snapshots via snapper: list, configs, create, delete".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["list", "configs", "create", "delete"]
                    },
                    "description": { "type": "string", "description": "Snapshot description (for create)" },
                    "snap_type": { "type": "string", "enum": ["pre", "post"], "default": "pre", "description": "Snapshot type" },
                    "snapshot_id": { "type": "integer", "description": "Snapshot ID (for delete)" },
                    "config": { "type": "string", "default": "root", "description": "Snapper config" },
                    "cleanup": { "type": "string", "enum": ["number", "timeline", "empty-pre"], "description": "Cleanup algorithm" }
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
        let config = args
            .get("config")
            .and_then(|v| v.as_str())
            .unwrap_or("root");

        match action {
            "list" => snapper_cmd(&["list", "-c", config]).await,
            "configs" => {
                let result = crate::command::run("snapper", &["list-configs"]).await?;
                Ok(serde_json::json!({ "output": result.stdout }))
            }
            "create" => {
                let snap_type = args
                    .get("snap_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("pre");
                let desc = args
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("arch-opsd snapshot");
                let mut snapper_args = vec!["create", "-c", config, "-t", snap_type, "-d", desc];
                if let Some(cleanup) = args.get("cleanup").and_then(|v| v.as_str()) {
                    snapper_args.push("-c");
                    snapper_args.push(cleanup);
                }
                let result = crate::command::run("snapper", &snapper_args).await?;
                Ok(serde_json::json!({
                    "action": "create",
                    "config": config,
                    "type": snap_type,
                    "description": desc,
                    "output": result.stdout,
                    "exit_code": result.exit_code
                }))
            }
            "delete" => {
                let id = args
                    .get("snapshot_id")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| ToolError::invalid_argument("Missing: snapshot_id"))?;
                let result = crate::command::run(
                    "snapper",
                    &["delete", "-c", config, &id.to_string()],
                ).await?;
                Ok(serde_json::json!({
                    "action": "delete",
                    "config": config,
                    "snapshot_id": id,
                    "output": result.stdout,
                    "exit_code": result.exit_code
                }))
            }
            _ => Err(ToolError::invalid_argument("Unknown action")),
        }
    }
}

// --- Manage BTRFS Scrub ---

pub struct ManageBtrfsScrub;

#[async_trait]
impl ToolHandler for ManageBtrfsScrub {
    fn info(&self) -> Tool {
        Tool {
            name: "manage_btrfs_scrub".into(),
            description: "Manage BTRFS scrub operations: status, start, cancel".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["status", "start", "cancel"] },
                    "path": { "type": "string", "default": "/" },
                    "background": { "type": "boolean", "default": false }
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
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("/");

        match action {
            "status" => btrfs_cmd(&["scrub", "status", path]).await,
            "start" => {
                let background = args
                    .get("background")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let mut cmd = vec!["scrub", "start"];
                if background {
                    cmd.push("-B");
                }
                cmd.push(path);
                btrfs_cmd(&cmd).await
            }
            "cancel" => btrfs_cmd(&["scrub", "cancel", path]).await,
            _ => Err(ToolError::invalid_argument("Unknown action")),
        }
    }
}
