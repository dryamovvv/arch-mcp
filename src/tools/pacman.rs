use crate::error::ToolError;
use crate::platform::is_arch_linux;
use crate::protocol::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::ToolHandler;

// --- Get Official Package Info ---

pub struct GetOfficialPackageInfo;

#[async_trait]
impl ToolHandler for GetOfficialPackageInfo {
    fn info(&self) -> Tool {
        Tool {
            name: "get_official_package_info".into(),
            description: "Get detailed information about an official Arch Linux package from pacman or the archlinux.org API".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "package_name": { "type": "string", "description": "Package name" }
                },
                "required": ["package_name"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        let pkg = args
            .get("package_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_argument("Missing: package_name"))?;

        if is_arch_linux() {
            let result = crate::command::run("pacman", &["-Si", pkg]).await;
            if let Ok(output) = result {
                if output.exit_code == 0 {
                    return Ok(parse_pacman_si(&output.stdout));
                }
            }
        }

        let url = format!(
            "https://archlinux.org/packages/search/json/?name={}",
            urlencode(pkg)
        );
        let api_result: Value = crate::client::get_json(&url).await?;
        let results = api_result
            .get("results")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();

        if results.is_empty() {
            return Err(ToolError::invalid_argument(&format!(
                "Package not found: {}",
                pkg
            )));
        }

        Ok(serde_json::json!({
            "package_name": pkg,
            "source": "archlinux.org API",
            "packages": results
        }))
    }
}

fn parse_pacman_si(output: &str) -> Value {
    let mut map = serde_json::Map::new();
    for line in output.lines() {
        if let Some((key, value)) = line.split_once(": ") {
            let k = key.trim().to_string();
            let v = value.trim().to_string();
            map.insert(k, Value::String(v));
        }
    }
    Value::Object(map)
}

fn urlencode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "+".into(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}

// --- Check Updates Dry Run ---

pub struct CheckUpdatesDryRun;

#[async_trait]
impl ToolHandler for CheckUpdatesDryRun {
    fn info(&self) -> Tool {
        Tool {
            name: "check_updates_dry_run".into(),
            description: "Check for available package updates without installing anything (uses checkupdates from pacman-contrib)".into(),
            input_schema: serde_json::json!({ "type": "object", "properties": {} }),
        }
    }

    async fn call(&self, _args: HashMap<String, Value>) -> Result<Value, ToolError> {
        if !is_arch_linux() {
            return Err(ToolError::platform_not_arch());
        }

        let result = crate::command::run("checkupdates", &[]).await?;

        if result.exit_code == 0 {
            let updates: Vec<&str> = result.stdout.lines().collect();
            Ok(serde_json::json!({
                "updates_available": updates.len(),
                "packages": updates
            }))
        } else if result.exit_code == 1 && result.stderr.contains("no updates") {
            Ok(serde_json::json!({
                "updates_available": 0,
                "message": "System is up to date"
            }))
        } else {
            Err(ToolError::pacman_failed(
                "checkupdates",
                &result.stderr,
            ))
        }
    }
}

// --- Remove Packages ---

pub struct RemovePackages;

#[async_trait]
impl ToolHandler for RemovePackages {
    fn info(&self) -> Tool {
        Tool {
            name: "remove_packages".into(),
            description: "Remove installed packages using pacman".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "packages": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Package names to remove"
                    },
                    "remove_dependencies": {
                        "type": "boolean",
                        "default": false,
                        "description": "Remove unneeded dependencies (-s)"
                    },
                    "force": {
                        "type": "boolean",
                        "default": false,
                        "description": "Force removal (-dd)"
                    }
                },
                "required": ["packages"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        if !is_arch_linux() {
            return Err(ToolError::platform_not_arch());
        }

        let packages = args
            .get("packages")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ToolError::invalid_argument("Missing: packages"))?;

        let remove_deps = args
            .get("remove_dependencies")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let force = args
            .get("force")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let pkg_names: Vec<&str> = packages.iter().filter_map(|v| v.as_str()).collect();
        if pkg_names.is_empty() {
            return Err(ToolError::invalid_argument("No valid package names provided"));
        }

        let mut pacman_args = vec!["-R"];
        if remove_deps {
            pacman_args.push("-s");
        }
        if force {
            pacman_args.push("-dd");
        }
        for p in &pkg_names {
            pacman_args.push(p);
        }

        let result = crate::command::run("sudo", &pacman_args).await?;

        Ok(serde_json::json!({
            "status": if result.exit_code == 0 { "removed" } else { "failed" },
            "packages": pkg_names,
            "exit_code": result.exit_code,
            "stdout": result.stdout,
            "stderr": result.stderr
        }))
    }
}
