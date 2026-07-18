use std::sync::Arc;

use sqlx::SqlitePool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: SqlitePool,
}

impl AppState {
    pub fn new(config: Arc<Config>, db: SqlitePool) -> Self {
        Self { config, db }
    }
}
