use crate::error::ToolError;
use crate::platform::is_arch_linux;
use crate::protocol::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::ToolHandler;

pub struct ManageRecovery;

#[async_trait]
impl ToolHandler for ManageRecovery {
    fn info(&self) -> Tool {
        Tool {
            name: "manage_recovery".into(),
            description: "System recovery diagnostics: check_emergency, system_state, last_boot_issues, repair_fstab".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["check_emergency", "system_state", "last_boot_issues", "repair_fstab"]
                    },
                    "dry_run": { "type": "boolean", "default": true }
                },
                "required": ["action"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        if !is_arch_linux() { return Err(ToolError::platform_not_arch()); }

        let action = args.get("action").and_then(|v| v.as_str()).ok_or_else(|| ToolError::invalid_argument("Missing: action"))?;

        match action {
            "check_emergency" => {
                let result = crate::command::run("systemctl", &["is-active", "emergency"]).await?;
                let in_emergency = result.stdout.trim() == "active";
                Ok(serde_json::json!({ "in_emergency": in_emergency, "status": result.stdout.trim() }))
            }
            "system_state" => {
                let mut state = serde_json::Map::new();
                if let Ok(r) = crate::command::run("systemctl", &["is-system-running"]).await {
                    state.insert("system_state".into(), Value::String(r.stdout.trim().into()));
                }
                Ok(Value::Object(state))
            }
            "last_boot_issues" => {
                let result = crate::command::run("journalctl", &["-b", "-1", "-p", "3", "--no-pager", "-n", "30"]).await?;
                let has_errors = !result.stdout.trim().is_empty();
                Ok(serde_json::json!({ "has_errors": has_errors, "logs": result.stdout }))
            }
            "repair_fstab" => {
                let dry_run = args.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(true);
                let fstab = std::fs::read_to_string("/etc/fstab").unwrap_or_default();
                let issues: Vec<String> = fstab.lines().enumerate().filter_map(|(i, line)| {
                    let trimmed = line.trim();
                    if trimmed.starts_with('#') || trimmed.is_empty() { return None; }
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() < 6 { Some(format!("line {}: incomplete entry", i + 1)) }
                    else { None }
                }).collect();
                Ok(serde_json::json!({ "dry_run": dry_run, "issues": issues, "fstab_path": "/etc/fstab" }))
            }
            _ => Err(ToolError::invalid_argument("Unknown action")),
        }
    }
}
