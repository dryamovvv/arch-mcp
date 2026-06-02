use crate::error::ToolError;
use crate::protocol::{Prompt, PromptArgument, PromptContent, PromptMessage};
use serde_json::Value;
use std::collections::HashMap;

pub fn list() -> Vec<Prompt> {
    vec![
        Prompt {
            name: "troubleshoot_issue".into(),
            description: "Diagnose an error using Arch Wiki knowledge".into(),
            arguments: vec![
                PromptArgument {
                    name: "error_message".into(),
                    description: "The error message to diagnose".into(),
                    required: true,
                },
                PromptArgument {
                    name: "context".into(),
                    description: "Additional context about the issue".into(),
                    required: false,
                },
            ],
        },
        Prompt {
            name: "safe_system_update".into(),
            description: "Run pre-update checks before a full system upgrade".into(),
            arguments: vec![],
        },
    ]
}

pub async fn get(
    name: &str,
    _args: HashMap<String, Value>,
) -> Result<Vec<PromptMessage>, ToolError> {
    match name {
        "troubleshoot_issue" => {
            Ok(vec![PromptMessage {
                role: "assistant".into(),
                content: PromptContent {
                    content_type: "text".into(),
                    text: "I can help you diagnose this issue. Please describe the error message you're seeing and any relevant context.".into(),
                },
            }])
        }
        "safe_system_update" => {
            Ok(vec![PromptMessage {
                role: "assistant".into(),
                content: PromptContent {
                    content_type: "text".into(),
                    text: "Before running a system update, I recommend:\n\
1. Check Arch News for critical updates\n\
2. Ensure sufficient disk space\n\
3. Check for failed services\n\
4. Verify pacman database freshness\n\
5. Review any ignored packages\n\
\nUse fetch_news, analyze_storage, diagnose_system, and other tools to verify.".into(),
                },
            }])
        }
        _ => Err(ToolError::invalid_argument(&format!(
            "Unknown prompt: {name}"
        ))),
    }
}
