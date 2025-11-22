use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{AppState, auth::user::User, websocket::handler::validate_user};
use axum::{
    extract::{
        Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::{IntoResponse, Response},
};
use futures::{SinkExt, StreamExt};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::broadcast;

#[derive(Debug, Serialize, Deserialize)]
pub struct GroupMessage {
    pub user_name: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct GroupQuery {
    pub user_id: String,
    pub group_id: String,
}

pub struct GroupState {
    pub tx: broadcast::Sender<String>,
    pub users: Mutex<HashMap<String, String>>,
}

impl GroupState {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(100);
        Self {
            tx,
            users: Mutex::new(HashMap::new()),
        }
    }
}

pub async fn group_chat_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<GroupQuery>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let user_id_exists = validate_user(&query.user_id, &state.pool).await;

    match user_id_exists {
        Some(user) => ws.on_upgrade(move |socket| {
            group_chat(socket, user, query.group_id, state.group.clone())
        }),
        _ => Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body("Unauthorized: Invalid user_id".into())
            .unwrap(),
    }
}

pub async fn group_chat(ws: WebSocket, user: User, group_id: String, state: Arc<GroupState>) {
    let (mut sender, mut receiver) = ws.split();

    let mut rx = state.tx.subscribe();

    let welcome_message = format!(
        r#"Welcome {} to Group {}"#,
        user.user_name.clone(),
        group_id.clone(),
    );
    let _ = state.tx.clone().send(welcome_message);

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
                            user_name: user.user_name.clone(),
                            message: text.to_string(),
                        };
                        let response = match serde_json::to_string(&group_msg) {
                            Ok(json) => json,
                            Err(e) => json!({
                                "error": format!("Failed to serialize message: {}",e.to_string()),
                                "message": text.to_string(),
                            })
                            .to_string(),
                        };
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
