use crate::error::ToolError;
use crate::platform::is_arch_linux;
use crate::protocol::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::ToolHandler;

pub struct ManageFirewall;

#[async_trait]
impl ToolHandler for ManageFirewall {
    fn info(&self) -> Tool {
        Tool {
            name: "manage_firewall".into(),
            description: "Manage nftables firewall: list_rules, add_port, remove_port, validate, reload".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["list_rules", "add_port", "remove_port", "validate", "reload"] },
                    "port": { "type": "integer", "description": "Port number" },
                    "proto": { "type": "string", "enum": ["tcp", "udp"], "default": "tcp" },
                    "interface": { "type": "string", "description": "Network interface" },
                    "chain": { "type": "string", "default": "input", "description": "nftables chain" }
                },
                "required": ["action"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        if !is_arch_linux() { return Err(ToolError::platform_not_arch()); }

        let action = args.get("action").and_then(|v| v.as_str()).ok_or_else(|| ToolError::invalid_argument("Missing: action"))?;

        match action {
            "list_rules" => {
                let result = crate::command::run("sudo", &["nft", "list", "ruleset"]).await?;
                Ok(serde_json::json!({ "ruleset": result.stdout }))
            }
            "add_port" => {
                let port = args.get("port").and_then(|v| v.as_u64()).ok_or_else(|| ToolError::invalid_argument("Missing: port"))?;
                let proto = args.get("proto").and_then(|v| v.as_str()).unwrap_or("tcp");
                let chain = args.get("chain").and_then(|v| v.as_str()).unwrap_or("input");
                let result = crate::command::run("sudo", &["nft", "add", "rule", "inet", "filter", chain, &format!("{}", proto), "dport", &port.to_string(), "accept"]).await?;
                Ok(serde_json::json!({ "action": "add_port", "port": port, "proto": proto, "chain": chain, "exit_code": result.exit_code }))
            }
            "remove_port" => {
                let port = args.get("port").and_then(|v| v.as_u64()).ok_or_else(|| ToolError::invalid_argument("Missing: port"))?;
                let proto = args.get("proto").and_then(|v| v.as_str()).unwrap_or("tcp");
                let chain = args.get("chain").and_then(|v| v.as_str()).unwrap_or("input");
                let handle_result = crate::command::run("sudo", &["nft", "--handle", "list", "chain", "inet", "filter", chain]).await?;
                let handle_re = regex::Regex::new(&format!(r".* {} dport {} accept # handle (\d+)", regex::escape(proto), port)).ok();
                let handle = handle_result.stdout.lines().find_map(|l| {
                    handle_re.as_ref().and_then(|re| re.captures(l)).and_then(|c| c.get(1)).map(|m| m.as_str().to_string())
                }).ok_or_else(|| ToolError::invalid_argument("Rule not found"))?;
                let result = crate::command::run("sudo", &["nft", "delete", "rule", "inet", "filter", chain, "handle", &handle]).await?;
                Ok(serde_json::json!({ "action": "remove_port", "port": port, "exit_code": result.exit_code }))
            }
            "validate" => {
                let result = crate::command::run("sudo", &["nft", "-c", "list", "ruleset"]).await?;
                Ok(serde_json::json!({ "valid": result.exit_code == 0, "output": result.stderr }))
            }
            "reload" => {
                let result = crate::command::run("sudo", &["systemctl", "reload", "nftables"]).await?;
                Ok(serde_json::json!({ "action": "reload", "exit_code": result.exit_code, "output": result.stdout }))
            }
            _ => Err(ToolError::invalid_argument("Unknown action")),
        }
    }
}
