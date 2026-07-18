use anyhow::{Context, Result};
use serde::Serialize;
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DesiredState {
    Installed,
    Removed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
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

pub async fn fetch_all_for_manager(pool: &SqlitePool, manager: &str) -> Result<Vec<Package>> {
    Ok(sqlx::query_as!(
        Package,
        r#"select
            p.id as "id!",
            p.name as "name!",
            p.description,
            p.desired_state as "desired_state!: DesiredState",
            p.status as "status!: PackageStatus",
            p.created_at as "created_at!",
            p.updated_at as "updated_at!"
        from vulpecula_packages p
        where exists (
            select 1 from package_names pn
            where pn.package_id = p.id and pn.package_manager = ?
        )"#,
        manager
    )
    .fetch_all(pool)
    .await?)
}

pub async fn set_desired_state(pool: &SqlitePool, id: &str, state: DesiredState) -> Result<bool> {
    let result = sqlx::query!(
        "update vulpecula_packages set desired_state = ?, updated_at = unixepoch() where id = ?",
        state,
        id
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
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

pub async fn transition_failed(
    pool: &SqlitePool,
    pkg: &Package,
    err: &anyhow::Error,
) -> Result<()> {
    transition(pool, pkg, PackageStatus::Failed, Some(&err.to_string())).await
}

pub async fn lookup_name_for_manager(
    pool: &SqlitePool,
    package_id: &str,
    manager: &str,
) -> Result<Option<String>> {
    sqlx::query_scalar!(
        "select name from package_names where package_id = ? and package_manager = ?",
        package_id,
        manager
    )
    .fetch_optional(pool)
    .await
    .context("failed to look up package name mapping")
}
