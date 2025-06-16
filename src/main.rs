mod model;
mod service;

use std::env;
use crate::service::ship_track_service::ShipTrackService;
use log::{error, info, warn};
use mongodb::options::ClientOptions;
use mongodb::Client;
use pretty_env_logger::env_logger::Env;
use rumqttc::tokio_rustls::rustls::ClientConfig;
use rumqttc::{AsyncClient, Event, MqttOptions, QoS, TlsConfiguration, Transport};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use dotenv::dotenv;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    dotenv().ok();
    // 初始化日志
    pretty_env_logger::formatted_builder()
        .parse_env(Env::default().default_filter_or("info"))
        .init();
    // 从环境变量读取配置
    let mongodb_uri = env::var("MONGODB_URI").expect("MONGODB_URI 必须设置");
    let mqtt_host = env::var("MQTT_HOST").expect("MQTT_HOST 必须设置");
    let mqtt_port_str = env::var("MQTT_PORT").expect("MQTT_PORT 必须设置");
    let mqtt_port: u16 = mqtt_port_str
        .parse()
        .expect("MQTT_PORT 必须是一个有效的端口号");
    let mqtt_username = env::var("MQTT_USERNAME").expect("MQTT_USERNAME 必须设置");
    let mqtt_password = env::var("MQTT_PASSWORD").expect("MQTT_PASSWORD 必须设置");
    let ca_cert_path = env::var("CA_CERT_PATH").expect("CA_CERT_PATH 必须设置");
    
    //配置mongodb连接
    let client_options = ClientOptions::parse(mongodb_uri).await.unwrap();
    let client = Client::with_options(client_options).unwrap();
    let db = client.database("shipTracking");
    let collection = db.collection::<model::ship_track::ShipTrack>("trackSegments");
    let db_service = Arc::new(ShipTrackService::new(collection));
    // 配置MQTT客户端
    let mut mqttoptions = MqttOptions::new(&mqtt_username, mqtt_host, mqtt_port);
    mqttoptions.set_credentials(
        mqtt_username,  // 替换为实际用户名
        mqtt_password,  // 替换为实际密码
    );
    // 配置TLS
    let mut ca_file = File::open(ca_cert_path).await.unwrap();
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
                    if let Err(e) = db_service.get(&task_id).await {
                        error!("获取任务失败: {:?}", e);
                        return;
                    }
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