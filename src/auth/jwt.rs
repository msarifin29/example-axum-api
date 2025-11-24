use crate::config::connection::Configure;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct Secret;
impl Secret {
    pub fn new(env: &str) -> String {
        let configure = Configure::build(env).expect("Failed to load environment");
        let secret_key = configure
            .get_string("jwt.key")
            .expect("Failed to get jwt secret key");
        secret_key
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String, // Subject (user_id)
    pub exp: usize,
    pub iat: usize, // Issued at (unnix timestamp)
    pub user_id: String,
    pub email: String,
}

#[derive(Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub access_token_expiry: usize,
    pub refresh_token_expiry: usize,
}

impl JwtConfig {
    pub fn new(secret: String) -> Self {
        Self {
            secret,
            access_token_expiry: 3600,    // 1 hour
            refresh_token_expiry: 604800, // 7 days
        }
    }
}

pub fn create_access_token(
    config: &JwtConfig,
    user_id: &str,
    email: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        exp: now + config.access_token_expiry,
        iat: now,
        user_id: user_id.to_string(),
        email: email.to_string(),
    };

    encode(
        &Header::default(), // Use default algoritme (HS256)
        &claims,            // Token payload
        &EncodingKey::from_secret(config.secret.as_bytes()), // Secret key
    )
}

pub fn create_refresh_token(
    config: &JwtConfig,
    user_id: &str,
    email: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        exp: now + config.refresh_token_expiry,
        iat: now,
        user_id: user_id.to_string(),
        email: email.to_string(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.secret.as_bytes()),
    )
}

pub fn verify_token(
    config: &JwtConfig,
    token: &str,
) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,                                               // Token string to verify
        &DecodingKey::from_secret(config.secret.as_bytes()), // Secret key
        &Validation::new(Algorithm::HS256),                  // Validation settings
    )?;

    Ok(token_data.claims)
}
