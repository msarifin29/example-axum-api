use std::sync::Arc;

use axum::{
    Router, middleware,
    routing::{delete, get, post, put},
};

use crate::app_state::AppState;
use crate::{
    auth::{
        handler::{
            delete_user_handler, get_users_handler, login_handler, register_handler,
            update_password_handler,
        },
        middleware::auth_middleware,
    },
    group::handler::{create_group_handler, groups_handler},
    websocket::{chat::private_chat_handler, group::group_chat_handler, handler::ws_handler},
};

pub fn routes(state: Arc<AppState>) -> Router {
    let auth_route = Router::new()
        .route("/api/auth/register", post(register_handler))
        .route("/api/auth/login", post(login_handler));

    let auth_private_route = Router::new()
        .route("/api/auth/update-password", put(update_password_handler))
        .route("/api/auth/delete-account", delete(delete_user_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    let user_route = Router::new()
        .route("/api/users", get(get_users_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    let group_route = Router::new()
        .route("/api/groups", post(create_group_handler))
        .route("/api/groups", get(groups_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    let ws_route = Router::new()
        .route("/ws", get(ws_handler))
        .route("/chat", get(private_chat_handler))
        .route("/group-chat", get(group_chat_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    Router::new()
        .merge(auth_route)
        .merge(auth_private_route)
        .merge(user_route)
        .merge(group_route)
        .merge(ws_route)
        .with_state(state)
}
