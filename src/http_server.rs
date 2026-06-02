use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use tower_http::cors::CorsLayer;

pub async fn run_http(address: &str) -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        .route("/health", get(|| async { Json(serde_json::json!({"status": "ok"})) }))
        .route("/mcp", post(mcp_handler))
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(address).await?;
    println!("HTTP server listening on {}", address);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn mcp_handler(
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "jsonrpc": "2.0",
        "id": body.get("id"),
        "result": { "note": "STDIO transport recommended for full MCP functionality" }
    })))
}
