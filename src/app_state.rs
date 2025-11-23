use std::sync::Arc;

use sqlx::{Pool, Postgres};

use crate::{
    auth::jwt::JwtConfig,
    websocket::{chat::PrivateChatState, group::GroupState},
};

#[derive(Clone)]
pub struct AppState {
    pub pool: Arc<Pool<Postgres>>,
    pub chat: Arc<PrivateChatState>,
    pub group: Arc<GroupState>,
    pub jwt_config: Arc<JwtConfig>,
}

impl AppState {
    pub fn new(pool: Pool<Postgres>, secret: String) -> Self {
        Self {
            pool: Arc::new(pool),
            chat: Arc::new(PrivateChatState::new()),
            group: Arc::new(GroupState::new()),
            jwt_config: Arc::new(JwtConfig::new(secret)),
        }
    }
}
