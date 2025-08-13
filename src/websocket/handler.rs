use std::sync::Arc;
use axum::extract::ws::{Message, WebSocket};
use tokio::sync::broadcast;
use futures::{sink::SinkExt, stream::StreamExt};
use log::info;

/// 处理WebSocket连接
pub async fn handle_websocket(socket: WebSocket, flight_broadcaster: Arc<broadcast::Sender<String>>) {
    let (mut sender, mut receiver) = socket.split();
    let mut flight_rx = flight_broadcaster.subscribe();
    
    info!("新的WebSocket连接已建立");

    // 创建一个任务来处理从广播通道接收消息并发送到WebSocket
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = flight_rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                info!("WebSocket发送失败，连接已断开");
                break;
            }
        }
        info!("发送任务结束 - 可能是广播通道关闭或连接断开");
    });

    // 创建一个任务来处理从WebSocket接收消息（用于保持连接和心跳检测）
    let mut recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(msg) => {
                    match msg {
                        Message::Text(text) => {
                            info!("收到WebSocket消息: {}", text);
                            // 这里可以处理客户端发送的消息，但不结束任务
                        }
                        Message::Binary(_) => {
                            info!("收到WebSocket二进制消息");
                        }
                        Message::Ping(_) => {
                            info!("收到WebSocket Ping消息");
                            // Axum会自动处理Ping/Pong，这里只是记录
                        }
                        Message::Pong(_) => {
                            info!("收到WebSocket Pong消息");
                        }
                        Message::Close(_) => {
                            info!("收到WebSocket关闭消息，客户端主动断开连接");
                            break;
                        }
                    }
                }
                Err(e) => {
                    info!("WebSocket接收消息出错，连接异常断开: {}", e);
                    break;
                }
            }
        }
        info!("接收任务结束 - 连接已断开");
    });

    // 等待任一任务完成，这通常意味着连接已断开
    tokio::select! {
        _ = &mut send_task => {
            info!("发送任务完成，可能是连接断开或广播通道关闭");
            // 如果发送任务结束，说明连接可能已断开，终止接收任务
            recv_task.abort();
        },
        _ = &mut recv_task => {
            info!("接收任务完成，客户端已断开连接");
            // 如果接收任务结束，说明客户端断开了连接，终止发送任务
            send_task.abort();
        }
    }

    info!("WebSocket连接已断开");
}
