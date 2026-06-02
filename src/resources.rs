use crate::error::ToolError;
use crate::platform::is_arch_linux;
use crate::protocol::{Resource, ResourceContents};
use crate::tools::ToolHandler;
use serde_json::Value;
use std::collections::HashMap;

pub fn list() -> Vec<Resource> {
    vec![
        Resource { uri: "archwiki://Installation_guide".into(), name: "Arch Wiki - Installation Guide".into(), description: "Fetch Arch Wiki pages as Markdown".into(), mime_type: "text/markdown".into() },
        Resource { uri: "aur://yay/pkgbuild".into(), name: "AUR - yay PKGBUILD".into(), description: "Fetch AUR package PKGBUILD files".into(), mime_type: "text/x-script.shell".into() },
        Resource { uri: "aur://yay/info".into(), name: "AUR - yay Package Info".into(), description: "Fetch AUR package metadata (votes, maintainer, etc)".into(), mime_type: "application/json".into() },
        Resource { uri: "archrepo://vim".into(), name: "Official Repository - Package Info".into(), description: "Fetch official repository package details".into(), mime_type: "application/json".into() },
        Resource { uri: "pacman://installed".into(), name: "System - Installed Packages".into(), description: "List installed packages on Arch Linux system".into(), mime_type: "application/json".into() },
        Resource { uri: "pacman://orphans".into(), name: "System - Orphan Packages".into(), description: "List orphaned packages (dependencies no longer required)".into(), mime_type: "application/json".into() },
        Resource { uri: "pacman://explicit".into(), name: "System - Explicitly Installed Packages".into(), description: "List packages explicitly installed by user".into(), mime_type: "application/json".into() },
        Resource { uri: "pacman://groups".into(), name: "System - Package Groups".into(), description: "List all available package groups".into(), mime_type: "application/json".into() },
        Resource { uri: "pacman://group/base-devel".into(), name: "System - Packages in base-devel Group".into(), description: "Example: List packages in a specific group".into(), mime_type: "application/json".into() },
        Resource { uri: "system://info".into(), name: "System - System Information".into(), description: "Get system information (kernel, arch, memory, uptime)".into(), mime_type: "application/json".into() },
        Resource { uri: "system://disk".into(), name: "System - Disk Space".into(), description: "Check disk space usage for critical paths".into(), mime_type: "application/json".into() },
        Resource { uri: "system://services/failed".into(), name: "System - Failed Services".into(), description: "List failed systemd services".into(), mime_type: "application/json".into() },
        Resource { uri: "system://logs/boot".into(), name: "System - Boot Logs".into(), description: "Get recent boot logs from journalctl".into(), mime_type: "text/plain".into() },
        Resource { uri: "archnews://latest".into(), name: "Arch News - Latest".into(), description: "Get latest Arch Linux news announcements".into(), mime_type: "application/json".into() },
        Resource { uri: "archnews://critical".into(), name: "Arch News - Critical".into(), description: "Get critical Arch Linux news requiring manual intervention".into(), mime_type: "application/json".into() },
        Resource { uri: "archnews://since-update".into(), name: "Arch News - Since Last Update".into(), description: "Get news posted since last pacman update".into(), mime_type: "application/json".into() },
        Resource { uri: "pacman://log/recent".into(), name: "Pacman Log - Recent Transactions".into(), description: "Get recent package transactions from pacman log".into(), mime_type: "application/json".into() },
        Resource { uri: "pacman://log/failed".into(), name: "Pacman Log - Failed Transactions".into(), description: "Get failed package transactions".into(), mime_type: "application/json".into() },
        Resource { uri: "mirrors://active".into(), name: "Mirrors - Active Configuration".into(), description: "Get currently configured mirrors".into(), mime_type: "application/json".into() },
        Resource { uri: "mirrors://health".into(), name: "Mirrors - Health Status".into(), description: "Get mirror configuration health assessment".into(), mime_type: "application/json".into() },
        Resource { uri: "config://pacman".into(), name: "Config - pacman.conf".into(), description: "Get parsed pacman.conf configuration".into(), mime_type: "application/json".into() },
        Resource { uri: "config://makepkg".into(), name: "Config - makepkg.conf".into(), description: "Get parsed makepkg.conf configuration".into(), mime_type: "application/json".into() },
        Resource { uri: "pacman://database/freshness".into(), name: "Pacman - Database Freshness".into(), description: "Check when package databases were last synchronized".into(), mime_type: "application/json".into() },
        Resource { uri: "system://health".into(), name: "System - Health Check".into(), description: "Comprehensive system health check report".into(), mime_type: "application/json".into() },
    ]
}

