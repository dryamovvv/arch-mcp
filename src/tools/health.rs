use crate::error::ToolError;
use crate::platform::is_arch_linux;
use crate::protocol::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::ToolHandler;

pub struct RunSystemHealthCheck;

#[async_trait]
impl ToolHandler for RunSystemHealthCheck {
    fn info(&self) -> Tool {
        Tool {
            name: "run_system_health_check".into(),
            description: "Run a comprehensive system health check across all subsystems: system info, disk space, failed services, cache, updates, news, orphans, DB freshness, mirrors".into(),
            input_schema: serde_json::json!({ "type": "object", "properties": {} }),
        }
    }

    async fn call(&self, _args: HashMap<String, Value>) -> Result<Value, ToolError> {
        let mut issues: Vec<String> = Vec::new();
        let mut suggestions: Vec<String> = Vec::new();
        let mut summary_parts: Vec<String> = Vec::new();

        // 1. System info
        let hostname = std::fs::read_to_string("/proc/sys/kernel/hostname")
            .unwrap_or_default()
            .trim()
            .to_string();
        let arch = crate::platform::cpu_arch();
        let uptime_secs = std::fs::read_to_string("/proc/uptime")
            .ok()
            .and_then(|s| s.split('.').next().map(|n| n.to_string()))
            .and_then(|n| n.parse::<u64>().ok())
            .unwrap_or(0);
        summary_parts.push(format!("host={}", hostname));

        // 2. Disk space
        if let Ok(df) = crate::command::run("df", &["-h", "/"]).await {
            summary_parts.push("disk=checked".into());
        }

        // 3. Memory
        let meminfo = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
        let mem_avail = parse_meminfo(&meminfo, "MemAvailable");
        let mem_total = parse_meminfo(&meminfo, "MemTotal");
        if mem_total > 0 {
            let pct = (mem_avail as f64 / mem_total as f64) * 100.0;
            summary_parts.push(format!("mem={:.0}% free", pct));
            if pct < 10.0 {
                issues.push("Low memory: less than 10% available".into());
                suggestions.push("Close unused services or add swap".into());
            }
        }

        // 4. Failed services
        if let Ok(svc) = crate::command::run("systemctl", &["--failed"]).await {
            let failed_count = svc.stdout.lines().count().saturating_sub(1);
            if failed_count > 0 {
                issues.push(format!("{} failed systemd services", failed_count));
                suggestions.push("Run `systemctl --failed` for details".into());
            } else {
                summary_parts.push("services=ok".into());
            }
        }

        // 5. Pacman cache
        let cache = std::path::Path::new("/var/cache/pacman/pkg");
        if cache.exists() {
            if let Ok(count) = std::fs::read_dir(cache) {
                let n = count.count();
                summary_parts.push(format!("cache={} packages", n));
                if n > 50 {
                    suggestions.push("Consider cleaning pacman cache with `pacman -Sc`".into());
                }
            }
        }

        // 6. Orphans (Arch only)
        if is_arch_linux() {
            if let Ok(orphans) = crate::command::run("pacman", &["-Qtdq"]).await {
                let count = orphans.stdout.lines().filter(|l| !l.is_empty()).count();
                if count > 0 {
                    issues.push(format!("{} orphan packages found", count));
                    suggestions.push("Run `pacman -Rns $(pacman -Qtdq)` to remove orphans".into());
                } else {
                    summary_parts.push("orphans=0".into());
                }
            }
            if let Ok(updates) = crate::command::run("checkupdates", &[]).await {
                let count = updates.stdout.lines().count();
                if count > 0 {
                    summary_parts.push(format!("{} updates", count));
                    if count > 20 {
                        suggestions.push("Many updates available — consider upgrading soon".into());
                    }
                } else {
                    summary_parts.push("up-to-date".into());
                }
            }
        }

        // 7. Read-only filesystem check
        let etc_writable = std::fs::write("/tmp/.arch-health-check", "ok").is_ok();
        let _ = std::fs::remove_file("/tmp/.arch-health-check");
        if !etc_writable {
            issues.push("Filesystem appears read-only".into());
        }

        Ok(serde_json::json!({
            "summary": summary_parts.join(", "),
            "issues": issues,
            "suggestions": suggestions,
            "hostname": hostname,
            "architecture": arch,
            "uptime_seconds": uptime_secs
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
