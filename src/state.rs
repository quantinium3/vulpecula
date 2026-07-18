use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::Notify;

use crate::{config::Config, infra::packages::package_manager::PackageManager};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: SqlitePool,
    /// Pinged whenever a handler changes a package's desired state, so the
    /// reconciler loop wakes up immediately instead of waiting for its next
    /// periodic tick.
    pub reconcile_notify: Arc<Notify>,
    pub package_manager: PackageManager,
}

impl AppState {
    pub fn new(
        config: Arc<Config>,
        db: SqlitePool,
        reconcile_notify: Arc<Notify>,
        package_manager: PackageManager,
    ) -> Self {
        Self {
            config,
            db,
            reconcile_notify,
            package_manager,
        }
    }
}