pub async fn read(uri: &str) -> Result<ResourceContents, ToolError> {
    if let Some(page) = uri.strip_prefix("archwiki://") {
        return read_wiki_page(page).await;
    }

    let no_scheme = if let Some(rest) = uri.strip_prefix("aur://") {
        read_aur(rest).await?
    } else if let Some(rest) = uri.strip_prefix("archrepo://") {
        read_archrepo(rest).await?
    } else if let Some(rest) = uri.strip_prefix("pacman://") {
        read_pacman(rest).await?
    } else if let Some(rest) = uri.strip_prefix("system://") {
        read_system(rest).await?
    } else if let Some(rest) = uri.strip_prefix("archnews://") {
        read_archnews(rest).await?
    } else if let Some(rest) = uri.strip_prefix("mirrors://") {
        read_mirrors(rest).await?
    } else if let Some(rest) = uri.strip_prefix("config://") {
        read_config(rest).await?
    } else {
        return Err(ToolError::invalid_argument(&format!("Unknown resource URI: {uri}")));
    };

    Ok(ResourceContents { uri: uri.into(), mime_type: "application/json".into(), text: no_scheme })
}

async fn read_aur(path: &str) -> Result<String, ToolError> {
    let parts: Vec<&str> = path.split('/').collect();
    let pkg = parts[0];
    if pkg.is_empty() {
        return Err(ToolError::invalid_argument("AUR package name required"));
    }

    if parts.len() > 1 && parts[1] == "pkgbuild" {
        let url = format!("https://aur.archlinux.org/cgit/aur.git/plain/PKGBUILD?h={}", urlencode(pkg));
        let text = crate::client::get_text(&url).await?;
        return Ok(text);
    }

    let url = format!("https://aur.archlinux.org/rpc/v5/info/{}", urlencode(pkg));
    let val: Value = crate::client::get_json(&url).await?;
    Ok(serde_json::to_string_pretty(&val).unwrap_or_default())
}

pub(crate) fn urlencode(s: &str) -> String {
    s.chars().map(|c| match c {
        'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
        ' ' => "+".into(),
        _ => format!("%{:02X}", c as u8),
    }).collect()
}

async fn read_archrepo(pkg: &str) -> Result<String, ToolError> {
    if pkg.is_empty() {
        return Err(ToolError::invalid_argument("Package name required"));
    }
    let mut args = HashMap::new();
    args.insert("package_name".into(), Value::String(pkg.into()));
    let result = crate::tools::pacman::GetOfficialPackageInfo.call(args).await?;
    Ok(serde_json::to_string_pretty(&result).unwrap_or_default())
}

async fn read_pacman(path: &str) -> Result<String, ToolError> {
    if !is_arch_linux() {
        return Err(ToolError::platform_not_arch());
    }

    match path {
        "installed" => {
            let r = crate::command::run("pacman", &["-Q"]).await?;
            let packages: Vec<Value> = r.stdout.lines().filter(|l| !l.is_empty()).filter_map(|l| {
                let (name, version) = l.rsplit_once(' ')?;
                Some(serde_json::json!({"name": name, "version": version}))
            }).collect();
            Ok(serde_json::to_string_pretty(&packages).unwrap_or_default())
        }
        "orphans" => {
            let r = crate::command::run("pacman", &["-Qtdq"]).await?;
            let pkgs: Vec<&str> = r.stdout.lines().filter(|l| !l.is_empty()).collect();
            Ok(serde_json::to_string_pretty(&serde_json::json!({"orphans": pkgs, "count": pkgs.len()})).unwrap_or_default())
        }
        "explicit" => {
            let r = crate::command::run("pacman", &["-Qe"]).await?;
            let packages: Vec<Value> = r.stdout.lines().filter(|l| !l.is_empty()).filter_map(|l| {
                let (name, version) = l.rsplit_once(' ')?;
                Some(serde_json::json!({"name": name, "version": version}))
            }).collect();
            Ok(serde_json::to_string_pretty(&packages).unwrap_or_default())
        }
        "groups" => {
            let r = crate::command::run("pacman", &["-Sg"]).await?;
            let groups: Vec<&str> = r.stdout.lines().filter(|l| !l.is_empty()).collect();
            Ok(serde_json::to_string_pretty(&serde_json::json!({"groups": groups, "count": groups.len()})).unwrap_or_default())
        }
        _ => {
            if let Some(group) = path.strip_prefix("group/") {
                let r = crate::command::run("pacman", &["-Sg", group]).await?;
                let packages: Vec<&str> = r.stdout.lines().filter(|l| !l.is_empty()).collect();
                return Ok(serde_json::to_string_pretty(&serde_json::json!({"group": group, "packages": packages, "count": packages.len()})).unwrap_or_default());
            }
            if let Some(log_type) = path.strip_prefix("log/") {
                let qtype = match log_type {
                    "recent" => "all",
                    "failed" => "failures",
                    _ => return Err(ToolError::invalid_argument(&format!("Unsupported log resource: {log_type}"))),
                };
                let mut args = HashMap::new();
                args.insert("query_type".into(), Value::String(qtype.into()));
                args.insert("limit".into(), Value::Number(serde_json::Number::from(50)));
                let result = crate::tools::logs::QueryPackageHistory.call(args).await?;
                return Ok(serde_json::to_string_pretty(&result).unwrap_or_default());
            }
            if path == "database/freshness" {
                let result = crate::tools::packages::CheckDatabaseFreshness.call(HashMap::new()).await?;
                return Ok(serde_json::to_string_pretty(&result).unwrap_or_default());
            }
            Err(ToolError::invalid_argument(&format!("Unsupported pacman resource: {path}")))
        }
    }
}

