use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};

use crate::auth::jwt::Claims;

pub struct AuthUser(pub Claims);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Allows extracting authenticated user in handler parameters
        // Gets claims from request extensions (added by middleware)
        parts
            .extensions
            .get::<Claims>()
            .cloned()
            .map(AuthUser)
            .ok_or((StatusCode::UNAUTHORIZED, "Unauthorized"))
    }
}
