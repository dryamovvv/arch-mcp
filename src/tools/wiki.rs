use crate::error::ToolError;
use crate::protocol::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::ToolHandler;

pub struct SearchArchWiki;

#[async_trait]
impl ToolHandler for SearchArchWiki {
    fn info(&self) -> Tool {
        Tool {
            name: "search_archwiki".into(),
            description: "Search the Arch Linux Wiki for articles matching the given query. Returns a list of article titles and summaries.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search term to find in the Arch Wiki"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results",
                        "default": 10
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_argument("Missing required argument: query"))?;
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;

        let results = search_wiki(query, limit).await?;
        Ok(serde_json::json!({ "results": results, "query": query }))
    }
}

async fn search_wiki(query: &str, limit: usize) -> Result<Vec<Value>, ToolError> {
    let url = format!(
        "https://wiki.archlinux.org/api.php?action=opensearch&search={}&limit={}&format=json",
        urlencode(query),
        limit
    );

    let resp: Vec<Value> = crate::client::get_json(&url).await?;

    let titles = resp
        .get(1)
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let descriptions = resp
        .get(2)
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let urls = resp
        .get(3)
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let results: Vec<Value> = titles
        .into_iter()
        .enumerate()
        .map(|(i, title)| {
            serde_json::json!({
                "title": title,
                "description": descriptions.get(i).cloned().unwrap_or(Value::Null),
                "url": urls.get(i).cloned().unwrap_or(Value::Null),
            })
        })
        .collect();

    Ok(results)
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
