use crate::error::ToolError;
use crate::platform::is_arch_linux;
use crate::protocol::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::ToolHandler;

const CONFIG_TXT: &str = "/boot/config.txt";
const CMDLINE_TXT: &str = "/boot/cmdline.txt";

pub struct ManageBootConfig;

#[async_trait]
impl ToolHandler for ManageBootConfig {
    fn info(&self) -> Tool {
        Tool {
            name: "manage_boot_config".into(),
            description: "Manage RPi boot configuration: read config.txt, read cmdline.txt, verify boot files, check initramfs hooks, check boot order".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["read_config", "read_cmdline", "verify_boot", "check_initramfs_hooks", "check_boot_order"]
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        if !is_arch_linux() { return Err(ToolError::platform_not_arch()); }

        let action = args.get("action").and_then(|v| v.as_str()).ok_or_else(|| ToolError::invalid_argument("Missing: action"))?;

        match action {
            "read_config" => {
                let content = std::fs::read_to_string(CONFIG_TXT)
                    .map_err(|e| ToolError::parse_error(&format!("Cannot read config.txt: {}", e)))?;
                let gpu_mem = content.lines().find(|l| l.starts_with("gpu_mem"));
                let arm_64bit = content.lines().find(|l| l.starts_with("arm_64bit"));
                Ok(serde_json::json!({
                    "config": content,
                    "gpu_mem": gpu_mem,
                    "arm_64bit": arm_64bit
                }))
            }
            "read_cmdline" => {
                let content = std::fs::read_to_string(CMDLINE_TXT)
                    .map_err(|e| ToolError::parse_error(&format!("Cannot read cmdline.txt: {}", e)))?;
                Ok(serde_json::json!({ "cmdline": content.trim() }))
            }
            "verify_boot" => {
                let files = ["kernel8.img", "initramfs-linux.img", "bcm2712-rpi-5-b.dtb", "config.txt", "cmdline.txt"];
                let boot_path = std::path::Path::new("/boot");
                let results: Vec<Value> = files.iter().map(|f| {
                    let exists = boot_path.join(f).exists();
                    serde_json::json!({ "file": f, "exists": exists })
                }).collect();
                Ok(serde_json::json!({ "boot_files": results }))
            }
            "check_initramfs_hooks" => {
                let content = std::fs::read_to_string("/etc/mkinitcpio.conf").unwrap_or_default();
                let hooks_line = content.lines().find(|l| l.trim().starts_with("HOOKS"));
                Ok(serde_json::json!({ "hooks": hooks_line }))
            }
            "check_boot_order" => {
                if let Ok(config) = std::fs::read_to_string("/proc/device-tree/chosen/boot-orders") {
                    let hex = config.trim().trim_start_matches('\u{0000}');
                    Ok(serde_json::json!({ "boot_order": hex }))
                } else {
                    Ok(serde_json::json!({ "boot_order": "not available on this system" }))
                }
            }
            _ => Err(ToolError::invalid_argument("Unknown action")),
        }
    }
}
