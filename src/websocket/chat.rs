use std::{collections::HashMap, sync::Arc, time};

use crate::{
    AppState,
    auth::{extractors::AuthUser, user::User},
    websocket::handler::validate_user,
};
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
    pub sender_user: User,
    pub receiver_user: User,
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
        (Some(sender), Some(receiver)) => (
            headers.clone(),
            ws.on_upgrade(move |socket| private_chat(socket, sender, receiver, state.chat.clone())),
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
    sender_user: User,
    receiver_user: User,
    state: Arc<PrivateChatState>,
) {
    let (mut sender, mut receiver) = ws.split();

    let (tx, mut rx) = broadcast::channel(100);

    {
        let mut connections = state.connections.write().await;
        connections.insert(sender_user.user_id.clone(), tx.clone());
    }

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    let state_clone = state.clone();
    let sender_clone = sender_user.clone();

    let mut recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            if let Ok(msg) = msg {
                match msg {
                    Message::Text(text) => {
                        send_to_user(&state_clone, &sender_clone, &receiver_user, text.as_str())
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
        connections.remove(&sender_user.user_id.clone());
    }
}

pub async fn send_to_user(
    state: &PrivateChatState,
    sender_user: &User,
    receiver_user: &User,
    msg: &str,
) {
    let connections = state.connections.read().await;

    if let Some(tx) = connections.get(&receiver_user.user_id) {
        let response = json_msg(sender_user, receiver_user, msg);

        let _ = tx.send(response);
    }
    if let Some(tx) = connections.get(&sender_user.user_id) {
        let response = json_msg(sender_user, receiver_user, msg);
        let _ = tx.send(response);
    }
}

fn json_msg(sender_user: &User, receiver_user: &User, msg: &str) -> String {
    let seconds = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let chat_message = ChatMessage {
        sender_user: sender_user.clone(),
        receiver_user: receiver_user.clone(),
        message: msg.to_string(),
        timestamp: seconds,
    };

    match serde_json::to_string(&chat_message) {
        Ok(json) => json,
        Err(e) => json!({
            "error": format!("Failed to serialize message: {}",e.to_string())})
        .to_string(),
    }
}
