#![allow(dead_code)]

use crate::error::ToolError;
use crate::protocol::{Resource, ResourceContents};

pub fn list() -> Vec<Resource> {
    vec![
        Resource {
            uri: "archwiki://Installation_guide".into(),
            name: "Arch Wiki: Installation Guide".into(),
            description: "The Arch Linux installation guide".into(),
            mime_type: "text/markdown".into(),
        },
    ]
}

pub async fn read(uri: &str) -> Result<ResourceContents, ToolError> {
    if let Some(page) = uri.strip_prefix("archwiki://") {
        return read_wiki_page(page).await;
    }
    Err(ToolError::invalid_argument(&format!(
        "Unknown resource URI: {uri}"
    )))
}

async fn read_wiki_page(page: &str) -> Result<ResourceContents, ToolError> {
    let url = format!(
        "https://wiki.archlinux.org/api.php?action=parse&page={}&format=json&prop=text",
        page
    );

    #[derive(serde::Deserialize)]
    struct ParseResponse {
        parse: Option<ParseResult>,
    }

    #[derive(serde::Deserialize)]
    struct ParseResult {
        text: ParseText,
        title: Option<String>,
    }

    #[derive(serde::Deserialize)]
    struct ParseText {
        #[serde(rename = "*")]
        content: Option<String>,
    }

    let resp: ParseResponse = crate::client::get_json(&url).await?;
    let text = resp
        .parse
        .and_then(|p| p.text.content)
        .unwrap_or_default();

    Ok(ResourceContents {
        uri: format!("archwiki://{page}"),
        mime_type: "text/html".into(),
        text,
    })
}
