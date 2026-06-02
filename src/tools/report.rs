use crate::error::ToolError;
use crate::platform::is_arch_linux;
use crate::protocol::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::ToolHandler;

pub struct GenerateReport;

#[async_trait]
impl ToolHandler for GenerateReport {
    fn info(&self) -> Tool {
        Tool {
            name: "generate_report".into(),
            description: "Generate a structured Markdown health report. Actions: full, system, packages, storage, btrfs, mirrors, config".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["full", "system", "packages", "storage", "btrfs", "mirrors", "config"],
                        "default": "full"
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
            .unwrap_or("full");

        let mut sections: Vec<String> = Vec::new();
        sections.push("# Отчёт о состоянии системы\n".into());

        if action == "full" || action == "system" {
            sections.push(system_section().await);
        }

        if is_arch_linux() {
            if action == "full" || action == "packages" {
                sections.push(packages_section().await);
            }
            if action == "full" || action == "storage" {
                sections.push(storage_section().await);
            }
            if action == "full" || action == "mirrors" {
                sections.push(mirrors_section().await);
            }
            if action == "full" || action == "config" {
                sections.push(config_section().await);
            }
            if action == "full" || action == "btrfs" {
                sections.push(btrfs_section().await);
            }
        }

        Ok(serde_json::json!({
            "report": sections.join("\n"),
            "action": action
        }))
    }
}

async fn system_section() -> String {
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
    let uptime_h = uptime_secs / 3600;

    let meminfo = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mem_total = parse_meminfo(&meminfo, "MemTotal") / 1024;
    let mem_avail = parse_meminfo(&meminfo, "MemAvailable") / 1024;

    format!(
        "## Система\n\n\
| Параметр | Значение |\n\
|----------|----------|\n\
| Хост | {} |\n\
| Архитектура | {} |\n\
| Uptime | {} ч |\n\
| RAM всего | {} MB |\n\
| RAM доступно | {} MB |\n\n",
        hostname, arch, uptime_h, mem_total, mem_avail
    )
}

async fn packages_section() -> String {
    let mut s = "## Пакеты\n\n| Параметр | Значение |\n|----------|----------|\n".to_string();

    if let Ok(result) = crate::command::run("pacman", &["-Q"]).await {
        let count = result.stdout.lines().count();
        s.push_str(&format!("| Установлено | {} |\n", count));
    }
    if let Ok(result) = crate::command::run("pacman", &["-Qtdq"]).await {
        let orphans = result.stdout.lines().filter(|l| !l.is_empty()).count();
        s.push_str(&format!("| Сирот | {} |\n", orphans));
    }
    if let Ok(updates) = crate::command::run("checkupdates", &[]).await {
        let count = updates.stdout.lines().count();
        s.push_str(&format!("| Обновления | {} |\n", count));
    }

    s.push('\n');
    s
}

async fn storage_section() -> String {
    let mut s = "## Хранилище\n\n".to_string();

    if let Ok(df) = crate::command::run("df", &["-h", "/", "/home", "/var"]).await {
        s.push_str("```\n");
        s.push_str(&df.stdout);
        s.push_str("```\n\n");
    }

    let cache = std::path::Path::new("/var/cache/pacman/pkg");
    if cache.exists() {
        if let Ok(du) = crate::command::run("du", &["-sh", "/var/cache/pacman/pkg"]).await {
            let count = std::fs::read_dir(cache).map(|e| e.count()).unwrap_or(0);
            s.push_str(&format!(
                "Кэш pacman: {} ({} пакетов)\n\n",
                du.stdout.trim(),
                count
            ));
        }
    }

    s
}

async fn mirrors_section() -> String {
    let mut s = "## Зеркала\n\n| Параметр | Значение |\n|----------|----------|\n".to_string();

    match std::fs::read_to_string("/etc/pacman.d/mirrorlist") {
        Ok(content) => {
            let count = content.lines().filter(|l| l.starts_with("Server = ")).count();
            s.push_str(&format!("| Активных зеркал | {} |\n", count));
        }
        Err(_) => {
            s.push_str("| Ошибка | Не удалось прочитать mirrorlist |\n");
        }
    }

    s.push('\n');
    s
}

async fn config_section() -> String {
    let mut s = "## Конфигурация\n\n".to_string();

    s.push_str("### pacman.conf\n\n");
    match std::fs::read_to_string("/etc/pacman.conf") {
        Ok(content) => {
            let parallel: String = content
                .lines()
                .find(|l| l.trim().starts_with("ParallelDownloads"))
                .unwrap_or("не задан")
                .to_string();
            s.push_str(&format!("- ParallelDownloads: {}\n", parallel));
        }
        Err(_) => s.push_str("- Не удалось прочитать\n"),
    }

    s.push_str("\n### makepkg.conf\n\n");
    match std::fs::read_to_string("/etc/makepkg.conf") {
        Ok(content) => {
            let jobs: String = content
                .lines()
                .find(|l| l.trim().starts_with("MAKEFLAGS"))
                .unwrap_or("не задан")
                .to_string();
            s.push_str(&format!("- MAKEFLAGS: {}\n", jobs));
        }
        Err(_) => s.push_str("- Не удалось прочитать\n"),
    }

    s.push('\n');
    s
}

async fn btrfs_section() -> String {
    "## BTRFS\n\nРаздел в разработке.\n\n".into()
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
