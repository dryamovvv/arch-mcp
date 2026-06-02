use crate::error::ToolError;
use crate::protocol::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::ToolHandler;

// --- Manage Logs ---

pub struct ManageLogs;

#[async_trait]
impl ToolHandler for ManageLogs {
    fn info(&self) -> Tool {
        Tool {
            name: "manage_logs".into(),
            description: "View and filter systemd journal logs by unit, priority, time range, grep pattern, or boot session".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "unit": { "type": "string", "description": "Systemd unit name (e.g. sshd, pacman)" },
                    "priority": { "type": "string", "description": "Log priority (0=emerg .. 7=debug)" },
                    "lines": { "type": "integer", "default": 50 },
                    "since": { "type": "string", "description": "Start time (e.g. 'yesterday', '2024-01-01')" },
                    "until": { "type": "string", "description": "End time" },
                    "grep": { "type": "string", "description": "Filter pattern" },
                    "boot": { "type": "integer", "description": "Boot number (-1 = previous boot)" }
                }
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        let mut cmd: Vec<String> = vec![
            "journalctl".into(),
            "--no-pager".into(),
            "-o".into(),
            "short-iso".into(),
        ];

        if let Some(unit) = args.get("unit").and_then(|v| v.as_str()) {
            cmd.push("-u".into());
            cmd.push(unit.into());
        }
        if let Some(priority) = args.get("priority").and_then(|v| v.as_str()) {
            cmd.push("-p".into());
            cmd.push(priority.into());
        }
        if let Some(boot) = args.get("boot").and_then(|v| v.as_i64()) {
            cmd.push("-b".into());
            cmd.push(boot.to_string());
        }
        let lines = args
            .get("lines")
            .and_then(|v| v.as_u64())
            .unwrap_or(50);
        cmd.push("-n".into());
        cmd.push(lines.to_string());

        if let Some(since) = args.get("since").and_then(|v| v.as_str()) {
            cmd.push("--since".into());
            cmd.push(since.into());
        }
        if let Some(until) = args.get("until").and_then(|v| v.as_str()) {
            cmd.push("--until".into());
            cmd.push(until.into());
        }

        let refs: Vec<&str> = cmd.iter().map(|s| s.as_str()).collect();
        let result = crate::command::run("journalctl", &refs).await?;

        let output = if let Some(pattern) = args.get("grep").and_then(|v| v.as_str()) {
            let re = regex::Regex::new(pattern).unwrap_or_else(|_| regex::Regex::new(pattern).unwrap());
            result
                .stdout
                .lines()
                .filter(|l| re.is_match(l))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            result.stdout
        };

        Ok(serde_json::json!({
            "lines": output.lines().count(),
            "output": output,
            "command": refs[1..].join(" ")
        }))
    }
}

// --- Manage Journal Gateway ---

pub struct ManageJournalGateway;

#[async_trait]
impl ToolHandler for ManageJournalGateway {
    fn info(&self) -> Tool {
        Tool {
            name: "manage_journal_gateway".into(),
            description: "Manage and query systemd-journal-gatewayd: check status, query logs via HTTP, get recent errors".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["status", "query", "recent_errors"],
                        "description": "Gateway action"
                    },
                    "filter_param": { "type": "string", "description": "Filter parameter for query" },
                    "boot": { "type": "string", "description": "Boot identifier" }
                },
                "required": ["action"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_argument("Missing: action"))?;

        match action {
            "status" => {
                let result = crate::command::run("systemctl", &["status", "systemd-journal-gatewayd"]).await?;
                let is_active = result.stdout.contains("active (running)");
                Ok(serde_json::json!({
                    "service": "systemd-journal-gatewayd",
                    "active": is_active,
                    "status": result.stdout
                }))
            }
            "query" => {
                let filter = args.get("filter_param").and_then(|v| v.as_str()).unwrap_or("");
                let result = crate::command::run("journalctl", &["--no-pager", "-n", "50", &format!("--grep={}", filter)]).await?;
                Ok(serde_json::json!({
                    "action": "query",
                    "filter": filter,
                    "lines": result.stdout.lines().count(),
                    "output": result.stdout
                }))
            }
            "recent_errors" => {
                let result = crate::command::run("journalctl", &["--no-pager", "-p", "3", "-n", "50"]).await?;
                Ok(serde_json::json!({
                    "action": "recent_errors",
                    "priority": "err",
                    "lines": result.stdout.lines().count(),
                    "output": result.stdout
                }))
            }
            _ => Err(ToolError::invalid_argument("Unknown action")),
        }
    }
}
