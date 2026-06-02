use crate::error::ToolError;
use crate::platform::is_arch_linux;
use crate::protocol::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::ToolHandler;

// --- Manage Orphans ---

pub struct ManageOrphans;

#[async_trait]
impl ToolHandler for ManageOrphans {
    fn info(&self) -> Tool {
        Tool {
            name: "manage_orphans".into(),
            description: "List or remove orphan packages (unneeded dependencies)".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["list", "remove"],
                        "description": "Operation"
                    },
                    "dry_run": { "type": "boolean", "default": true },
                    "exclude": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Packages to exclude from removal"
                    }
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
            "list" => {
                let result = crate::command::run("pacman", &["-Qtdq"]).await?;
                let orphans: Vec<&str> =
                    result.stdout.lines().filter(|l| !l.is_empty()).collect();
                Ok(serde_json::json!({ "orphans": orphans, "count": orphans.len() }))
            }
            "remove" => {
                let dry_run = args
                    .get("dry_run")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let exclude: Vec<String> = args
                    .get("exclude")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                let result = crate::command::run("pacman", &["-Qtdq"]).await?;
                let orphans: Vec<&str> =
                    result.stdout.lines().filter(|l| !l.is_empty()).collect();
                let to_remove: Vec<&str> = orphans
                    .iter()
                    .filter(|p| !exclude.contains(&p.to_string()))
                    .copied()
                    .collect();

                if dry_run {
                    return Ok(serde_json::json!({
                        "action": "dry_run",
                        "to_remove": to_remove,
                        "count": to_remove.len(),
                        "note": "Use dry_run=false to actually remove"
                    }));
                }

                if to_remove.is_empty() {
                    return Ok(serde_json::json!({
                        "action": "remove",
                        "removed": 0,
                        "message": "No orphans to remove"
                    }));
                }

                let mut args = vec!["-R", "-s", "--noconfirm"];
                for p in &to_remove {
                    args.push(p);
                }
                let result = crate::command::run("sudo", &args).await?;
                Ok(serde_json::json!({
                    "action": "remove",
                    "removed": to_remove.len(),
                    "exit_code": result.exit_code,
                    "stdout": result.stdout
                }))
            }
            _ => Err(ToolError::invalid_argument("Unknown action")),
        }
    }
}

// --- Manage Install Reason ---

pub struct ManageInstallReason;

#[async_trait]
impl ToolHandler for ManageInstallReason {
    fn info(&self) -> Tool {
        Tool {
            name: "manage_install_reason".into(),
            description: "List or change install reason for packages (explicit vs dependency)".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["list", "mark_explicit", "mark_dependency"]
                    },
                    "package_name": { "type": "string" }
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
            "list" => {
                let explicit = crate::command::run("pacman", &["-Qqe"]).await?;
                let deps = crate::command::run("pacman", &["-Qqd"]).await?;
                let explicit_pkgs: Vec<&str> =
                    explicit.stdout.lines().filter(|l| !l.is_empty()).collect();
                let dep_pkgs: Vec<&str> =
                    deps.stdout.lines().filter(|l| !l.is_empty()).collect();
                Ok(serde_json::json!({
                    "explicit": explicit_pkgs,
                    "dependencies": dep_pkgs
                }))
            }
            "mark_explicit" => {
                let pkg = args
                    .get("package_name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::invalid_argument("Missing: package_name"))?;
                crate::command::run("sudo", &["pacman", "-D", "--asexplicit", pkg]).await?;
                Ok(serde_json::json!({ "status": "marked_explicit", "package": pkg }))
            }
            "mark_dependency" => {
                let pkg = args
                    .get("package_name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::invalid_argument("Missing: package_name"))?;
                crate::command::run("sudo", &["pacman", "-D", "--asdeps", pkg]).await?;
                Ok(serde_json::json!({ "status": "marked_dependency", "package": pkg }))
            }
            _ => Err(ToolError::invalid_argument("Unknown action")),
        }
    }
}

// --- Verify Package Integrity ---

pub struct VerifyPackageIntegrity;

#[async_trait]
impl ToolHandler for VerifyPackageIntegrity {
    fn info(&self) -> Tool {
        Tool {
            name: "verify_package_integrity".into(),
            description: "Verify installed package files against pacman database".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "package_name": {
                        "type": "string",
                        "description": "Package name (or \"all\")"
                    },
                    "thorough": {
                        "type": "boolean",
                        "default": false,
                        "description": "Use -Qkk (checksum verification)"
                    }
                },
                "required": ["package_name"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        if !is_arch_linux() {
            return Err(ToolError::platform_not_arch());
        }
        let pkg = args
            .get("package_name")
            .and_then(|v| v.as_str())
            .unwrap_or("all");
        let thorough = args
            .get("thorough")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let flag = if thorough { "-Qkk" } else { "-Qk" };
        let result = crate::command::run("pacman", &[flag, pkg]).await?;

        Ok(serde_json::json!({
            "exit_code": result.exit_code,
            "output": result.stdout,
            "errors": result.stderr
        }))
    }
}

// --- Check Database Freshness ---

pub struct CheckDatabaseFreshness;

#[async_trait]
impl ToolHandler for CheckDatabaseFreshness {
    fn info(&self) -> Tool {
        Tool {
            name: "check_database_freshness".into(),
            description: "Check when the pacman database was last synchronized".into(),
            input_schema: serde_json::json!({ "type": "object", "properties": {} }),
        }
    }

    async fn call(&self, _args: HashMap<String, Value>) -> Result<Value, ToolError> {
        if !is_arch_linux() {
            return Err(ToolError::platform_not_arch());
        }

        let sync_dir = std::path::Path::new("/var/lib/pacman/sync");
        if !sync_dir.exists() {
            return Ok(serde_json::json!({
                "status": "never_synced",
                "message": "No sync database found - run pacman -Sy"
            }));
        }

        let mut repos = Vec::new();
        if let Ok(entries) = std::fs::read_dir(sync_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("db") {
                    if let Ok(metadata) = std::fs::metadata(&path) {
                        if let Ok(modified) = metadata.modified() {
                            let name = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown")
                                .to_string();
                            let elapsed = modified.elapsed().map(|d| d.as_secs()).unwrap_or(0);
                            repos.push(serde_json::json!({
                                "name": name,
                                "last_sync_seconds_ago": elapsed,
                                "last_sync": format!("{:?}", modified)
                            }));
                        }
                    }
                }
            }
        }

        Ok(serde_json::json!({ "repos": repos }))
    }
}
