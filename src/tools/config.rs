use crate::error::ToolError;
use crate::platform::is_arch_linux;
use crate::protocol::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::ToolHandler;

pub struct AnalyzePacmanConf;

#[async_trait]
impl ToolHandler for AnalyzePacmanConf {
    fn info(&self) -> Tool {
        Tool {
            name: "analyze_pacman_conf".into(),
            description: "Analyze pacman.conf for ignored packages, parallel downloads, and other settings".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "focus": {
                        "type": "string",
                        "enum": ["full", "ignored_packages", "parallel_downloads"],
                        "default": "full"
                    }
                },
                "required": ["focus"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        if !is_arch_linux() {
            return Err(ToolError::platform_not_arch());
        }
        let focus = args
            .get("focus")
            .and_then(|v| v.as_str())
            .unwrap_or("full");

        let content = std::fs::read_to_string("/etc/pacman.conf")
            .map_err(|e| ToolError::parse_error(&format!("Cannot read pacman.conf: {e}")))?;

        match focus {
            "full" => Ok(serde_json::json!({ "content": content })),
            "ignored_packages" => {
                let ignored: Vec<String> = content
                    .lines()
                    .filter(|l| l.trim().starts_with("IgnorePkg"))
                    .flat_map(|l| {
                        l.split('=')
                            .nth(1)
                            .map(|v| {
                                v.split_whitespace()
                                    .map(String::from)
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default()
                    })
                    .collect();
                Ok(serde_json::json!({ "ignored_packages": ignored }))
            }
            "parallel_downloads" => {
                let parallel: u32 = content
                    .lines()
                    .find(|l| l.trim().starts_with("ParallelDownloads"))
                    .and_then(|l| l.split('=').nth(1))
                    .and_then(|v| v.trim().parse().ok())
                    .unwrap_or(0);
                let recommendation = if parallel < 5 {
                    "Consider increasing ParallelDownloads to 5 or more"
                } else {
                    "Good setting"
                };
                Ok(serde_json::json!({
                    "parallel_downloads": parallel,
                    "recommendation": recommendation
                }))
            }
            _ => Err(ToolError::invalid_argument("Unknown focus")),
        }
    }
}

pub struct AnalyzeMakepkgConf;

#[async_trait]
impl ToolHandler for AnalyzeMakepkgConf {
    fn info(&self) -> Tool {
        Tool {
            name: "analyze_makepkg_conf".into(),
            description: "Analyze makepkg.conf for build configuration".into(),
            input_schema: serde_json::json!({ "type": "object", "properties": {} }),
        }
    }

    async fn call(&self, _args: HashMap<String, Value>) -> Result<Value, ToolError> {
        if !is_arch_linux() {
            return Err(ToolError::platform_not_arch());
        }
        let content = std::fs::read_to_string("/etc/makepkg.conf")
            .map_err(|e| ToolError::parse_error(&format!("Cannot read makepkg.conf: {e}")))?;

        let cflags = extract_var(&content, "CFLAGS");
        let jobs = extract_var(&content, "MAKEFLAGS").and_then(|v| {
            v.split('-')
                .nth(1)
                .and_then(|v| v.trim().parse::<u32>().ok())
        });

        Ok(serde_json::json!({
            "cflags": cflags,
            "jobs": jobs,
            "content": content
        }))
    }
}

fn extract_var(content: &str, var: &str) -> Option<String> {
    let pattern = format!(r#"^{}\s*=\s*["'](.*)["']"#, regex::escape(var));
    let re = regex::Regex::new(&pattern).ok()?;
    content.lines().find_map(|l| {
        re.captures(l)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
    })
}
