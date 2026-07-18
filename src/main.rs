use std::{future, sync::Arc};

use anyhow::{Context, Result};
use tokio::{net::TcpListener, signal::unix, sync::Notify};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use crate::{config::Config, infra::packages::package_manager::PackageManager, state::AppState};

mod config;
mod constant;
mod db;
mod handler;
mod infra;
mod router;
mod state;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("vulpecula=info,tower_http=debug"))
        .context("failed to configure tracing subscriber")?;

    tracing_subscriber::fmt()
        .json()
        .with_env_filter(env_filter)
        .init();

    let config = Arc::new(Config::from_env()?);

    let db = db::pool::connect(&config.database_url)
        .await
        .context("Failed to connect to database")?;
    db::migrations::migrate(&db)
        .await
        .context("Failed to migrate database")?;

    let pm = PackageManager::detect()
        .await
        .context("failed to detect package manager")?;

    db::migrations::seed(&db, &pm)
        .await
        .context("Failed to seed database")?;

    let reconcile_notify = Arc::new(Notify::new());

    infra::packages::reconciler::start(&db, reconcile_notify.clone(), pm)
        .await
        .context("failed to start package reconciler")?;

    let addr = config.socket_addr();
    let state = AppState::new(config, db, reconcile_notify, pm);
    info!("Starting server on {}", addr);

    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind tcp listener on {addr}"))?;

    axum::serve(listener, router::routes(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server failed while handling requests")?;

    Ok(())
}

async fn shutdown_signal() {
    let terminate = async {
        match unix::signal(unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(e) => {
                error!(%e, "failed to install SIGTERM handler");
                future::pending::<()>().await;
            }
        }
    };

    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    tokio::select! {
        _ = ctrl_c => {
            info!("received Ctrl+C shutdown signal");
        }
        _ = terminate => {
            info!("received SIGTERM shutdown signal");
        }
    }
}
