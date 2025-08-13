mod model;
mod service;
mod config;
mod mqtt;
mod websocket;
mod sse;

use std::sync::Arc;
use std::time::Duration;
use dotenv::dotenv;
use log::{error, info};
use mongodb::options::ClientOptions;
use mongodb::Client;
use pretty_env_logger::env_logger::Env;
use rumqttc::{Event, QoS};
use tokio::sync::broadcast;
use tokio::time::sleep;

use crate::config::AppConfig;
use crate::service::ship_track_service::ShipTrackService;
use crate::service::flight_service::FlightService;
use crate::mqtt::{create_mqtt_client, subscribe_with_retry, handle_mqtt_message, run_mqtt_loop};
use crate::websocket::start_websocket_server;
use crate::sse::start_sse_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    
    // 初始化日志
    pretty_env_logger::formatted_builder()
        .parse_env(Env::default().default_filter_or("info"))
        .init();
    
    // 加载配置
    let config = AppConfig::from_env()?;
    info!("配置加载成功");
    
    // 创建广播通道用于WebSocket推送flight消息
    let (flight_tx, _) = broadcast::channel::<String>(100);
    let flight_broadcaster = Arc::new(flight_tx);
    
    // 创建广播通道用于SSE推送位置消息
    let (location_tx, _) = broadcast::channel::<String>(100);
    let location_broadcaster = Arc::new(location_tx);

    // 配置MongoDB连接
    let client_options = ClientOptions::parse(&config.mongodb_uri).await?;
    let client = Client::with_options(client_options)?;
    let db = client.database("shipTracking");
    
    // 创建服务实例
    let track_collection = db.collection::<model::ship_track::ShipTrack>("trackSegments");
    let track_service = Arc::new(ShipTrackService::new(track_collection));
    
    let flight_collection = db.collection::<model::flight::Flight>("flights");
    let flight_service = Arc::new(FlightService::new(flight_collection));
    
    // 启动WebSocket服务器
    let flight_broadcaster_clone = flight_broadcaster.clone();
    tokio::spawn(async move {
        if let Err(e) = start_websocket_server(flight_broadcaster_clone).await {
            error!("WebSocket服务器启动失败: {}", e);
        }
    });
    
    // 启动SSE服务器
    let location_broadcaster_clone = location_broadcaster.clone();
    tokio::spawn(async move {
        if let Err(e) = start_sse_server(location_broadcaster_clone).await {
            error!("SSE服务器启动失败: {}", e);
        }
    });

    // 创建MQTT客户端并开始主循环
    run_mqtt_loop(config, track_service, flight_service, flight_broadcaster, location_broadcaster).await?;

    Ok(())
}
