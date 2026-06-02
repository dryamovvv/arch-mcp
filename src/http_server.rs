use crate::server::handle_mcp_request;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::sse::{Event, Sse},
    routing::{get, post},
    Json, Router,
};
use serde_json::Value;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tower_http::cors::CorsLayer;

struct AppState {
    session_tx: broadcast::Sender<String>,
}

pub async fn run_http(address: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (session_tx, _) = broadcast::channel::<String>(128);

    let state = Arc::new(AppState { session_tx });

    let app = Router::new()
        .route("/health", get(|| async { Json(serde_json::json!({"status": "ok"})) }))
        .route("/mcp", post(handle_direct_mcp).delete(handle_delete))
        .route("/sse", get(handle_sse))
        .route("/messages", post(handle_messages))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(address).await?;
    println!("HTTP/MCP server listening on {}", address);
    println!("Endpoints: POST /mcp (direct), GET /sse (SSE), POST /messages?session_id=xxx");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn handle_direct_mcp(
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if let Some(key) = std::env::var("ARCH_OPS_SERVER_API_KEY").ok().filter(|k| !k.is_empty()) {
        let auth = headers.get("authorization").and_then(|v| v.to_str().ok()).unwrap_or("");
        if !auth.starts_with("Bearer ") || auth.trim_start_matches("Bearer ") != key {
            return Err((StatusCode::UNAUTHORIZED, Json(serde_json::json!({
                "jsonrpc": "2.0", "error": {"code": -32001, "message": "Unauthorized"}, "id": null
            }))));
        }
    }

    match handle_mcp_request(&body).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "jsonrpc": "2.0", "error": {"code": -32603, "message": e.to_string()}, "id": body.get("id")
        })))),
    }
}

async fn handle_sse(
    State(state): State<Arc<AppState>>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.session_tx.subscribe();
    let session_id = uuid::Uuid::new_v4().to_string();
    let _ = state.session_tx.send(format!("data: {}\n\n", serde_json::json!({
        "session_id": session_id
    })));

    let stream = BroadcastStream::new(rx).filter_map(|msg| match msg {
        Ok(data) => Some(Ok(Event::default().data(data))),
        Err(_) => None,
    });

    Sse::new(stream)
}

async fn handle_messages(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let response = match handle_mcp_request(&body).await {
        Ok(r) => r,
        Err(e) => serde_json::json!({
            "jsonrpc": "2.0", "error": {"code": -32603, "message": e.to_string()}, "id": body.get("id")
        }),
    };

    let _ = state.session_tx.send(serde_json::to_string(&response).unwrap_or_default());

    Ok(Json(response))
}

async fn handle_delete() -> (StatusCode, &'static str) {
    (StatusCode::OK, "Connection closed")
}
