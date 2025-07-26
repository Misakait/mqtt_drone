mod model;
mod service;
mod config;
mod mqtt;
mod websocket;

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
use crate::mqtt::{create_mqtt_client, subscribe_with_retry, handle_mqtt_message};
use crate::websocket::start_websocket_server;

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
        start_websocket_server(flight_broadcaster_clone).await;
    });
    
    // 创建MQTT客户端并开始主循环
    run_mqtt_loop(config, track_service, flight_service, flight_broadcaster).await?;
    
    Ok(())
}

/// 运行MQTT事件循环
async fn run_mqtt_loop(
    config: AppConfig,
    track_service: Arc<ShipTrackService>,
    flight_service: Arc<FlightService>,
    flight_broadcaster: Arc<broadcast::Sender<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        match create_mqtt_client(
            &config.mqtt_host,
            config.mqtt_port,
            &config.mqtt_username,
            &config.mqtt_password,
            &config.ca_cert_path,
        ).await {
            Ok((mut client, mut eventloop)) => {
                info!("MQTT客户端创建成功");
                
                // 订阅主题
                subscribe_with_retry(&mut client, "drone/+/location", QoS::AtLeastOnce, 3).await;
                subscribe_with_retry(&mut client, "drone/+/state", QoS::AtLeastOnce, 3).await;
                
                // 事件循环处理
                loop {
                    match eventloop.poll().await {
                        Ok(event) => {
                            match event {
                                Event::Incoming(packet) => {
                                    info!("收到消息: {:?}", packet);
                                    handle_mqtt_message(
                                        track_service.clone(),
                                        flight_service.clone(),
                                        flight_broadcaster.clone(),
                                        packet
                                    ).await;
                                }
                                Event::Outgoing(_) => {}
                            }
                        }
                        Err(e) => {
                            error!("事件循环错误: {}", e);
                            break; // 跳出内层循环，重新创建客户端
                        }
                    }
                }
            }
            Err(e) => {
                error!("创建MQTT客户端失败: {}", e);
            }
        }
        
        error!("尝试重新连接...");
        sleep(Duration::from_secs(5)).await;
    }
}