async fn read_system(path: &str) -> Result<String, ToolError> {
    match path {
        "info" => {
            let result = crate::tools::system::GetSystemInfo.call(HashMap::new()).await?;
            Ok(serde_json::to_string_pretty(&result).unwrap_or_default())
        }
        "disk" => {
            let result = crate::command::run("df", &["-h", "/", "/home", "/var"]).await?;
            Ok(serde_json::to_string_pretty(&serde_json::json!({"disk_usage": result.stdout})).unwrap_or_default())
        }
        "services/failed" => {
            let r = crate::command::run("systemctl", &["--failed"]).await?;
            Ok(serde_json::to_string_pretty(&serde_json::json!({"failed_services": r.stdout})).unwrap_or_default())
        }
        "logs/boot" => {
            let r = crate::command::run("journalctl", &["-b", "-n", "100"]).await?;
            Ok(r.stdout)
        }
        "health" => {
            let result = crate::tools::health::RunSystemHealthCheck.call(HashMap::new()).await?;
            Ok(serde_json::to_string_pretty(&result).unwrap_or_default())
        }
        _ => Err(ToolError::invalid_argument(&format!("Unsupported system resource: {path}"))),
    }
}

async fn read_archnews(path: &str) -> Result<String, ToolError> {
    let action = match path {
        "latest" => "latest",
        "critical" => "critical",
        "since-update" => "since_update",
        _ => return Err(ToolError::invalid_argument(&format!("Unsupported archnews resource: {path}"))),
    };
    let mut args = HashMap::new();
    args.insert("action".into(), Value::String(action.into()));
    let result = crate::tools::news::FetchNews.call(args).await?;
    Ok(serde_json::to_string_pretty(&result).unwrap_or_default())
}

async fn read_mirrors(path: &str) -> Result<String, ToolError> {
    if !is_arch_linux() {
        return Err(ToolError::platform_not_arch());
    }
    let action = match path {
        "active" => "status",
        "health" => "health",
        _ => return Err(ToolError::invalid_argument(&format!("Unsupported mirrors resource: {path}"))),
    };
    let mut args = HashMap::new();
    args.insert("action".into(), Value::String(action.into()));
    let result = crate::tools::mirrors::OptimizeMirrors.call(args).await?;
    Ok(serde_json::to_string_pretty(&result).unwrap_or_default())
}

async fn read_config(path: &str) -> Result<String, ToolError> {
    if !is_arch_linux() {
        return Err(ToolError::platform_not_arch());
    }
    match path {
        "pacman" => {
            let mut args = HashMap::new();
            args.insert("focus".into(), Value::String("full".into()));
            let result = crate::tools::config::AnalyzePacmanConf.call(args).await?;
            Ok(serde_json::to_string_pretty(&result).unwrap_or_default())
        }
        "makepkg" => {
            let result = crate::tools::config::AnalyzeMakepkgConf.call(HashMap::new()).await?;
            Ok(serde_json::to_string_pretty(&result).unwrap_or_default())
        }
        _ => Err(ToolError::invalid_argument(&format!("Unsupported config resource: {path}"))),
    }
}

async fn read_wiki_page(page: &str) -> Result<ResourceContents, ToolError> {
    let url = format!(
        "https://wiki.archlinux.org/api.php?action=parse&page={}&format=json&prop=text",
        page
    );

    #[derive(serde::Deserialize)]
    struct ParseResponse { parse: Option<ParseResult> }
    #[derive(serde::Deserialize)]
    struct ParseResult { text: ParseText, #[allow(dead_code)] title: Option<String> }
    #[derive(serde::Deserialize)]
    struct ParseText { #[serde(rename = "*")] content: Option<String> }

    let resp: ParseResponse = crate::client::get_json(&url).await?;
    let text = resp.parse.and_then(|p| p.text.content).unwrap_or_default();

    Ok(ResourceContents { uri: format!("archwiki://{page}"), mime_type: "text/html".into(), text })
}
