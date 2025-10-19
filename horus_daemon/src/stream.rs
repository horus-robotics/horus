use crate::AppState;
use axum::{
    extract::{
        ws::{WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use horus_core::core::log_buffer::GLOBAL_LOG_BUFFER;
use std::time::Duration;
use tokio::time;

/// WebSocket handler for streaming logs from a specific deployment
pub async fn stream_deployment_logs(
    Path(deployment_id): Path<String>,
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, deployment_id, state))
}

async fn handle_socket(socket: WebSocket, deployment_id: String, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    // Check if deployment exists
    if state.registry.get(&deployment_id).is_none() {
        let _ = sender
            .send(axum::extract::ws::Message::Text(
                serde_json::json!({
                    "error": "Deployment not found"
                })
                .to_string(),
            ))
            .await;
        return;
    }

    let node_name = format!("deploy-{}", deployment_id);
    let mut last_count = 0;

    // Start streaming logs
    let mut interval = time::interval(Duration::from_millis(100));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                // Get new logs for this deployment
                let logs = GLOBAL_LOG_BUFFER.get_for_node(&node_name);

                if logs.len() > last_count {
                    // Send new logs
                    for log in &logs[last_count..] {
                        let msg = serde_json::json!({
                            "timestamp": log.timestamp,
                            "log_type": format!("{:?}", log.log_type),
                            "message": log.message,
                            "topic": log.topic,
                        });

                        if sender.send(axum::extract::ws::Message::Text(msg.to_string())).await.is_err() {
                            // Client disconnected
                            return;
                        }
                    }
                    last_count = logs.len();
                }

                // Check if process is still running
                if let Some(info) = state.registry.get(&deployment_id) {
                    use crate::process::ProcessStatus;
                    match info.status {
                        ProcessStatus::Running => {},
                        _ => {
                            // Process stopped, send final status and close
                            let _ = sender.send(axum::extract::ws::Message::Text(
                                serde_json::json!({
                                    "status": format!("{:?}", info.status),
                                    "exit_code": info.exit_code,
                                    "message": "Process terminated"
                                }).to_string()
                            )).await;
                            return;
                        }
                    }
                }
            }
            msg = receiver.next() => {
                match msg {
                    Some(Ok(axum::extract::ws::Message::Close(_))) | None => {
                        // Client closed connection
                        return;
                    }
                    Some(Ok(axum::extract::ws::Message::Ping(data))) => {
                        let _ = sender.send(axum::extract::ws::Message::Pong(data)).await;
                    }
                    _ => {}
                }
            }
        }
    }
}
