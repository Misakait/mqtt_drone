use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use log::{error, info, warn};
use rumqttc::{Event, QoS};
use serde_json;
use tokio::time::sleep;
use crate::config::AppConfig;
use crate::model::flight::FlightDto;
use crate::mqtt::{create_mqtt_client, subscribe_with_retry};
use crate::service::flight_service::FlightService;
use crate::service::ship_track_service::ShipTrackService;


// 运行MQTT事件循环
pub async fn run_mqtt_loop(
    config: AppConfig,
    track_service: Arc<ShipTrackService>,
    flight_service: Arc<FlightService>,
    flight_broadcaster: Arc<broadcast::Sender<String>>,
    location_broadcaster: Arc<broadcast::Sender<String>>,
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
                                        location_broadcaster.clone(),
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

/// 处理MQTT消息的主要分发函数
pub async fn handle_mqtt_message(
    track_service: Arc<ShipTrackService>,
    flight_service: Arc<FlightService>,
    flight_broadcaster: Arc<broadcast::Sender<String>>,
    location_broadcaster: Arc<broadcast::Sender<String>>,
    packet: rumqttc::Packet,
) {
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
            let task_type = parts[2];
            let task_id = parts[1].to_string();

            match task_type {
                "location" => {
                    info!("接收到位置更新任务: {}", task_id);
                    handle_location_message(track_service, task_id, payload, location_broadcaster).await;
                }
                "state" => {
                    info!("接收到状态更新任务: {}", task_id);
                    handle_state_message(flight_service, task_id, payload, flight_broadcaster).await;
                }
                _ => {
                    warn!("未知任务类型: {}", task_type);
                }
            }
        }
        _ => {}
    }
}

/// 处理flight状态消息并广播到WebSocket
pub async fn handle_state_message(
    flight_service: Arc<FlightService>,
    task_id: String,
    payload: Vec<u8>,
    flight_broadcaster: Arc<broadcast::Sender<String>>,
) {
    // 解析消息内容
    match serde_json::from_slice::<FlightDto>(&payload) {
        Ok(state) => {
            info!("handle_state_message:{:?}", state);
            if let Err(e) = flight_service.get(&task_id).await {
                error!("获取航行报告任务失败: {:?}", e);
                return;
            }
            match flight_service.append_data_and_update(&task_id, state.clone()).await {
                Ok(_) => {
                    info!("航行报告消息处理成功: {}", task_id);
                    
                    // 创建包含task_id的完整消息结构
                    let flight_message = serde_json::json!({
                        // "task_id": task_id,
                        "data": state
                    });
                    
                    // 将消息广播到所有WebSocket连接
                    if let Ok(json_str) = serde_json::to_string(&flight_message) {
                        if let Err(e) = flight_broadcaster.send(json_str) {
                            warn!("广播flight消息失败: {}", e);
                        } else {
                            info!("已广播flight消息到WebSocket客户端");
                        }
                    }
                }
                Err(e) => warn!("航行报告消息处理失败: {}", e),
            }
        }
        Err(e) => {
            error!("解析任务消息失败: {}", e);
        }
    }
}

/// 处理位置消息并广播到SSE
pub async fn handle_location_message(
    db_service: Arc<ShipTrackService>,
    task_id: String,
    payload: Vec<u8>,
    location_broadcaster: Arc<broadcast::Sender<String>>,
) {
    // 解析消息内容
    match serde_json::from_slice::<Vec<[f64;2]>>(&payload) {
        Ok(task) => {
            info!("taskid: {} ,longitude: {},and latitude: {}", task_id, task[0][0], task[0][1]);
            if let Err(e) = db_service.get(&task_id).await {
                error!("获取位置任务失败: {:?}", e);
                return;
            }
            match db_service.append_coordinates_and_update(&task_id, task.clone()).await {
                Ok(_) => {
                    info!("任务消息处理成功: {}", task_id);
                    
                    // 创建包含task_id和位置信息的完整消息结构
                    let location_message = serde_json::json!({
                        "longitude": task[0][0],
                        "latitude": task[0][1],
                    });
                    
                    // 将位置消息广播到所有SSE连接
                    if let Ok(json_str) = serde_json::to_string(&location_message) {
                        if let Err(e) = location_broadcaster.send(json_str) {
                            warn!("广播位置消息失败: {}", e);
                        } else {
                            info!("已广播位置消息到SSE客户端");
                        }
                    }
                }
                Err(e) => warn!("任务消息处理失败: {}", e),
            }
        }
        Err(e) => {
            error!("解析任务消息失败: {}", e);
        }
    }
}
