use anyhow::{Context, Result};
use sqlx::SqlitePool;

use crate::{
    constant::SEED_PACKAGES,
    db::queries::package::{DesiredState, PackageStatus},
    infra::packages::package_manager::PackageManager,
};

pub async fn migrate(pool: &SqlitePool) -> Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .context("Failed to run database migrations")?;
    Ok(())
}

pub async fn seed(pool: &SqlitePool, pm: &PackageManager) -> Result<()> {
    let mut tx = pool
        .begin()
        .await
        .context("failed to start seed transaction")?;

    for pkg in SEED_PACKAGES {
        let manager_name = match pm {
            PackageManager::APT => pkg.apt_name,
            PackageManager::RPM(_) => pkg.rpm_name,
        };

        let installed = match manager_name {
            Some(name) => pm.is_installed(name).await.unwrap_or(false),
            None => false,
        };

        let (status, desired_state) = if installed {
            (PackageStatus::Installed, DesiredState::Installed)
        } else {
            (PackageStatus::Removed, DesiredState::Removed)
        };

        sqlx::query!(
            "insert into vulpecula_packages (id, name, description, status, desired_state) values (?, ?, ?, ?, ?) on conflict (id) do nothing",
            pkg.id,
            pkg.name,
            pkg.description,
            status,
            desired_state
        ).execute(&mut *tx).await.with_context(|| format!("failed to seed package {}", pkg.id))?;

        for (manager, manager_name) in [("apt", pkg.apt_name), ("rpm", pkg.rpm_name)]
            .into_iter()
            .filter_map(|(m, n)| n.map(|n| (m, n)))
        {
            sqlx::query!(
                "insert into package_names (package_id, package_manager, name) values (?, ?, ?)
                            on conflict (package_id, package_manager) do nothing",
                pkg.id,
                manager,
                manager_name
            )
            .execute(&mut *tx)
            .await
            .with_context(|| format!("failed to seed package_names for {} on {manager}", pkg.id))?;
        }
    }

    tx.commit()
        .await
        .context("failed to commit seed transaction")?;
    Ok(())
}
