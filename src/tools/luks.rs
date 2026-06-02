use crate::error::ToolError;
use crate::platform::is_arch_linux;
use crate::protocol::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::ToolHandler;

pub struct ManageLuks;

#[async_trait]
impl ToolHandler for ManageLuks {
    fn info(&self) -> Tool {
        Tool {
            name: "manage_luks".into(),
            description: "Manage LUKS encryption: status, change_password, add_key, remove_key, is_unlocked".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["status", "change_password", "add_key", "remove_key", "is_unlocked"] },
                    "device": { "type": "string", "description": "LUKS device path (e.g. /dev/nvme0n1p2)" },
                    "old_pass": { "type": "string", "description": "Current passphrase" },
                    "new_pass": { "type": "string", "description": "New passphrase" },
                    "key_file": { "type": "string", "description": "Key file path" },
                    "slot": { "type": "integer", "description": "Key slot number" }
                },
                "required": ["action", "device"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        if !is_arch_linux() { return Err(ToolError::platform_not_arch()); }

        let action = args.get("action").and_then(|v| v.as_str()).ok_or_else(|| ToolError::invalid_argument("Missing: action"))?;
        let device = args.get("device").and_then(|v| v.as_str()).ok_or_else(|| ToolError::invalid_argument("Missing: device"))?;

        match action {
            "status" => {
                let result = crate::command::run("sudo", &["cryptsetup", "luksDump", device]).await?;
                Ok(serde_json::json!({ "device": device, "status": result.stdout }))
            }
            "is_unlocked" => {
                let result = crate::command::run("cryptsetup", &["status", device]).await;
                let unlocked = result.is_ok();
                Ok(serde_json::json!({ "device": device, "unlocked": unlocked }))
            }
            "change_password" => {
                let old = args.get("old_pass").and_then(|v| v.as_str()).ok_or_else(|| ToolError::invalid_argument("Missing: old_pass"))?;
                let new = args.get("new_pass").and_then(|v| v.as_str()).ok_or_else(|| ToolError::invalid_argument("Missing: new_pass"))?;
                let input = format!("{}\n{}\n", old, new);
                let result = crate::command::run_with_stdin("sudo", &["cryptsetup", "luksChangeKey", device], &input).await?;
                Ok(serde_json::json!({ "action": "change_password", "device": device, "exit_code": result.exit_code, "output": result.stdout }))
            }
            "add_key" => {
                let new = args.get("new_pass").and_then(|v| v.as_str()).ok_or_else(|| ToolError::invalid_argument("Missing: new_pass"))?;
                let result = crate::command::run_with_stdin("sudo", &["cryptsetup", "luksAddKey", device], &format!("{}\n", new)).await?;
                Ok(serde_json::json!({ "action": "add_key", "device": device, "exit_code": result.exit_code }))
            }
            "remove_key" => {
                let slot = args.get("slot").and_then(|v| v.as_u64()).ok_or_else(|| ToolError::invalid_argument("Missing: slot"))?;
                let result = crate::command::run("sudo", &["cryptsetup", "luksKillSlot", device, &slot.to_string()]).await?;
                Ok(serde_json::json!({ "action": "remove_key", "device": device, "slot": slot, "exit_code": result.exit_code }))
            }
            _ => Err(ToolError::invalid_argument("Unknown action")),
        }
    }
}
