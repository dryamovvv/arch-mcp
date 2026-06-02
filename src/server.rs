use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::tools::ToolRegistry;
use serde_json::Value;
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

fn parse_request(line: &str) -> Option<JsonRpcRequest> {
    serde_json::from_str(line).ok()
}

fn serialize_response(resp: &JsonRpcResponse) -> String {
    serde_json::to_string(resp).unwrap_or_else(|_| {
        r#"{"id":null,"error":{"code":-32700,"message":"serialization error"}}"#.into()
    })
}

fn parse_args(params: Value) -> HashMap<String, Value> {
    match params {
        Value::Object(map) => map.into_iter().collect(),
        _ => HashMap::new(),
    }
}

pub async fn run_stdio() -> Result<(), Box<dyn std::error::Error>> {
    let registry = ToolRegistry::new();
    let stdin = tokio::io::stdin();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();
    let mut stdout = tokio::io::stdout();
    let mut initialized = false;

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        let request = match parse_request(&line) {
            Some(r) => r,
            None => {
                let out = serialize_response(&JsonRpcResponse::error(
                    Value::Null,
                    -32700,
                    "Parse error",
                ));
                stdout.write_all(out.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
                stdout.flush().await?;
                continue;
            }
        };

        let response = match request.method.as_str() {
            "initialize" => {
                initialized = true;
                JsonRpcResponse::success(
                    request.id,
                    serde_json::json!({
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {},
                            "resources": {},
                            "prompts": {}
                        },
                        "serverInfo": {
                            "name": "arch-opsd",
                            "version": "0.1.0"
                        }
                    }),
                )
            }
            "tools/list" => {
                if !initialized {
                    JsonRpcResponse::error(request.id, -32000, "Not initialized")
                } else {
                    JsonRpcResponse::success(
                        request.id,
                        serde_json::json!({ "tools": registry.list_tools() }),
                    )
                }
            }
            "tools/call" => {
                if !initialized {
                    JsonRpcResponse::error(request.id, -32000, "Not initialized")
                } else {
                    let name = request
                        .params
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let args = request
                        .params
                        .get("arguments")
                        .map(|v| parse_args(v.clone()))
                        .unwrap_or_default();

                    match registry.call(name, args).await {
                        Ok(result) => JsonRpcResponse::success(
                            request.id,
                            serde_json::json!({
                                "content": [{
                                    "type": "text",
                                    "text": serde_json::to_string_pretty(&result)
                                        .unwrap_or_default()
                                }]
                            }),
                        ),
                        Err(e) => JsonRpcResponse::error(
                            request.id,
                            -32603,
                            &format!("{:?}: {}", e.kind, e.message),
                        ),
                    }
                }
            }
            "resources/list" => {
                JsonRpcResponse::success(
                    request.id,
                    serde_json::json!({ "resources": crate::resources::list() }),
                )
            }
            "resources/read" => {
                let uri = request
                    .params
                    .get("uri")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                match crate::resources::read(uri).await {
                    Ok(contents) => JsonRpcResponse::success(
                        request.id,
                        serde_json::json!({ "contents": [contents] }),
                    ),
                    Err(e) => JsonRpcResponse::error(
                        request.id,
                        -32603,
                        &format!("{:?}: {}", e.kind, e.message),
                    ),
                }
            }
            "prompts/list" => {
                JsonRpcResponse::success(
                    request.id,
                    serde_json::json!({ "prompts": crate::prompts::list() }),
                )
            }
            "prompts/get" => {
                let name = request
                    .params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let args = request
                    .params
                    .get("arguments")
                    .map(|v| parse_args(v.clone()))
                    .unwrap_or_default();
                match crate::prompts::get(name, args).await {
                    Ok(msgs) => JsonRpcResponse::success(
                        request.id,
                        serde_json::json!({ "messages": msgs }),
                    ),
                    Err(e) => JsonRpcResponse::error(
                        request.id,
                        -32603,
                        &format!("{:?}: {}", e.kind, e.message),
                    ),
                }
            }
            "shutdown" => break,
            m => JsonRpcResponse::error(
                request.id,
                -32601,
                &format!("Method not found: {m}"),
            ),
        };

        let out = serialize_response(&response);
        stdout.write_all(out.as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;
    }

    Ok(())
}
