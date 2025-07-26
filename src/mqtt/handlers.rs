use std::sync::Arc;
use tokio::sync::broadcast;
use log::{error, info, warn};
use serde_json;

use crate::model::flight::FlightDto;
use crate::service::flight_service::FlightService;
use crate::service::ship_track_service::ShipTrackService;

/// 处理MQTT消息的主要分发函数
pub async fn handle_mqtt_message(
    track_service: Arc<ShipTrackService>,
    flight_service: Arc<FlightService>,
    flight_broadcaster: Arc<broadcast::Sender<String>>,
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
                    handle_location_message(track_service, task_id, payload).await;
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

/// 处理位置消息
pub async fn handle_location_message(
    db_service: Arc<ShipTrackService>,
    task_id: String,
    payload: Vec<u8>,
) {
    // 解析消息内容
    match serde_json::from_slice::<Vec<[f64;2]>>(&payload) {
        Ok(task) => {
            info!("taskid: {} ,longitude: {},and latitude: {}", task_id, task[0][0], task[0][1]);
            if let Err(e) = db_service.get(&task_id).await {
                error!("获取位置任务失败: {:?}", e);
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
