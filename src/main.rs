mod app_state;
mod auth;
mod config;
mod group;
mod websocket;

use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, post, put},
};

use crate::{
    app_state::AppState,
    config::{connection::ConnectionBuilder, flavor::load_config},
    websocket::{chat::private_chat_handler, group::group_chat_handler, handler::ws_handler},
};
use crate::{
    auth::jwt::Secret,
    group::handler::{create_group_handler, groups_handler},
};
use auth::handler::{
    delete_user_handler, get_users_handler, login_handler, register_handler,
    update_password_handler,
};

#[tokio::main]
async fn main() {
    let flavor = load_config().expect("Failed to load configuration");
    let builder = ConnectionBuilder(flavor.clone());
    let pool = ConnectionBuilder::new(&builder)
        .await
        .expect("Failed to connect to database");
    let tcp = ConnectionBuilder::listen_on(&builder).expect("Failed to execute environment");

    let secret_key = Secret::new(&flavor);
    let state = Arc::new(AppState::new(pool, secret_key));

    let auth_route = Router::new()
        .route("/api/auth/register", post(register_handler))
        .route("/api/auth/login", post(login_handler));

    let user_route = Router::new()
        .route("/api/users", get(get_users_handler))
        .route("/api/users", put(update_password_handler))
        .route("/api/users/{user_id}", delete(delete_user_handler));

    let group_route = Router::new()
        .route("/api/groups", post(create_group_handler))
        .route("/api/groups", get(groups_handler));

    let ws_route = Router::new()
        .route("/ws", get(ws_handler))
        .route("/ws/chat", get(private_chat_handler))
        .route("/ws/group", get(group_chat_handler));

    let app = Router::new()
        .merge(auth_route)
        .merge(user_route)
        .merge(ws_route)
        .merge(group_route)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", tcp.ip, tcp.port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
