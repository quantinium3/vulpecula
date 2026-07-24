use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum BuildStatus {
    Success,
    Failed,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Build {
    pub id: String,
    pub project_id: String,
    pub number: i64,
    pub status: BuildStatus,
    pub tag: Option<String>,
    pub commit_sha: Option<String>,
    pub error: Option<String>,
    pub log: String,
    pub started_at: i64,
    pub finished_at: i64,
    pub created_at: i64,
}

pub struct NewBuild<'a> {
    pub project_id: &'a str,
    pub status: BuildStatus,
    pub tag: Option<&'a str>,
    pub commit_sha: Option<&'a str>,
    pub error: Option<&'a str>,
    pub log: &'a str,
    pub started_at: i64,
    pub finished_at: i64,
}

pub async fn create(pool: &SqlitePool, b: NewBuild<'_>) -> Result<Build> {
    let mut tx = pool.begin().await.context("failed to begin build tx")?;

    let number = sqlx::query_scalar!(
        r#"select coalesce(max(number), 0) + 1 as "next!: i64" from builds where project_id = ?"#,
        b.project_id
    )
    .fetch_one(&mut *tx)
    .await
    .context("failed to compute build number")?;

    let id = uuid::Uuid::now_v7().to_string();

    let build = sqlx::query_as!(
        Build,
        r#"insert into builds
            (id, project_id, number, status, sha, tag, error, log, started_at, finished_at)
        values (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        returning
            id as "id!",
            project_id as "project_id!",
            number as "number!",
            status as "status!: BuildStatus",
            tag,
            sha as "commit_sha",
            error,
            log as "log!",
            started_at as "started_at!",
            finished_at as "finished_at!",
            created_at as "created_at!""#,
        id,
        b.project_id,
        number,
        b.status,
        b.commit_sha,
        b.tag,
        b.error,
        b.log,
        b.started_at,
        b.finished_at,
    )
    .fetch_one(&mut *tx)
    .await
    .context("failed to insert build")?;

    tx.commit().await.context("failed to commit build")?;
    Ok(build)
}

pub async fn list(pool: &SqlitePool, project_id: &str) -> Result<Vec<Build>> {
    sqlx::query_as!(
        Build,
        r#"select
            id as "id!",
            project_id as "project_id!",
            number as "number!",
            status as "status!: BuildStatus",
            tag,
            sha as "commit_sha",
            error,
            log as "log!",
            started_at as "started_at!",
            finished_at as "finished_at!",
            created_at as "created_at!"
        from builds
        where project_id = ?
        order by number desc"#,
        project_id
    )
    .fetch_all(pool)
    .await
    .context("failed to fetch builds")
}

pub async fn get(pool: &SqlitePool, project_id: &str, id: &str) -> Result<Option<Build>> {
    sqlx::query_as!(
        Build,
        r#"select
            id as "id!",
            project_id as "project_id!",
            number as "number!",
            status as "status!: BuildStatus",
            tag,
            sha as "commit_sha",
            error,
            log as "log!",
            started_at as "started_at!",
            finished_at as "finished_at!",
            created_at as "created_at!"
        from builds
        where id = ? and project_id = ?"#,
        id,
        project_id
    )
    .fetch_optional(pool)
    .await
    .context("failed to fetch build")
}
