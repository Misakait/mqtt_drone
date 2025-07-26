use std::sync::Arc;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::time::sleep;
use rumqttc::{AsyncClient, MqttOptions, QoS, TlsConfiguration, Transport};
use log::{error, info, warn};

/// 创建MQTT客户端配置
pub async fn create_mqtt_client(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    ca_cert_path: &str,
) -> Result<(AsyncClient, rumqttc::EventLoop), Box<dyn std::error::Error>> {
    let mut mqttoptions = MqttOptions::new(username, host, port);
    mqttoptions.set_credentials(username.to_string(), password.to_string());
    
    // 配置TLS
    let mut ca_file = File::open(ca_cert_path).await?;
    let mut ca = Vec::new();
    ca_file.read_to_end(&mut ca).await?;
    mqttoptions.set_transport(Transport::Tls(TlsConfiguration::Simple {
        ca,
        alpn: None,
        client_auth: None,
    }));
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    mqttoptions.set_clean_session(false);

    let (client, eventloop) = AsyncClient::new(mqttoptions, 10);
    Ok((client, eventloop))
}

/// 带重试的订阅函数
pub async fn subscribe_with_retry(client: &mut AsyncClient, topic: &str, qos: QoS, max_retries: u8) {
    for attempt in 1..=max_retries {
        match client.subscribe(topic, qos).await {
            Ok(_) => {
                info!("订阅成功: {}", topic);
                return;
            }
            Err(e) => {
                warn!("第{}次订阅{}失败: {}", attempt, topic, e);
                sleep(Duration::from_secs(2)).await;
            }
        }
    }
    error!("多次尝试后仍无法订阅: {}", topic);
}
