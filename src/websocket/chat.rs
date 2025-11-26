use std::{collections::HashMap, sync::Arc, time};

use crate::{AppState, auth::extractors::AuthUser, websocket::handler::validate_user};
use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{
        StatusCode,
        header::{AUTHORIZATION, HeaderMap, HeaderValue},
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use http::HeaderName;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::{RwLock, broadcast};

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub sender_id: String,
    pub receiver_id: String,
    pub message: String,
    pub timestamp: u64,
}

pub struct PrivateChatState {
    pub connections: RwLock<HashMap<String, broadcast::Sender<String>>>,
}

impl PrivateChatState {
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
        }
    }
}

pub async fn private_chat_handler(
    ws: WebSocketUpgrade,
    AuthUser(user): AuthUser,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let sender_id = user.user_id;

    let receiver_id = match headers.get("receiver_id") {
        Some(v) => match v.to_str() {
            Ok(id) => id.to_string(),
            Err(_) => {
                return (StatusCode::BAD_REQUEST, "Invalid recevier_id header format")
                    .into_response();
            }
        },
        None => {
            return (StatusCode::BAD_REQUEST, "Missing receiver_id header").into_response();
        }
    };
    let sender_exists = validate_user(&sender_id, &state.pool).await;
    let receiver_exists = validate_user(&receiver_id, &state.pool).await;

    let mut headers = HeaderMap::new();
    let token = format!("Bearer {}", sender_id);
    let header_value = HeaderValue::from_str(&token).expect("invalid header value");
    headers.insert(AUTHORIZATION, header_value);

    let receiver_header = HeaderValue::from_str(&receiver_id).expect("Invalid header value");
    headers.insert(HeaderName::from_static("receiver_id"), receiver_header);

    match (sender_exists, receiver_exists) {
        (Some(_), Some(_)) => (
            headers.clone(),
            ws.on_upgrade(move |socket| {
                private_chat(socket, sender_id, receiver_id, state.chat.clone())
            }),
        )
            .into_response(),
        _ => {
            let mut resp =
                (StatusCode::BAD_REQUEST, "Invalid user_id or receiver_id").into_response();
            for (k, v) in headers.iter() {
                resp.headers_mut().append(k, v.clone());
            }
            resp
        }
    }
}

pub async fn private_chat(
    ws: WebSocket,
    sender_id: String,
    receiver_id: String,
    state: Arc<PrivateChatState>,
) {
    let (mut sender, mut receiver) = ws.split();

    let (tx, mut rx) = broadcast::channel(100);

    {
        let mut connections = state.connections.write().await;
        connections.insert(sender_id.clone(), tx.clone());
    }

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    let state_clone = state.clone();
    let sender_id_clone = sender_id.clone();
    let receiver_id_clone = receiver_id.clone();

    let mut recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            if let Ok(msg) = msg {
                match msg {
                    Message::Text(text) => {
                        send_to_user(
                            &state_clone,
                            &sender_id_clone,
                            &receiver_id_clone.clone(),
                            text.as_str(),
                        )
                        .await;
                    }

                    Message::Close(_) => {
                        break;
                    }
                    _ => {}
                }
            } else {
                break;
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    {
        let mut connections = state.connections.write().await;
        connections.remove(&sender_id.clone());
    }
}

pub async fn send_to_user(state: &PrivateChatState, sender_id: &str, receiver_id: &str, msg: &str) {
    let connections = state.connections.read().await;
    if let Some(tx) = connections.get(receiver_id) {
        let chat_message = ChatMessage {
            sender_id: sender_id.to_string(),
            receiver_id: receiver_id.to_string(),
            message: msg.to_string(),
            timestamp: time::SystemTime::now()
                .duration_since(time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let response = match serde_json::to_string(&chat_message) {
            Ok(json) => json,
            Err(e) => json!({
                "error": format!("Failed to serialize message: {}",e.to_string()),
                "sender_id": sender_id,
                "receiver_id": receiver_id,
                "message": msg,
            })
            .to_string(),
        };

        let _ = tx.send(response);
    }
}
