use anyhow::{Context, Result};
use serde::Serialize;
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DesiredState {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum FirewallStatus {
    Pending,
    Applying,
    Enabled,
    Failed,
    Disabling,
    Disabled,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct FirewallSettings {
    pub desired_state: DesiredState,
    pub status: FirewallStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

pub async fn fetch(pool: &SqlitePool) -> Result<FirewallSettings> {
    sqlx::query_as!(
        FirewallSettings,
        r#"select
            desired_state as "desired_state!: DesiredState",
            status as "status!: FirewallStatus",
            created_at as "created_at!",
            updated_at as "updated_at!"
        from firewall_settings
        where id = 1"#
    )
    .fetch_one(pool)
    .await
    .context("failed to fetch firewall settings")
}

pub async fn set_desired_state(pool: &SqlitePool, state: DesiredState) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "update firewall_settings set desired_state = ?, updated_at = unixepoch() where id = 1",
        state
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn set_status(pool: &SqlitePool, status: FirewallStatus) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "update firewall_settings set status = ?, updated_at = unixepoch() where id = 1",
        status
    )
    .execute(pool)
    .await?;

    Ok(())
}
