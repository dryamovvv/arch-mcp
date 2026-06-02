use crate::error::ToolError;
use crate::protocol::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::ToolHandler;

pub struct GetSystemInfo;

#[async_trait]
impl ToolHandler for GetSystemInfo {
    fn info(&self) -> Tool {
        Tool {
            name: "get_system_info".into(),
            description: "Get detailed system information: kernel, architecture, uptime, memory".into(),
            input_schema: serde_json::json!({ "type": "object", "properties": {} }),
        }
    }

    async fn call(&self, _args: HashMap<String, Value>) -> Result<Value, ToolError> {
        let hostname =
            std::fs::read_to_string("/proc/sys/kernel/hostname")
                .unwrap_or_default()
                .trim()
                .to_string();
        let arch = crate::platform::cpu_arch();

        let uptime_secs = std::fs::read_to_string("/proc/uptime")
            .ok()
            .and_then(|s| s.split('.').next().map(|n| n.to_string()))
            .and_then(|n| n.parse::<u64>().ok())
            .unwrap_or(0);

        let meminfo = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
        let mem_total = parse_meminfo(&meminfo, "MemTotal");
        let mem_avail = parse_meminfo(&meminfo, "MemAvailable");

        Ok(serde_json::json!({
            "hostname": hostname,
            "architecture": arch,
            "uptime_seconds": uptime_secs,
            "memory": {
                "total_kb": mem_total,
                "available_kb": mem_avail
            }
        }))
    }
}

fn parse_meminfo(content: &str, key: &str) -> u64 {
    content
        .lines()
        .find(|l| l.starts_with(key))
        .and_then(|l| l.split(':').nth(1))
        .and_then(|v| v.trim().split_whitespace().next())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

pub struct AnalyzeStorage;

#[async_trait]
impl ToolHandler for AnalyzeStorage {
    fn info(&self) -> Tool {
        Tool {
            name: "analyze_storage".into(),
            description: "Analyze disk usage or pacman cache statistics".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["disk_usage", "cache_stats"]
                    }
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
            "disk_usage" => {
                let result =
                    crate::command::run("df", &["-h", "/", "/home", "/var"]).await?;
                Ok(serde_json::json!({ "disk_usage": result.stdout }))
            }
            "cache_stats" => {
                let cache = std::path::Path::new("/var/cache/pacman/pkg");
                if cache.exists() {
                    let result =
                        crate::command::run("du", &["-sh", "/var/cache/pacman/pkg"]).await?;
                    let count = std::fs::read_dir(cache).map(|e| e.count()).unwrap_or(0);
                    Ok(serde_json::json!({
                        "cache_size": result.stdout.trim(),
                        "package_count": count,
                        "cache_path": "/var/cache/pacman/pkg"
                    }))
                } else {
                    Ok(serde_json::json!({
                        "cache_path": "/var/cache/pacman/pkg",
                        "exists": false
                    }))
                }
            }
            _ => Err(ToolError::invalid_argument("Unknown action")),
        }
    }
}

pub struct DiagnoseSystem;

#[async_trait]
impl ToolHandler for DiagnoseSystem {
    fn info(&self) -> Tool {
        Tool {
            name: "diagnose_system".into(),
            description: "Diagnose system issues: failed systemd services or boot logs".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["failed_services", "boot_logs"]
                    },
                    "lines": { "type": "integer", "default": 50 }
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
        let lines = args
            .get("lines")
            .and_then(|v| v.as_u64())
            .unwrap_or(50);

        match action {
            "failed_services" => {
                let result = crate::command::run("systemctl", &["--failed"]).await?;
                Ok(serde_json::json!({ "failed_services": result.stdout }))
            }
            "boot_logs" => {
                let result =
                    crate::command::run("journalctl", &["-b", "-n", &lines.to_string()])
                        .await?;
                Ok(serde_json::json!({ "boot_logs": result.stdout }))
            }
            _ => Err(ToolError::invalid_argument("Unknown action")),
        }
    }
}
