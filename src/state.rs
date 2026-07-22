use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::Notify;

use crate::{
    config::Config,
    infra::{
        containers::docker::DockerClient, packages::package_manager::PackageManager,
        parameters::secrets::MasterKey, proxy::docker::ProxyDockerClient,
    },
};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: SqlitePool,
    pub reconcile_notify: Arc<Notify>,
    pub container_reconcile_notify: Arc<Notify>,
    pub proxy_reconcile_notify: Arc<Notify>,
    pub firewall_reconcile_notify: Arc<Notify>,
    pub package_manager: PackageManager,
    pub master_key: Arc<MasterKey>,
    pub docker: DockerClient,
    pub proxy_docker: ProxyDockerClient,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: Arc<Config>,
        db: SqlitePool,
        reconcile_notify: Arc<Notify>,
        container_reconcile_notify: Arc<Notify>,
        proxy_reconcile_notify: Arc<Notify>,
        firewall_reconcile_notify: Arc<Notify>,
        package_manager: PackageManager,
        master_key: Arc<MasterKey>,
        docker: DockerClient,
        proxy_docker: ProxyDockerClient,
    ) -> Self {
        Self {
            config,
            db,
            reconcile_notify,
            container_reconcile_notify,
            proxy_reconcile_notify,
            firewall_reconcile_notify,
            package_manager,
            master_key,
            docker,
            proxy_docker,
        }
    }
}
