use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::Notify;

use crate::{
    config::Config,
    infra::{packages::package_manager::PackageManager, parameters::secrets::MasterKey},
};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: SqlitePool,
    pub reconcile_notify: Arc<Notify>,
    pub package_manager: PackageManager,
    pub master_key: Arc<MasterKey>,
}

impl AppState {
    pub fn new(
        config: Arc<Config>,
        db: SqlitePool,
        reconcile_notify: Arc<Notify>,
        package_manager: PackageManager,
        master_key: Arc<MasterKey>,
    ) -> Self {
        Self {
            config,
            db,
            reconcile_notify,
            package_manager,
            master_key,
        }
    }
}
