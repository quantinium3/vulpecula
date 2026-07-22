use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DesiredState {
    Running,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ProxyStatus {
    Pending,
    Starting,
    Running,
    Failed,
    Stopping,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DnsProvider {
    Cloudflare,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ProxySettings {
    pub desired_state: DesiredState,
    pub status: ProxyStatus,
    pub dns_provider: Option<DnsProvider>,
    pub created_at: i64,
    pub updated_at: i64,
}

pub async fn fetch(pool: &SqlitePool) -> Result<ProxySettings> {
    sqlx::query_as!(
        ProxySettings,
        r#"select
            desired_state as "desired_state!: DesiredState",
            status as "status!: ProxyStatus",
            dns_provider as "dns_provider: DnsProvider",
            created_at as "created_at!",
            updated_at as "updated_at!"
        from proxy_settings
        where id = 1"#
    )
    .fetch_one(pool)
    .await
    .context("failed to fetch proxy settings")
}

pub async fn set_desired_state(pool: &SqlitePool, state: DesiredState) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "update proxy_settings set desired_state = ?, updated_at = unixepoch() where id = 1",
        state
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn set_status(pool: &SqlitePool, status: ProxyStatus) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "update proxy_settings set status = ?, updated_at = unixepoch() where id = 1",
        status
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn set_dns_provider(pool: &SqlitePool, provider: Option<DnsProvider>) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "update proxy_settings set dns_provider = ?, updated_at = unixepoch() where id = 1",
        provider
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct DnsCredential {
    pub credential_name: String,
    pub parameter_key: String,
}

pub async fn fetch_credentials(pool: &SqlitePool) -> Result<Vec<DnsCredential>> {
    sqlx::query_as!(
        DnsCredential,
        r#"select
            credential_name as "credential_name!",
            parameter_key as "parameter_key!"
        from proxy_dns_credentials
        order by credential_name"#
    )
    .fetch_all(pool)
    .await
    .context("failed to fetch proxy dns credentials")
}

pub async fn upsert_credential(pool: &SqlitePool, credential_name: &str, parameter_key: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "insert into proxy_dns_credentials (credential_name, parameter_key) values (?, ?)
        on conflict (credential_name) do update set parameter_key = excluded.parameter_key",
        credential_name,
        parameter_key
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_credential(pool: &SqlitePool, credential_name: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "delete from proxy_dns_credentials where credential_name = ?",
        credential_name
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}
