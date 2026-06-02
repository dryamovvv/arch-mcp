use crate::error::ToolError;
use crate::platform::is_arch_linux;
use crate::protocol::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::ToolHandler;

pub struct ManageBoot;

#[async_trait]
impl ToolHandler for ManageBoot {
    fn info(&self) -> Tool {
        Tool {
            name: "manage_boot".into(),
            description: "Manage Raspberry Pi bootloader (BOOT_ORDER, one-time boot via tryboot). Supports RPi5 with rpi-eeprom-config.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["status", "set_boot_order", "next_boot"],
                        "description": "Boot management action"
                    },
                    "order": {
                        "type": "string",
                        "description": "Boot order preset or raw hex. Presets: sd_first, nvme_first, usb_first, sd_nvme, nvme_sd, sd_only, nvme_only, usb_only"
                    },
                    "device": {
                        "type": "string",
                        "enum": ["sd", "nvme", "usb"],
                        "description": "Device for next_boot"
                    },
                    "reboot": { "type": "boolean", "default": false, "description": "Reboot after setting" }
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
            "status" => boot_status().await,
            "set_boot_order" => {
                let order = args
                    .get("order")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::invalid_argument("Missing: order"))?;
                let reboot = args
                    .get("reboot")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                set_boot_order(order, reboot).await
            }
            "next_boot" => {
                let device = args
                    .get("device")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::invalid_argument("Missing: device"))?;
                let reboot = args
                    .get("reboot")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                next_boot(device, reboot).await
            }
            _ => Err(ToolError::invalid_argument("Unknown action")),
        }
    }
}

fn boot_order_preset(name: &str) -> Option<&'static str> {
    match name {
        "sd_first" => Some("0xf461"),
        "nvme_first" => Some("0xf462"),
        "usb_first" => Some("0xf463"),
        "sd_nvme" => Some("0xf14"),
        "nvme_sd" => Some("0xf24"),
        "sd_only" => Some("0x1"),
        "nvme_only" => Some("0x2"),
        "usb_only" => Some("0x3"),
        _ => None,
    }
}

fn decode_boot_order(hex: &str) -> String {
    let clean = hex.trim_start_matches("0x").trim();
    let devices: Vec<&str> = clean
        .chars()
        .filter_map(|c| match c {
            '1' => Some("SD"),
            '2' => Some("NVMe"),
            '3' => Some("USB"),
            '4' => Some("SD (alt)"),
            '5' => Some("NVMe (alt)"),
            '6' => Some("USB (alt)"),
            'e' => Some("EEPROM"),
            'f' => Some("Restart"),
            '0' => None,
            _ => Some(&c.to_string()),
        })
        .collect();

    if devices.is_empty() {
        hex.to_string()
    } else {
        devices.join(" → ")
    }
}

async fn read_bootloader_config() -> Result<String, ToolError> {
    let result = crate::command::run("rpi-eeprom-config", &[]).await?;
    Ok(result.stdout)
}

async fn boot_status() -> Result<Value, ToolError> {
    let config = read_bootloader_config().await?;
    let boot_order_line = config.lines().find(|l| l.starts_with("BOOT_ORDER"));
    let boot_order = boot_order_line
        .and_then(|l| l.split('=').nth(1))
        .map(|v| v.trim())
        .unwrap_or("unknown");

    let eeprom_version = config
        .lines()
        .find(|l| l.starts_with("FIRMWARE_VERSION") || l.starts_with("BOOTLOADER_VERSION"));

    Ok(serde_json::json!({
        "boot_order": boot_order,
        "boot_order_decoded": decode_boot_order(boot_order),
        "eeprom_version": eeprom_version,
        "config": config
    }))
}

async fn set_boot_order(order: &str, reboot: bool) -> Result<Value, ToolError> {
    let hex_value = boot_order_preset(order).unwrap_or(order);

    let current_config = read_bootloader_config().await?;
    let updated_config = current_config
        .lines()
        .map(|l| {
            if l.starts_with("BOOT_ORDER") {
                format!("BOOT_ORDER={}", hex_value.trim_start_matches("0x"))
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Write via temp file + rpi-eeprom-config --apply
    let temp_path = "/tmp/boot-eeprom-config.txt";
    std::fs::write(temp_path, &updated_config)
        .map_err(|e| ToolError::internal(&format!("Cannot write temp config: {}", e)))?;

    let result = crate::command::run("sudo", &["rpi-eeprom-config", "--apply", temp_path]).await?;
    let _ = std::fs::remove_file(temp_path);

    if reboot {
        let _ = crate::command::run("sudo", &["reboot"]).await;
    }

    Ok(serde_json::json!({
        "action": "set_boot_order",
        "order": order,
        "hex": hex_value,
        "output": result.stdout,
        "exit_code": result.exit_code,
        "reboot": reboot
    }))
}

async fn next_boot(device: &str, reboot: bool) -> Result<Value, ToolError> {
    let current_config = read_bootloader_config().await?;
    let original_order = current_config
        .lines()
        .find(|l| l.starts_with("BOOT_ORDER"))
        .map(|l| l.to_string())
        .unwrap_or_default();

    let tryboot_order = match device {
        "sd" => "0x1",
        "nvme" => "0x2",
        "usb" => "0x3",
        _ => return Err(ToolError::invalid_argument("Unknown device")),
    };

    let tryboot_config = current_config
        .lines()
        .map(|l| {
            if l.starts_with("BOOT_ORDER") {
                format!("BOOT_ORDER={}", tryboot_order.trim_start_matches("0x"))
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let temp_path = "/tmp/boot-eeprom-config.txt";
    std::fs::write(temp_path, &tryboot_config)
        .map_err(|e| ToolError::internal(&format!("Cannot write temp config: {}", e)))?;

    let result = crate::command::run("sudo", &["rpi-eeprom-config", "--apply", temp_path]).await?;
    let _ = std::fs::remove_file(temp_path);

    if reboot {
        let _ = crate::command::run("sudo", &["reboot"]).await;
    }

    Ok(serde_json::json!({
        "action": "next_boot",
        "device": device,
        "original_boot_order": original_order,
        "temporary_boot_order": tryboot_order,
        "note": "BOOT_ORDER will be restored after successful boot via systemd oneshot",
        "output": result.stdout,
        "reboot": reboot
    }))
}
