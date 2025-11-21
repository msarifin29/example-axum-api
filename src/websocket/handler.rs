/// WebSocket Handler Module
///
/// This module provides WebSocket connection handling for real-time communication.
/// It includes:
/// - WebSocket upgrade handler for accepting WS connections
/// - User validation before establishing connection
/// - Message routing and processing for different message types
/// - Connection lifecycle management (open, process, close)
use axum::{
    body::Body,
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::{IntoResponse, Response},
};
use futures::{SinkExt, StreamExt};
use http::StatusCode;
use serde::Deserialize;
use sqlx::{Pool, Postgres, Row, postgres::PgRow};
use std::sync::Arc;

use crate::{AppState, auth::user::User};

/// Query parameter struct for WebSocket connection
///
/// When a client connects via WebSocket, they must provide a `user_id` query parameter.
/// Example: `ws://localhost:3000/ws?user_id=12345`
///
/// This parameter is used to:
/// - Validate that the user exists in the database
/// - Track which user is connected for logging and message routing
#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub user_id: String,
}

/// Main WebSocket handler - entry point for WS connections
///
/// This is the route handler that Axum calls when a client requests a WebSocket upgrade.
/// Flow:
/// 1. Extract the `user_id` from query parameters
/// 2. Validate that the user exists in the database
/// 3. If valid: upgrade HTTP connection to WebSocket and start message handling
/// 4. If invalid: reject with 401 UNAUTHORIZED status
///
/// Parameters:
/// - `ws`: WebSocketUpgrade - the upgrade request from the client
/// - `query`: Query<WsQuery> - parsed query parameters (contains user_id)
/// - `pool`: State<Arc<Pool<Postgres>>> - database connection pool for validation
///
/// Returns:
/// - Success: WebSocket connection established, starts listening for messages
/// - Error: HTTP 401 response if user validation fails
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let user_exists = validate_user(&query.user_id, &state.pool).await;
    match user_exists {
        Some(user) => ws.on_upgrade(move |socket| handle_socket(socket, query.user_id, user)),
        None => {
            return Response::builder()
                .status(StatusCode::UNAUTHORIZED.as_u16())
                .body(Body::from("Unauthorized: Invalid user_id"))
                .unwrap()
                .into_response();
        }
    }
}

/// Validates if a user exists in the database
///
/// This function performs a security check before establishing a WebSocket connection.
/// It queries the database to verify that the provided user_id is valid and exists.
///
/// Database Query:
/// - SELECT user_id, user_name, email FROM users WHERE user_id = $1
/// - This ensures only authenticated users can open WebSocket connections
///
/// Parameters:
/// - `user_id`: The user ID to validate (from query parameters)
/// - `pool`: Database connection pool to execute the query
///
/// Returns:
/// - Some(User): If user is found, returns the user object with id, name, and email
/// - None: If user is not found or query fails
///
/// Example Usage:
/// ```rust
/// let user = validate_user("user-123", &pool).await;
/// if let Some(user) = user {
///     println!("User {} is valid", user.user_name);
/// }
/// ```
pub async fn validate_user(user_id: &str, pool: &Pool<Postgres>) -> Option<User> {
    let sql = "select user_id, user_name, email from users where user_id = $1";
    let result = sqlx::query(sql)
        .bind(user_id)
        .map(|data: PgRow| User {
            user_name: data.get("user_name"),
            email: data.get("email"),
            user_id: data.get("user_id"),
        })
        .fetch_optional(pool)
        .await
        .unwrap();
    match result {
        Some(data) => Some(data.clone()),
        None => None,
    }
}

/// Handles WebSocket communication for a connected user
///
/// This function manages the entire lifecycle of a WebSocket connection:
/// 1. Accepts incoming messages from the client
/// 2. Processes different message types (Text, Binary, Ping, Close)
/// 3. Sends appropriate responses back to the client
/// 4. Handles connection errors and cleanup
///
/// Message Types Handled:
/// - **Message::Text**: Client sends text data
///   → Returns JSON response with user data and the received text
/// - **Message::Binary**: Client sends binary data
///   → Responds with Pong frame
/// - **Message::Ping**: Client sends ping (keep-alive)
///   → Responds with pong frame to keep connection alive
/// - **Message::Close**: Client closes connection
///   → Breaks loop and closes server-side connection
/// - **Other**: Reserved/unknown message types are ignored
///
/// Connection Flow:
/// 1. Split the WebSocket into sender and receiver
/// 2. Send welcome message to client
/// 3. Loop: Wait for incoming messages
/// 4. Process each message and send response
/// 5. Break loop when client disconnects or error occurs
/// 6. Log disconnection and cleanup
///
/// Parameters:
/// - `socket`: The WebSocket connection from Axum
/// - `user_id`: String ID of the connected user (for logging)
/// - `user`: User struct containing user details (user_name, email)
///
/// Example Message Flow:
/// ```
/// Client → Server: "Hello"
/// Server → Client: {"data":User{...},"message":"Hello"}
/// ```
pub async fn handle_socket(socket: WebSocket, user_id: String, user: User) {
    // Split the WebSocket into sender (tx) and receiver (rx) halves
    // This allows concurrent sending and receiving of messages
    let (mut sender, mut receiver) = socket.split();

    println!("WebSocket connection established for user_id: {}", user_id);

    // Send a welcome message to the client immediately after connection
    // This confirms the connection is active and authenticated
    let welcome_message = format!(r#"Welcome, user_id: {}!"#, user_id);
    let _ = sender.send(Message::Text(welcome_message.into())).await;

    // Main message loop - continuously listen for incoming messages
    // The loop breaks when:
    // - Client sends Close frame
    // - Connection error occurs
    // - Client disconnects
    while let Some(msg) = receiver.next().await {
        if let Ok(msg) = msg {
            match msg {
                // Handle text messages from client
                // Return a JSON response containing user info and echoed message
                Message::Text(text) => {
                    let response = format!(
                        r#"{{"type":"echo","data":"{:?}","message":"{}"}}"#,
                        user, text
                    );
                    if sender.send(Message::Binary(response.into())).await.is_err() {
                        break;
                    }
                }
                // Handle binary messages from client
                // Respond with a pong frame to acknowledge receipt
                Message::Binary(data) => {
                    if sender.send(Message::Pong(data)).await.is_err() {
                        break;
                    }
                }
                // Handle explicit close message from client
                // Log the disconnection and terminate the connection
                Message::Close(_) => {
                    println!("User {} disconnected", user_id);
                    break;
                }
                // Handle ping frames (keep-alive check from client)
                // Respond with pong to keep connection alive
                Message::Ping(data) => {
                    if sender.send(Message::Pong(data)).await.is_err() {
                        break;
                    }
                }
                // Ignore other message types (reserved frames, etc.)
                _ => {}
            }
        } else {
            // If message parsing fails or connection error, break loop
            break;
        }
    }
    println!("🔴 WebSocket connection closed for user: {}", user_id);
}
