use std::sync::Arc;
use axum::{
    extract::{ws::WebSocketUpgrade, State},
    response::Response,
    routing::get,
    Router,
};
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use log::info;

use super::handle_websocket;

/// 启动WebSocket服务器
pub async fn start_websocket_server(flight_broadcaster: Arc<broadcast::Sender<String>>) -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        .route("/flight_ws", get(websocket_handler))
        .layer(CorsLayer::permissive())
        .with_state(flight_broadcaster);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    info!("WebSocket服务器启动在 ws://0.0.0.0:8080/flight_ws");
    
    axum::serve(listener, app).await?;
    Ok(())
}

/// WebSocket升级处理器
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(flight_broadcaster): State<Arc<broadcast::Sender<String>>>,
) -> Response {
    ws.on_upgrade(|socket| handle_websocket(socket, flight_broadcaster))
}
