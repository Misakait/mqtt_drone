mod model;
mod service;

use std::sync::Arc;
use std::time::Duration;
use log::{error, info, warn};
use mongodb::Client;
use mongodb::options::ClientOptions;
use pretty_env_logger::env_logger::Env;
use rumqttc::{AsyncClient, Event, MqttOptions, QoS, TlsConfiguration, Transport};
use rumqttc::tokio_rustls::rustls::ClientConfig;
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::time::sleep;
use crate::service::ship_track_service::ShipTrackService;

#[tokio::main]
async fn main() {
    // 初始化日志
    pretty_env_logger::formatted_builder()
        .parse_env(Env::default().default_filter_or("info"))
        .init();
    //配置mongodb连接
    let client_options = ClientOptions::parse("mongodb://localhost:27017").await.unwrap();
    let client = Client::with_options(client_options).unwrap();
    let db = client.database("shipTracking");
    let collection = db.collection::<model::ship_track::ShipTrack>("trackSegments");
    let db_service = Arc::new(ShipTrackService::new(collection));
    // 配置MQTT客户端
    let mut mqttoptions = MqttOptions::new("rust", "n531cfaf.ala.cn-hangzhou.emqxsl.cn", 8883);
    mqttoptions.set_credentials(
        "misakait",  // 替换为实际用户名
        "alieencharlotte",  // 替换为实际密码
    );
    // 配置TLS
    let mut ca_file = File::open("ca.crt").await.unwrap();
    let mut ca = Vec::new();
    ca_file.read_to_end(&mut ca).await.expect("TODO: panic message");
    mqttoptions.set_transport(Transport::Tls(TlsConfiguration::Simple {
        ca,
        alpn: None,
        client_auth: None,
    }));
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    //TODO: 实机运行时候改成false
    mqttoptions.set_clean_session(false);

    // 创建MQTT客户端和事件循环
    let (mut client, mut eventloop) = AsyncClient::new(mqttoptions.clone(), 10);
    // 订阅任务相关主题
    client.subscribe("drone/+/location", QoS::AtLeastOnce).await.unwrap();
    // 事件循环处理
    loop {
        match eventloop.poll().await {
            Ok(event) => {
                match event {
                    Event::Incoming(packet) => {
                        info!("收到消息: {:?}", packet);
                        // 处理接收到的消息
                        handle_mqtt_message(db_service.clone(),packet).await;
                    }
                    Event::Outgoing(_) =>{}
                }
            }
            Err(e) => {
                error!("事件循环错误: {}", e);
                // 可添加重连逻辑
                // sleep(Duration::from_secs(1)).await;
                loop {
                    error!("尝试重新连接...");
                    let (new_client, new_eventloop) = AsyncClient::new(mqttoptions.clone(), 10);
                    client = new_client;
                    eventloop = new_eventloop;
                    info!("重新创建客户端和事件循环");

                    // 重新订阅主题
                    match client.subscribe("drone/+/location", QoS::AtLeastOnce).await {
                        Ok(_) => {
                            info!("已发送重新订阅请求，等待连接确认");
                            sleep(Duration::from_secs(5)).await; // 延迟后重试整个重连过程
                            break; // 跳出重连循环
                        }
                        Err(sub_err) => {
                            error!("订阅失败: {}", sub_err);
                            // 订阅失败，可能需要再次尝试重连或采取其他错误处理
                            sleep(Duration::from_secs(5)).await; // 延迟后重试整个重连过程
                        }
                    }
                }
            }
        }
    }
}

// 处理MQTT消息
async fn handle_mqtt_message(db_service: Arc<ShipTrackService>, packet: rumqttc::Packet) {
    match packet {
        rumqttc::Packet::Publish(publish) => {
            let topic = publish.topic.clone();
            let payload = publish.payload.to_vec();

            // 从主题中提取任务ID
            let parts: Vec<&str> = topic.split('/').collect();
            if parts.len() < 3 {
                error!("无效的主题格式: {}", topic);
                return;
            }

            let task_id = parts[1].to_string();

            // 解析消息内容
            match serde_json::from_slice::<Vec<[f64;2]>>(&payload) {
                Ok(task) => {
                    info!("taskid: {} ,longitude: {},and latitude: {}",task_id, task[0][0], task[0][1]);
                    match db_service.append_coordinates_and_update(&task_id, task).await {
                        Ok(_) => info!("任务消息处理成功: {}", task_id),
                        Err(e) => warn!("任务消息处理失败: {}", e),
                    } 
                }
                Err(e) => {
                    error!("解析任务消息失败: {}", e);
                }
            }
        }
        _ => {}
    }
}