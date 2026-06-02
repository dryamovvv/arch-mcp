use crate::error::ToolError;
use crate::protocol::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::ToolHandler;

pub struct FetchNews;

#[async_trait]
impl ToolHandler for FetchNews {
    fn info(&self) -> Tool {
        Tool {
            name: "fetch_news".into(),
            description: "Fetch the latest Arch Linux news from the official RSS feed".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["latest", "critical", "since_update"],
                        "default": "latest"
                    },
                    "limit": { "type": "integer", "default": 10 },
                    "since_date": {
                        "type": "string",
                        "description": "Date filter (YYYY-MM-DD) for since_update"
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
            .unwrap_or("latest");
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;

        match action {
            "latest" => fetch_latest(limit).await,
            "critical" => fetch_critical(limit).await,
            "since_update" => {
                let since = args
                    .get("since_date")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                fetch_since_update(since, limit).await
            }
            _ => Err(ToolError::invalid_argument("Unknown action")),
        }
    }
}

async fn fetch_news_feed() -> Result<String, ToolError> {
    crate::client::get_text("https://archlinux.org/feeds/news/").await
}

fn parse_rss_items(xml: &str, limit: usize) -> Vec<Value> {
    let mut items = Vec::new();
    let re =
        regex::Regex::new(r"(?s)<item>.*?</item>").unwrap();
    let title_re = regex::Regex::new(r"<title>(.*?)</title>").unwrap();
    let link_re = regex::Regex::new(r"<link>(.*?)</link>").unwrap();
    let desc_re = regex::Regex::new(r"(?s)<description>(.*?)</description>").unwrap();
    let date_re = regex::Regex::new(r"<pubDate>(.*?)</pubDate>").unwrap();

    for (i, cap) in re.find_iter(xml).enumerate() {
        if i >= limit {
            break;
        }
        let entry = cap.as_str();
        let title = title_re
            .captures(entry)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str())
            .unwrap_or("")
            .to_string();
        let link = link_re
            .captures(entry)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str())
            .unwrap_or("")
            .to_string();
        let description = desc_re
            .captures(entry)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str())
            .unwrap_or("")
            .to_string();
        let date = date_re
            .captures(entry)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str())
            .unwrap_or("")
            .to_string();

        items.push(serde_json::json!({
            "title": title,
            "link": link,
            "description": description,
            "pub_date": date
        }));
    }
    items
}

async fn fetch_latest(limit: usize) -> Result<Value, ToolError> {
    let xml = fetch_news_feed().await?;
    let items = parse_rss_items(&xml, limit);
    Ok(serde_json::json!({ "news": items, "count": items.len() }))
}

async fn fetch_critical(limit: usize) -> Result<Value, ToolError> {
    let xml = fetch_news_feed().await?;
    let items = parse_rss_items(&xml, 50);
    let critical_keywords = [
        "manual intervention",
        "action required",
        "breaking change",
        "important",
        "urgent",
    ];
    let critical: Vec<Value> = items
        .into_iter()
        .filter(|item| {
            let text = format!(
                "{} {} {}",
                item.get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
                item.get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
                item.get("pub_date")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
            );
            critical_keywords
                .iter()
                .any(|kw| text.to_lowercase().contains(kw))
        })
        .take(limit)
        .collect();

    Ok(serde_json::json!({ "critical_news": critical, "count": critical.len() }))
}

async fn fetch_since_update(since_date: &str, limit: usize) -> Result<Value, ToolError> {
    let xml = fetch_news_feed().await?;
    let items = parse_rss_items(&xml, limit);

    if !since_date.is_empty() {
        let filtered: Vec<Value> = items
            .into_iter()
            .filter(|item| {
                item.get("pub_date")
                    .and_then(|v| v.as_str())
                    .map(|d| d.contains(since_date))
                    .unwrap_or(true)
            })
            .collect();
        Ok(serde_json::json!({
            "news": filtered,
            "count": filtered.len(),
            "since": since_date
        }))
    } else {
        Ok(serde_json::json!({ "news": items, "count": items.len() }))
    }
}
