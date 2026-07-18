use anyhow::{Context, Result};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum DesiredState {
    Installed,
    Removed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum PackageStatus {
    Pending,
    Installing,
    Installed,
    Failed,
    Removing,
    Removed,
}

#[derive(Debug, Clone, FromRow)]
pub struct Package {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub desired_state: DesiredState,
    pub status: PackageStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

pub async fn fetch_all(pool: &SqlitePool) -> Result<Vec<Package>> {
    Ok(sqlx::query_as!(
        Package,
        r#"select
            id as "id!",
            name as "name!",
            description,
            desired_state as "desired_state!: DesiredState",
            status as "status!: PackageStatus",
            created_at as "created_at!",
            updated_at as "updated_at!"
        from vulpecula_packages"#
    )
    .fetch_all(pool)
    .await?)
}

pub async fn set_desired_state(pool: &SqlitePool, id: &str, state: DesiredState) -> Result<()> {
    sqlx::query!(
        "update vulpecula_packages set desired_state = ?, updated_at = unixepoch() where id = ?",
        state,
        id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn transition(
    pool: &SqlitePool,
    pkg: &Package,
    to: PackageStatus,
    reason: Option<&str>,
) -> Result<()> {
    let mut tx = pool.begin().await?;

    sqlx::query!(
        "update vulpecula_packages set status = ?, updated_at = unixepoch() where id = ?",
        to,
        pkg.id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "insert into package_state_transitions (package_id, from_status, to_status, reason) values (?, ?, ?, ?)",
        pkg.id,
        pkg.status,
        to,
        reason
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn transition_failed(pool: &SqlitePool, pkg: &Package, err: &anyhow::Error) -> Result<()> {
    transition(pool, pkg, PackageStatus::Failed, Some(&err.to_string())).await
}

pub async fn lookup_name_for_manager(
    pool: &SqlitePool,
    package_id: &str,
    manager: &str,
) -> Result<String> {
    sqlx::query_scalar!(
        "select name from package_names where package_id = ? and package_manager = ?",
        package_id,
        manager
    )
    .fetch_one(pool)
    .await
    .with_context(|| format!("no package name mapping for {package_id} on {manager}"))
}
