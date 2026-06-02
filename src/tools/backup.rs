use crate::error::ToolError;
use crate::platform::is_arch_linux;
use crate::protocol::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::ToolHandler;

pub struct ManageBackup;

#[async_trait]
impl ToolHandler for ManageBackup {
    fn info(&self) -> Tool {
        Tool {
            name: "manage_backup".into(),
            description: "Manage btrbk backups: run, list, status, restore_info".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["run", "list", "status", "restore_info"] },
                    "config": { "type": "string", "default": "/etc/btrbk/btrbk.conf" },
                    "dry_run": { "type": "boolean", "default": false },
                    "snapshot_id": { "type": "string", "description": "Snapshot ID for restore_info" }
                },
                "required": ["action"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        if !is_arch_linux() { return Err(ToolError::platform_not_arch()); }

        let action = args.get("action").and_then(|v| v.as_str()).ok_or_else(|| ToolError::invalid_argument("Missing: action"))?;
        let config = args.get("config").and_then(|v| v.as_str()).unwrap_or("/etc/btrbk/btrbk.conf");

        match action {
            "run" => {
                let dry_run = args.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false);
                let mut cmd = vec!["btrbk", "-c", config];
                if dry_run { cmd.push("-n"); }
                let result = crate::command::run("sudo", &cmd).await?;
                Ok(serde_json::json!({ "action": "run", "dry_run": dry_run, "output": result.stdout, "exit_code": result.exit_code }))
            }
            "list" => {
                let result = crate::command::run("sudo", &["btrbk", "-c", config, "list"]).await?;
                Ok(serde_json::json!({ "snapshots": result.stdout }))
            }
            "status" => {
                let result = crate::command::run("sudo", &["btrbk", "-c", config, "--verbose", "list"]).await?;
                Ok(serde_json::json!({ "status": result.stdout }))
            }
            "restore_info" => {
                let id = args.get("snapshot_id").and_then(|v| v.as_str()).unwrap_or("");
                let result = crate::command::run("sudo", &["btrbk", "-c", config, "list", id]).await?;
                Ok(serde_json::json!({ "restore_info": result.stdout }))
            }
            _ => Err(ToolError::invalid_argument("Unknown action")),
        }
    }
}
