use std::sync::Arc;

use crate::auth::extractors::AuthUser;
use crate::group::handler::{Group, get_by_id};
use crate::{AppState, auth::user::User, websocket::handler::validate_user};
use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{
        StatusCode,
        header::{AUTHORIZATION, HeaderMap, HeaderName, HeaderValue},
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::broadcast;

#[derive(Debug, Serialize, Deserialize)]
pub struct GroupMessage {
    pub id: String,
    pub name: String,
    pub message: String,
}

pub struct GroupState {
    pub tx: broadcast::Sender<String>,
}

impl GroupState {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(100);
        Self { tx }
    }
}

pub async fn group_chat_handler(
    ws: WebSocketUpgrade,
    AuthUser(user): AuthUser,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let group_id = match headers.get("group_id") {
        Some(v) => match v.to_str() {
            Ok(id) => id.to_string(),
            Err(_) => {
                return (StatusCode::BAD_REQUEST, "Invalid group_id header").into_response();
            }
        },
        None => {
            return (StatusCode::BAD_REQUEST, "Missing group_id header").into_response();
        }
    };

    let user_id_exists = validate_user(&user.user_id, &state.pool).await;
    let group_id_exists = get_by_id(&state.pool, &group_id).await;

    let mut response_header = HeaderMap::new();

    let token = format!("Bearer {}", user.user_id);
    let header_token = HeaderValue::from_str(&token).expect("Invalid header value");
    response_header.insert(AUTHORIZATION, header_token);

    let header_group = HeaderValue::from_str(&group_id).expect("Invalid header group");
    response_header.insert(HeaderName::from_static("group_id"), header_group);
    match (user_id_exists, group_id_exists) {
        (Some(user), Some(group)) => (
            response_header.clone(),
            ws.on_upgrade(move |socket| group_chat(socket, user, group, state.group.clone())),
        )
            .into_response(),
        _ => {
            let mut resp = (StatusCode::BAD_REQUEST, "Invalid group_id or user_id").into_response();
            for (k, v) in response_header.iter() {
                resp.headers_mut().append(k, v.clone());
            }

            resp
        }
    }
}

pub async fn group_chat(ws: WebSocket, user: User, group: Group, state: Arc<GroupState>) {
    let (mut sender, mut receiver) = ws.split();

    let mut rx = state.tx.subscribe();
    let msg = format!(
        "Welcome {} to {}",
        user.user_name.clone(),
        group.name.clone(),
    );
    let group_msg = GroupMessage {
        id: group.group_id,
        name: group.name,
        message: msg.to_string(),
    };
    let response = serde_msg(&group_msg);
    let _ = state.tx.clone().send(response);

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    let state_clone = state.tx.clone();

    let mut recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            if let Ok(msg) = msg {
                match msg {
                    Message::Text(text) => {
                        let group_msg = GroupMessage {
                            id: user.user_id.clone(),
                            name: user.user_name.clone(),
                            message: text.to_string(),
                        };
                        let response = serde_msg(&group_msg);
                        let _ = state_clone.send(response);
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
}

pub fn serde_msg(group_msg: &GroupMessage) -> String {
    let response = match serde_json::to_string(&group_msg) {
        Ok(json) => json,
        Err(e) => json!({
            "error": format!("Failed to serialize message: {}",e.to_string()),
            "message": group_msg.message.to_string(),
        })
        .to_string(),
    };
    response
}
