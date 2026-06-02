use crate::error::ToolError;
use crate::platform::is_arch_linux;
use crate::protocol::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::ToolHandler;

pub struct ManageTelegramUnlock;

#[async_trait]
impl ToolHandler for ManageTelegramUnlock {
    fn info(&self) -> Tool {
        Tool {
            name: "manage_telegram_unlock".into(),
            description: "Telegram-based LUKS unlock: check status, test bot connection, send unlock command".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["status", "test_bot", "send_unlock"]
                    },
                    "token": { "type": "string", "description": "Telegram bot token" },
                    "chat_id": { "type": "string", "description": "Telegram chat ID" },
                    "password": { "type": "string", "description": "LUKS password to send" }
                },
                "required": ["action"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        if !is_arch_linux() { return Err(ToolError::platform_not_arch()); }

        let action = args.get("action").and_then(|v| v.as_str()).ok_or_else(|| ToolError::invalid_argument("Missing: action"))?;

        match action {
            "status" => {
                let result = crate::command::run("systemctl", &["status", "telegram-unlock"]).await;
                Ok(serde_json::json!({
                    "service": "telegram-unlock",
                    "active": result.map(|r| r.stdout.contains("active")).unwrap_or(false)
                }))
            }
            "test_bot" => {
                let token = args.get("token").and_then(|v| v.as_str()).ok_or_else(|| ToolError::invalid_argument("Missing: token"))?;
                let url = format!("https://api.telegram.org/bot{}/getMe", token);
                let result: Value = crate::client::get_json(&url).await?;
                Ok(result)
            }
            "send_unlock" => {
                let token = args.get("token").and_then(|v| v.as_str()).ok_or_else(|| ToolError::invalid_argument("Missing: token"))?;
                let chat_id = args.get("chat_id").and_then(|v| v.as_str()).ok_or_else(|| ToolError::invalid_argument("Missing: chat_id"))?;
                let password = args.get("password").and_then(|v| v.as_str()).ok_or_else(|| ToolError::invalid_argument("Missing: password"))?;
                let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
                let body = serde_json::json!({ "chat_id": chat_id, "text": format!("LUKS_UNLOCK:{}", password) });
                let client = reqwest::Client::new();
                let resp = client.post(&url).json(&body).send().await
                    .map_err(|e| ToolError::network_error(&e.to_string()))?;
                Ok(serde_json::json!({ "status": resp.status().as_u16() }))
            }
            _ => Err(ToolError::invalid_argument("Unknown action")),
        }
    }
}
