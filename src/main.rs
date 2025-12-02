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

use axum::http::{HeaderValue, Method, header};
use tower_http::cors::CorsLayer;

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

    let cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::ACCEPT])
        .allow_credentials(true);

    let app = routes(state).layer(cors);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", tcp.ip, tcp.port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
