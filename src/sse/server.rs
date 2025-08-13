use std::sync::Arc;
use std::time::Duration;
use axum::{
    extract::State,
    response::Sse,
    routing::get,
    Router,
};
use axum::response::sse::{Event, KeepAlive};
use futures_util::stream::{self, Stream};
use log::{error, info};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tower_http::cors::CorsLayer;


pub struct SseState {
    pub location_tx: Arc<broadcast::Sender<String>>,
}

pub async fn start_sse_server(
    location_broadcaster: Arc<broadcast::Sender<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = SseState {
        location_tx: location_broadcaster,
    };

    let app = Router::new()
        .route("/sse/location", get(location_sse_handler))
      
        .layer(CorsLayer::permissive())
        .with_state(Arc::new(state));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await?;
    info!("SSE服务器启动在端口3001");
    info!("位置更新SSE端点: http://localhost:3001/sse/location");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn location_sse_handler(
    State(state): State<Arc<SseState>>,
) -> Sse<impl Stream<Item = Result<Event, axum::Error>>> {
    info!("新的SSE位置连接建立");
    let rx = state.location_tx.subscribe();
    let stream = BroadcastStream::new(rx)
        .filter_map(|result| {
            match result {
                Ok(data) => {
                    info!("向SSE客户端发送位置数据: {}", data);
                    Some(Ok(Event::default().data(data)))
                },
                Err(e) => {
                    error!("SSE位置广播错误: {}", e);
                    None
                }
            }
        });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

