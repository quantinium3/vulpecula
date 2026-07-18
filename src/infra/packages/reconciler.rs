use anyhow::Result;
use sqlx::SqlitePool;

use crate::{
    db::queries::package::{
        self, DesiredState, PackageStatus, lookup_name_for_manager, transition, transition_failed,
    },
    infra::packages::package_manager::PackageManager,
};

pub async fn reconcile(pool: &SqlitePool, pm: &PackageManager) -> Result<()> {
    let packages = package::fetch_all(pool).await?;

    for pkg in packages {
        match (pkg.desired_state, pkg.status) {
            (DesiredState::Installed, PackageStatus::Removed | PackageStatus::Failed) => {
                transition(pool, &pkg, PackageStatus::Installing, None).await?;
                let manager_name = lookup_name_for_manager(pool, &pkg.id, pm.id()).await?;
                match pm.install(&manager_name).await {
                    Ok(()) => transition(pool, &pkg, PackageStatus::Installed, None).await?,
                    Err(e) => transition_failed(pool, &pkg, &e).await?,
                }
            }
            (DesiredState::Removed, PackageStatus::Installed | PackageStatus::Failed) => {
                transition(pool, &pkg, PackageStatus::Removing, None).await?;
                let manager_name = lookup_name_for_manager(pool, &pkg.id, pm.id()).await?;
                match pm.remove(&manager_name).await {
                    Ok(()) => transition(pool, &pkg, PackageStatus::Removed, None).await?,
                    Err(e) => transition_failed(pool, &pkg, &e).await?,
                }
            }
            _ => {} // already converged or mid-transition, leave alone
        }
    }
    Ok(())
}
