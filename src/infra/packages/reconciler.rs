use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use sqlx::SqlitePool;
use tokio::sync::Notify;

use crate::{
    db::queries::package::{
        self, DesiredState, Package, PackageStatus, lookup_name_for_manager, transition,
        transition_failed,
    },
    infra::packages::package_manager::PackageManager,
};

const RESYNC_INTERVAL: Duration = Duration::from_secs(30 * 60);

pub async fn start(pool: &SqlitePool, notify: Arc<Notify>, pm: PackageManager) -> Result<()> {
    reconcile(pool, &pm)
        .await
        .context("failed initial package reconcile")?;

    let pool = pool.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(RESYNC_INTERVAL);
        loop {
            tokio::select! {
                _ = notify.notified() => {}
                _ = ticker.tick() => {}
            }
            if let Err(e) = reconcile(&pool, &pm).await {
                tracing::error!(error = ?e, "package reconcile failed");
            }
        }
    });

    Ok(())
}

async fn reconcile(pool: &SqlitePool, pm: &PackageManager) -> Result<()> {
    let packages = package::fetch_all(pool).await?;

    for pkg in packages {
        let Some(manager_name) = lookup_name_for_manager(pool, &pkg.id, pm.id()).await? else {
            continue;
        };

        let mut pkg = match sync_reality(pool, pm, &pkg, &manager_name).await {
            Ok(pkg) => pkg,
            Err(e) => {
                tracing::error!(package = %pkg.id, error = ?e, "failed to sync package reality");
                continue;
            }
        };

        match (pkg.desired_state, pkg.status) {
            (DesiredState::Installed, PackageStatus::Removed | PackageStatus::Failed) => {
                transition(pool, &pkg, PackageStatus::Installing, None).await?;
                pkg.status = PackageStatus::Installing;
                match pm.install(&manager_name).await {
                    Ok(()) => transition(pool, &pkg, PackageStatus::Installed, None).await?,
                    Err(e) => transition_failed(pool, &pkg, &e).await?,
                }
            }
            (DesiredState::Removed, PackageStatus::Installed | PackageStatus::Failed) => {
                transition(pool, &pkg, PackageStatus::Removing, None).await?;
                pkg.status = PackageStatus::Removing;
                match pm.remove(&manager_name).await {
                    Ok(()) => transition(pool, &pkg, PackageStatus::Removed, None).await?,
                    Err(e) => transition_failed(pool, &pkg, &e).await?,
                }
            }
            _ => {}
        }
    }
    Ok(())
}

async fn sync_reality(
    pool: &SqlitePool,
    pm: &PackageManager,
    pkg: &Package,
    manager_name: &str,
) -> Result<Package> {
    if !matches!(pkg.status, PackageStatus::Installed | PackageStatus::Removed) {
        return Ok(pkg.clone());
    }

    let actual_status = if pm.is_installed(manager_name).await? {
        PackageStatus::Installed
    } else {
        PackageStatus::Removed
    };

    if actual_status == pkg.status {
        return Ok(pkg.clone());
    }

    transition(
        pool,
        pkg,
        actual_status,
        Some("reconciled with live system state"),
    )
    .await?;

    Ok(Package {
        status: actual_status,
        ..pkg.clone()
    })
}
