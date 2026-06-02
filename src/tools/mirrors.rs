#![allow(dead_code)]

use crate::error::ToolError;
use crate::protocol::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::ToolHandler;

pub struct OptimizeMirrors;

#[async_trait]
impl ToolHandler for OptimizeMirrors {
    fn info(&self) -> Tool {
        Tool {
            name: "optimize_mirrors".into(),
            description: "Manage and optimize pacman mirrors: status, speed test, suggestions, health check".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["status", "test", "suggest", "health"],
                        "default": "status"
                    },
                    "country": { "type": "string", "description": "Country filter for suggestions" },
                    "mirror_url": { "type": "string", "description": "Specific mirror URL for testing" },
                    "limit": { "type": "integer", "default": 10 },
                    "auto_test": { "type": "boolean", "default": false }
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
            "status" => list_mirrors().await,
            "test" => test_mirrors(&args).await,
            "suggest" => suggest_mirrors(&args).await,
            "health" => mirror_health().await,
            _ => Err(ToolError::invalid_argument("Unknown action")),
        }
    }
}

async fn list_mirrors() -> Result<Value, ToolError> {
    let content = std::fs::read_to_string("/etc/pacman.d/mirrorlist")
        .map_err(|e| ToolError::parse_error(&format!("Cannot read mirrorlist: {e}")))?;

    let servers: Vec<String> = content
        .lines()
        .filter(|l| l.starts_with("Server = "))
        .map(|l| l.trim_start_matches("Server = ").to_string())
        .collect();

    Ok(serde_json::json!({
        "mirror_count": servers.len(),
        "mirrorlist_path": "/etc/pacman.d/mirrorlist",
        "servers": servers
    }))
}

async fn test_mirrors(args: &HashMap<String, Value>) -> Result<Value, ToolError> {
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as usize;
    let specific = args
        .get("mirror_url")
        .and_then(|v| v.as_str());

    let servers = if let Some(url) = specific {
        vec![url.to_string()]
    } else {
        let content = std::fs::read_to_string("/etc/pacman.d/mirrorlist")
            .map_err(|e| ToolError::parse_error(&format!("Cannot read mirrorlist: {e}")))?;
        content
            .lines()
            .filter(|l| l.starts_with("Server = "))
            .map(|l| {
                l.trim_start_matches("Server = ")
                    .replace("$repo", "core")
                    .replace("$arch", "aarch64")
            })
            .take(limit)
            .collect()
    };

    let mut results = Vec::new();
    for server in &servers {
        let test_url = format!("{}/core.db", server.trim_end_matches('/'));
        match crate::client::head_latency(&test_url).await {
            Ok(duration) => {
                results.push(serde_json::json!({
                    "url": server,
                    "latency_ms": duration.as_millis(),
                    "status": "ok"
                }));
            }
            Err(e) => {
                results.push(serde_json::json!({
                    "url": server,
                    "error": e.message,
                    "status": "failed"
                }));
            }
        }
    }

    Ok(serde_json::json!({ "tested": results.len(), "results": results }))
}

#[derive(serde::Deserialize)]
struct MirrorStatus {
    urls: Vec<MirrorUrl>,
}

#[derive(serde::Deserialize)]
struct MirrorUrl {
    url: String,
    country: Option<String>,
    score: Option<f64>,
    delay: Option<f64>,
    last_sync: Option<String>,
}

async fn suggest_mirrors(args: &HashMap<String, Value>) -> Result<Value, ToolError> {
    let country = args
        .get("country")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as usize;

    let mut url = "https://archlinux.org/mirrors/status/json/".to_string();
    if !country.is_empty() {
        url = format!("{url}?country={country}");
    }

    let status: MirrorStatus = crate::client::get_json(&url).await?;
    let mut mirrors: Vec<&MirrorUrl> = status
        .urls
        .iter()
        .filter(|m| m.score.is_some() && m.delay.is_some())
        .collect();

    mirrors.sort_by(|a, b| {
        a.delay
            .unwrap_or(f64::MAX)
            .partial_cmp(&b.delay.unwrap_or(f64::MAX))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let suggestions: Vec<Value> = mirrors
        .iter()
        .take(limit)
        .map(|m| {
            serde_json::json!({
                "url": m.url,
                "country": m.country,
                "score": m.score,
                "delay_ms": m.delay.map(|d| (d * 1000.0) as u64),
                "last_sync": m.last_sync
            })
        })
        .collect();

    Ok(serde_json::json!({ "suggestions": suggestions, "count": suggestions.len() }))
}

#[derive(serde::Deserialize)]
struct MirrorStatusHealth {
    urls: Vec<MirrorUrlHealth>,
}

#[derive(serde::Deserialize)]
struct MirrorUrlHealth {
    url: String,
    last_sync: Option<String>,
    delay: Option<f64>,
    score: Option<f64>,
    completion_pct: Option<f64>,
}

async fn mirror_health() -> Result<Value, ToolError> {
    let status: MirrorStatusHealth =
        crate::client::get_json("https://archlinux.org/mirrors/status/json/").await?;

    let active = status
        .urls
        .iter()
        .filter(|m| m.last_sync.is_some())
        .count();
    let total = status.urls.len();
    let avg_delay: f64 = status
        .urls
        .iter()
        .filter_map(|m| m.delay)
        .sum::<f64>()
        / total.max(1) as f64;

    Ok(serde_json::json!({
        "total_mirrors": total,
        "active_mirrors": active,
        "avg_delay_ms": (avg_delay * 1000.0) as u64,
        "health_score": if active as f64 / total.max(1) as f64 > 0.8 {
            "good"
        } else {
            "degraded"
        }
    }))
}
