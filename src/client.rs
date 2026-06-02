#![allow(dead_code)]

use crate::error::ToolError;
use once_cell::sync::OnceCell;
use reqwest::Client;
use serde::de::DeserializeOwned;
use scraper::Html;

fn http_client() -> &'static Client {
    static CLIENT: OnceCell<Client> = OnceCell::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .user_agent("arch-opsd/0.1.0")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("reqwest Client::build")
    })
}

pub async fn get_json<T: DeserializeOwned>(url: &str) -> Result<T, ToolError> {
    http_client()
        .get(url)
        .send()
        .await
        .map_err(|e| ToolError::network_error(&format!("GET {}: {}", url, e)))?
        .json::<T>()
        .await
        .map_err(|e| ToolError::network_error(&format!("parse {}: {}", url, e)))
}

pub async fn get_text(url: &str) -> Result<String, ToolError> {
    http_client()
        .get(url)
        .send()
        .await
        .map_err(|e| ToolError::network_error(&format!("GET {}: {}", url, e)))?
        .text()
        .await
        .map_err(|e| ToolError::network_error(&format!("read {}: {}", url, e)))
}

pub async fn get_html(url: &str) -> Result<Html, ToolError> {
    let body = get_text(url).await?;
    Ok(Html::parse_document(&body))
}

pub async fn head_latency(url: &str) -> Result<std::time::Duration, ToolError> {
    let start = std::time::Instant::now();
    http_client()
        .head(url)
        .send()
        .await
        .map_err(|e| ToolError::network_error(&format!("HEAD {}: {}", url, e)))?;
    Ok(start.elapsed())
}
