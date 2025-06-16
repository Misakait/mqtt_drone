use std::time::Duration;
use log::{error, info};
use pretty_env_logger::env_logger::Env;
use rumqttc::{AsyncClient, Event, MqttOptions, QoS, TlsConfiguration, Transport};
use rumqttc::tokio_rustls::rustls::ClientConfig;
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    // 初始化日志
    pretty_env_logger::formatted_builder()
        .parse_env(Env::default().default_filter_or("info"))
        .init();

    // 配置MQTT客户端
    let mut mqttoptions = MqttOptions::new("rust", "n531cfaf.ala.cn-hangzhou.emqxsl.cn", 8883);
    mqttoptions.set_credentials(
        "misakait",  // 替换为实际用户名
        "alieencharlotte",  // 替换为实际密码
    );
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
    mqttoptions.set_clean_session(true);

    // 创建MQTT客户端和事件循环
    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

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
                        handle_mqtt_message(packet).await;
                    }
                    Event::Outgoing(_) => {
                        info!("Outgoing");
                    }
                }
            }
            Err(e) => {
                error!("事件循环错误: {}", e);
                // 可添加重连逻辑
                // sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

// 处理MQTT消息
async fn handle_mqtt_message(packet: rumqttc::Packet) {
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
            match serde_json::from_slice::<[f32;2]>(&payload) {
                Ok(task) => {
                    info!("taskid: {} ,longitude: {},and latitude: {}",task_id, task[0], task[1]);
                }
                Err(e) => {
                    error!("解析任务消息失败: {}", e);
                }
            }
        }
        _ => {}
    }
}