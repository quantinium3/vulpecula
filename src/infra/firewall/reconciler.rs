use std::{sync::Arc, time::Duration};

use anyhow::Result;
use sqlx::SqlitePool;
use tokio::sync::Notify;

use crate::{
    config::Config,
    constant::{PROXY_HTTP_PORT, PROXY_HTTPS_PORT},
    db::queries::{
        firewall::{self, DesiredState, FirewallStatus},
        project,
        proxy::{self, DesiredState as ProxyDesiredState},
    },
    infra::firewall::backend::{AllowedPort, FirewallBackend, Protocol},
};

const RESYNC_INTERVAL: Duration = Duration::from_secs(30);

pub async fn start(pool: &SqlitePool, notify: Arc<Notify>, config: Arc<Config>) -> Result<()> {
    let backend = match FirewallBackend::detect().await {
        Ok(backend) => backend,
        Err(e) => {
            tracing::warn!(error = ?e, "no firewall backend available, firewall management disabled for this run");
            return Ok(());
        }
    };

    if let Err(e) = reconcile(pool, backend, &config).await {
        tracing::error!(error = ?e, "initial firewall reconcile failed");
    }

    let pool = pool.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(RESYNC_INTERVAL);
        loop {
            tokio::select! {
                _ = notify.notified() => {}
                _ = ticker.tick() => {}
            }
            if let Err(e) = reconcile(&pool, backend, &config).await {
                tracing::error!(error = ?e, "firewall reconcile failed");
            }
        }
    });

    Ok(())
}

async fn reconcile(pool: &SqlitePool, backend: FirewallBackend, config: &Config) -> Result<()> {
    let settings = firewall::fetch(pool).await?;

    match settings.desired_state {
        DesiredState::Disabled => {
            firewall::set_status(pool, FirewallStatus::Disabling).await?;
            if let Err(e) = backend.teardown().await {
                firewall::set_status(pool, FirewallStatus::Failed).await?;
                return Err(e);
            }
            firewall::set_status(pool, FirewallStatus::Disabled).await?;
            Ok(())
        }
        DesiredState::Enabled => {
            firewall::set_status(pool, FirewallStatus::Applying).await?;

            let allowed = compute_allowed_ports(pool, config).await?;

            if let Err(e) = backend.apply(&allowed).await {
                firewall::set_status(pool, FirewallStatus::Failed).await?;
                return Err(e);
            }

            firewall::set_status(pool, FirewallStatus::Enabled).await?;
            Ok(())
        }
    }
}

async fn compute_allowed_ports(pool: &SqlitePool, config: &Config) -> Result<Vec<AllowedPort>> {
    let mut allowed = vec![
        AllowedPort {
            port: 22,
            protocol: Protocol::Tcp,
        },
        AllowedPort {
            port: config.port,
            protocol: Protocol::Tcp,
        },
    ];

    for p in project::fetch_all(pool).await? {
        if let Some(port) = p.port {
            allowed.push(AllowedPort {
                port: port as u16,
                protocol: Protocol::Tcp,
            });
        }
    }

    let proxy_settings = proxy::fetch(pool).await?;
    if proxy_settings.desired_state == ProxyDesiredState::Running {
        allowed.push(AllowedPort {
            port: PROXY_HTTP_PORT,
            protocol: Protocol::Tcp,
        });
        allowed.push(AllowedPort {
            port: PROXY_HTTPS_PORT,
            protocol: Protocol::Tcp,
        });
    }

    Ok(allowed)
}
