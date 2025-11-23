use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{app_state::AppState, auth::jwt::verify_token};

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, Response> {
    // Extract token from Authorization header
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|auth_header| auth_header.to_str().ok())
        .and_then(|auth_value| {
            if auth_value.starts_with("Bearer ") {
                Some(auth_value[7..].to_string())
            } else {
                None
            }
        });

    // Return error if no token
    let token = token.ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            "Missing or invalid Authorization header",
        )
            .into_response()
    })?;

    // Veirify token
    let claims = verify_token(&state.jwt_config, &token)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid or expired token").into_response())?;

    // Add claims to request extensions
    req.extensions_mut().insert(claims);

    // Continue to handler
    Ok(next.run(req).await)
}
