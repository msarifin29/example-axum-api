mod app_state;
mod auth;
mod config;
mod group;
mod routes;
mod websocket;

use std::sync::Arc;

use crate::auth::jwt::Secret;
use crate::{
    app_state::AppState,
    config::{connection::ConnectionBuilder, flavor::load_config},
    routes::routes,
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

    let app = routes(state);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", tcp.ip, tcp.port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
