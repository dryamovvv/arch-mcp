use crate::error::ToolError;
use crate::platform::is_arch_linux;
use crate::protocol::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::ToolHandler;

pub struct ManageHardware;

#[async_trait]
impl ToolHandler for ManageHardware {
    fn info(&self) -> Tool {
        Tool {
            name: "manage_hardware".into(),
            description: "RPi5 hardware diagnostics: health (vcgencmd), eeprom_info, eeprom_update, nvme_info, benchmark_disk".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["health", "eeprom_info", "eeprom_update", "nvme_info", "benchmark_disk"]
                    },
                    "channel": { "type": "string", "description": "vcgencmd channel" },
                    "path": { "type": "string", "default": "/", "description": "Path for disk benchmark" },
                    "size": { "type": "string", "default": "1G", "description": "Benchmark file size" }
                },
                "required": ["action"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        if !is_arch_linux() { return Err(ToolError::platform_not_arch()); }

        let action = args.get("action").and_then(|v| v.as_str()).ok_or_else(|| ToolError::invalid_argument("Missing: action"))?;

        match action {
            "health" => {
                let mut info = serde_json::Map::new();
                let checks: Vec<(&str, &str)> = vec![
                    ("measure_temp", "temp"),
                    ("measure_clock", "arm_freq"),
                    ("measure_volts", "core_volts"),
                    ("get_throttled", "throttled"),
                ];
                for (cmd, label) in &checks {
                    if let Ok(r) = crate::command::run("vcgencmd", &[cmd]).await {
                        info.insert(label.to_string(), Value::String(r.stdout.trim().into()));
                    }
                }
                Ok(Value::Object(info))
            }
            "eeprom_info" => {
                let result = crate::command::run("rpi-eeprom-config", &[]).await?;
                Ok(serde_json::json!({ "eeprom": result.stdout }))
            }
            "eeprom_update" => {
                let result = crate::command::run("sudo", &["rpi-eeprom-update", "-a"]).await?;
                Ok(serde_json::json!({ "update": result.stdout }))
            }
            "nvme_info" => {
                let result = crate::command::run("nvme", &["list"]).await;
                match result {
                    Ok(r) => Ok(serde_json::json!({ "nvme": r.stdout })),
                    Err(_) => Ok(serde_json::json!({ "nvme": "not available" })),
                }
            }
            "benchmark_disk" => {
                let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("/");
                let tmp = format!("{}/.arch-opsd-bench", path);
                let result = crate::command::run("dd", &["if=/dev/zero", &format!("of={}", tmp), "bs=1M", "count=1", "oflag=direct"]).await;
                let _ = std::fs::remove_file(&tmp);
                Ok(serde_json::json!({ "write_test": result.map(|r| r.stdout).unwrap_or_default(), "note": "1M direct write" }))
            }
            _ => Err(ToolError::invalid_argument("Unknown action")),
        }
    }
}
