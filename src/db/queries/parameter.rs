use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ParameterType {
    String,
    SecureString,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Parameter {
    pub key: String,
    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    pub type_: ParameterType,
    pub value: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

pub async fn fetch_all(pool: &SqlitePool) -> Result<Vec<Parameter>> {
    sqlx::query_as!(
        Parameter,
        r#"select
            key as "key!",
            type as "type_!: ParameterType",
            value,
            created_at as "created_at!",
            updated_at as "updated_at!"
        from parameters
        order by key"#
    )
    .fetch_all(pool)
    .await
    .context("failed to fetch parameters")
}

pub async fn fetch_one(pool: &SqlitePool, key: &str) -> Result<Option<Parameter>> {
    sqlx::query_as!(
        Parameter,
        r#"select
            key as "key!",
            type as "type_!: ParameterType",
            value,
            created_at as "created_at!",
            updated_at as "updated_at!"
        from parameters
        where key = ?"#,
        key
    )
    .fetch_optional(pool)
    .await
    .context("failed to fetch parameter")
}

pub async fn create_string(pool: &SqlitePool, key: &str, value: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "insert into parameters (key, type, value) values (?, 'string', ?)",
        key,
        value
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn create_secure_string(
    pool: &SqlitePool,
    key: &str,
    enc: &crate::infra::parameters::secrets::EncryptedValue,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "insert into parameters (key, type, ciphertext, nonce, wrapped_dek, dek_nonce)
        values (?, 'secure_string', ?, ?, ?, ?)",
        key,
        enc.ciphertext,
        enc.nonce,
        enc.wrapped_dek,
        enc.dek_nonce
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn fetch_encrypted(
    pool: &SqlitePool,
    key: &str,
) -> Result<Option<crate::infra::parameters::secrets::EncryptedValue>> {
    let row = sqlx::query!(
        r#"select
            ciphertext as "ciphertext!",
            nonce as "nonce!",
            wrapped_dek as "wrapped_dek!",
            dek_nonce as "dek_nonce!"
        from parameters
        where key = ? and type = 'secure_string'"#,
        key
    )
    .fetch_optional(pool)
    .await
    .context("failed to fetch encrypted parameter")?;

    Ok(
        row.map(|r| crate::infra::parameters::secrets::EncryptedValue {
            ciphertext: r.ciphertext,
            nonce: r.nonce,
            wrapped_dek: r.wrapped_dek,
            dek_nonce: r.dek_nonce,
        }),
    )
}

pub async fn update_string(pool: &SqlitePool, key: &str, value: &str) -> Result<bool> {
    let result = sqlx::query!(
        "update parameters set value = ?, updated_at = unixepoch() where key = ? and type = 'string'",
        value,
        key
    )
    .execute(pool)
    .await
    .context("failed to update parameter")?;

    Ok(result.rows_affected() > 0)
}

pub async fn update_secure_string(
    pool: &SqlitePool,
    key: &str,
    enc: &crate::infra::parameters::secrets::EncryptedValue,
) -> Result<bool> {
    let result = sqlx::query!(
        "update parameters
        set ciphertext = ?, nonce = ?, wrapped_dek = ?, dek_nonce = ?, updated_at = unixepoch()
        where key = ? and type = 'secure_string'",
        enc.ciphertext,
        enc.nonce,
        enc.wrapped_dek,
        enc.dek_nonce,
        key
    )
    .execute(pool)
    .await
    .context("failed to update secure parameter")?;

    Ok(result.rows_affected() > 0)
}

pub async fn delete(pool: &SqlitePool, key: &str) -> Result<bool> {
    let result = sqlx::query!("delete from parameters where key = ?", key)
        .execute(pool)
        .await
        .context("failed to delete parameter")?;

    Ok(result.rows_affected() > 0)
}
