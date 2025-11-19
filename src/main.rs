mod auth;
mod config;

use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, post, put},
};

use crate::config::connection::ConnectionBuilder;
use auth::handler::{
    add_user_handler, delete_user_handler, get_users_handler, update_password_handler,
};
use sqlx::{Pool, Postgres};

#[tokio::main]
async fn main() {
    let builder = ConnectionBuilder(String::from("dev.toml"));
    let pool = ConnectionBuilder::new(&builder)
        .await
        .expect("Failed to connect to database");
    let tcp = ConnectionBuilder::listen_on(&builder).expect("Failed to execute environment");

    let db_state: Arc<Pool<Postgres>> = Arc::new(pool);

    let user_route = Router::new()
        .route("/api/users", post(add_user_handler))
        .route("/api/users", get(get_users_handler))
        .route("/api/users", put(update_password_handler))
        .route("/api/users/{user_id}", delete(delete_user_handler));

    let app = Router::new().merge(user_route).with_state(db_state);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", tcp.ip, tcp.port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
