use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqliteExecutor, SqlitePool};

use super::project::EnvEntry;

#[derive(Debug, Serialize, Deserialize)]
pub struct PortMapping {
    pub host_port: Option<u16>,
    pub container_port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ContainerSpec {
    DockerImage {
        image: String,
        port: Option<PortMapping>,
        env: Vec<EnvEntry>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DesiredState {
    Running,
    Stopped,
    Removed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ContainerStatus {
    Pending,
    Creating,
    Running,
    Failed,
    Stopping,
    Stopped,
    Removing,
    Removed,
    CuttingOver,
}

#[derive(Debug, Clone, FromRow)]
pub struct Container {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub docker_container_id: Option<String>,
    pub pending_docker_container_id: Option<String>,
    pub desired_state: DesiredState,
    pub status: ContainerStatus,
    pub current_revision: i64,
}

pub async fn fetch_all(pool: &SqlitePool) -> Result<Vec<Container>> {
    sqlx::query_as!(
        Container,
        r#"select
            id as "id!",
            project_id as "project_id!",
            name as "name!",
            docker_container_id,
            pending_docker_container_id,
            desired_state as "desired_state!: DesiredState",
            status as "status!: ContainerStatus",
            current_revision as "current_revision!"
        from containers"#
    )
    .fetch_all(pool)
    .await
    .context("failed to fetch containers")
}

pub async fn fetch_for_project(
    pool: &SqlitePool,
    project_id: &str,
) -> Result<Option<Container>, sqlx::Error> {
    sqlx::query_as!(
        Container,
        r#"select
            id as "id!",
            project_id as "project_id!",
            name as "name!",
            docker_container_id,
            pending_docker_container_id,
            desired_state as "desired_state!: DesiredState",
            status as "status!: ContainerStatus",
            current_revision as "current_revision!"
        from containers
        where project_id = ?"#,
        project_id
    )
    .fetch_optional(pool)
    .await
}

pub async fn fetch_one(pool: &SqlitePool, id: &str) -> Result<Option<Container>, sqlx::Error> {
    sqlx::query_as!(
        Container,
        r#"select
            id as "id!",
            project_id as "project_id!",
            name as "name!",
            docker_container_id,
            pending_docker_container_id,
            desired_state as "desired_state!: DesiredState",
            status as "status!: ContainerStatus",
            current_revision as "current_revision!"
        from containers
        where id = ?"#,
        id
    )
    .fetch_optional(pool)
    .await
}

pub async fn latest_revision<'a>(
    executor: impl SqliteExecutor<'a>,
    container_id: &str,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query!(
        r#"select max(revision) as "max_revision: i64" from container_revisions where container_id = ?"#,
        container_id
    )
    .fetch_one(executor)
    .await?;

    Ok(row.max_revision.unwrap_or(0))
}

pub async fn fetch_revision_spec(
    pool: &SqlitePool,
    container_id: &str,
    revision: i64,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar!(
        "select spec_json from container_revisions where container_id = ? and revision = ?",
        container_id,
        revision
    )
    .fetch_optional(pool)
    .await
}

#[derive(Debug, FromRow)]
pub struct Revision {
    pub revision: i64,
    pub spec_json: String,
    pub created_at: i64,
}

pub async fn fetch_revisions(pool: &SqlitePool, container_id: &str) -> Result<Vec<Revision>, sqlx::Error> {
    sqlx::query_as!(
        Revision,
        r#"select
            revision as "revision!",
            spec_json as "spec_json!",
            created_at as "created_at!"
        from container_revisions
        where container_id = ?
        order by revision desc"#,
        container_id
    )
    .fetch_all(pool)
    .await
}

pub async fn insert_revision<'a>(
    executor: impl SqliteExecutor<'a>,
    container_id: &str,
    revision: i64,
    spec_json: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "insert into container_revisions (container_id, revision, spec_json) values (?, ?, ?)",
        container_id,
        revision,
        spec_json
    )
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn create_for_project<'a>(
    executor: impl SqliteExecutor<'a>,
    id: &str,
    project_id: &str,
    name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "insert into containers (id, project_id, name) values (?, ?, ?)",
        id,
        project_id,
        name
    )
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn set_desired_state_for_project<'a>(
    executor: impl SqliteExecutor<'a>,
    project_id: &str,
    state: DesiredState,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "update containers set desired_state = ?, updated_at = unixepoch() where project_id = ?",
        state,
        project_id
    )
    .execute(executor)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn set_desired_state(
    pool: &SqlitePool,
    id: &str,
    state: DesiredState,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "update containers set desired_state = ?, updated_at = unixepoch() where id = ?",
        state,
        id
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn set_docker_container_id(
    pool: &SqlitePool,
    id: &str,
    docker_container_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "update containers set docker_container_id = ?, updated_at = unixepoch() where id = ?",
        docker_container_id,
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn set_pending_docker_container_id(
    pool: &SqlitePool,
    id: &str,
    pending_docker_container_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "update containers set pending_docker_container_id = ?, updated_at = unixepoch() where id = ?",
        pending_docker_container_id,
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn promote_pending_to_current(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "update containers
        set docker_container_id = pending_docker_container_id,
            pending_docker_container_id = null,
            updated_at = unixepoch()
        where id = ?",
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn clear_docker_container_id(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "update containers set docker_container_id = null, updated_at = unixepoch() where id = ?",
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn bump_current_revision(pool: &SqlitePool, id: &str, revision: i64) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "update containers set current_revision = ?, updated_at = unixepoch() where id = ?",
        revision,
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn transition(
    pool: &SqlitePool,
    container: &Container,
    to: ContainerStatus,
    reason: Option<&str>,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query!(
        "update containers set status = ?, updated_at = unixepoch() where id = ?",
        to,
        container.id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "insert into container_state_transitions (container_id, from_status, to_status, reason) values (?, ?, ?, ?)",
        container.id,
        container.status,
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
    container: &Container,
    err: &anyhow::Error,
) -> Result<(), sqlx::Error> {
    transition(pool, container, ContainerStatus::Failed, Some(&err.to_string())).await
}
