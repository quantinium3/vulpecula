use anyhow::{Context, Result};
use serde::Serialize;
use sqlx::{FromRow, SqliteExecutor, SqlitePool};

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DesiredState {
    Active,
    Removed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RouteStatus {
    Pending,
    Synced,
    Failed,
    Removed,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Route {
    pub id: String,
    pub project_id: String,
    pub domain: String,
    pub path_prefix: String,
    pub desired_state: DesiredState,
    pub status: RouteStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct NewRoute<'a> {
    pub id: &'a str,
    pub project_id: &'a str,
    pub domain: &'a str,
    pub path_prefix: &'a str,
}

pub async fn create<'a>(executor: impl SqliteExecutor<'a>, r: NewRoute<'_>) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "insert into routes (id, project_id, domain, path_prefix) values (?, ?, ?, ?)",
        r.id,
        r.project_id,
        r.domain,
        r.path_prefix
    )
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn fetch_all(pool: &SqlitePool) -> Result<Vec<Route>> {
    sqlx::query_as!(
        Route,
        r#"select
            id as "id!",
            project_id as "project_id!",
            domain as "domain!",
            path_prefix as "path_prefix!",
            desired_state as "desired_state!: DesiredState",
            status as "status!: RouteStatus",
            created_at as "created_at!",
            updated_at as "updated_at!"
        from routes
        order by domain, path_prefix"#
    )
    .fetch_all(pool)
    .await
    .context("failed to fetch routes")
}

pub async fn fetch_one(pool: &SqlitePool, id: &str) -> Result<Option<Route>, sqlx::Error> {
    sqlx::query_as!(
        Route,
        r#"select
            id as "id!",
            project_id as "project_id!",
            domain as "domain!",
            path_prefix as "path_prefix!",
            desired_state as "desired_state!: DesiredState",
            status as "status!: RouteStatus",
            created_at as "created_at!",
            updated_at as "updated_at!"
        from routes
        where id = ?"#,
        id
    )
    .fetch_optional(pool)
    .await
}

pub async fn fetch_for_project(pool: &SqlitePool, project_id: &str) -> Result<Vec<Route>, sqlx::Error> {
    sqlx::query_as!(
        Route,
        r#"select
            id as "id!",
            project_id as "project_id!",
            domain as "domain!",
            path_prefix as "path_prefix!",
            desired_state as "desired_state!: DesiredState",
            status as "status!: RouteStatus",
            created_at as "created_at!",
            updated_at as "updated_at!"
        from routes
        where project_id = ?
        order by domain, path_prefix"#,
        project_id
    )
    .fetch_all(pool)
    .await
}

pub async fn set_desired_state(pool: &SqlitePool, id: &str, state: DesiredState) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "update routes set desired_state = ?, updated_at = unixepoch() where id = ?",
        state,
        id
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn set_status(pool: &SqlitePool, id: &str, status: RouteStatus) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "update routes set status = ?, updated_at = unixepoch() where id = ?",
        status,
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!("delete from routes where id = ?", id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}
