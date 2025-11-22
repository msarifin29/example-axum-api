use std::sync::Arc;

use sqlx::{Pool, Postgres};

use crate::websocket::{chat::PrivateChatState, group::GroupState};

#[derive(Clone)]
pub struct AppState {
    pub pool: Arc<Pool<Postgres>>,
    pub chat: Arc<PrivateChatState>,
    pub group: Arc<GroupState>,
}

impl AppState {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self {
            pool: Arc::new(pool),
            chat: Arc::new(PrivateChatState::new()),
            group: Arc::new(GroupState::new()),
        }
    }
}
