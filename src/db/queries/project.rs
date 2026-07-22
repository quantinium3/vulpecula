use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqliteExecutor, SqlitePool};

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ProjectSourceKind {
    DockerImage,
    GitRepo,
    LocalRepo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Framework {
    Dockerfile,
    React,
    Svelte,
    Express,
    Static,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub source_kind: ProjectSourceKind,
    pub image: Option<String>,
    pub repo_url: Option<String>,
    pub branch: Option<String>,
    pub framework: Option<Framework>,
    pub root_dir: Option<String>,
    pub install_command: Option<String>,
    pub build_command: Option<String>,
    pub output_directory: Option<String>,
    pub start_command: Option<String>,
    pub port: Option<i64>,
    pub container_port: Option<i64>,
    pub retention_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct NewProject<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub source_kind: ProjectSourceKind,
    pub image: Option<&'a str>,
    pub repo_url: Option<&'a str>,
    pub branch: Option<&'a str>,
    pub framework: Option<Framework>,
    pub root_dir: Option<&'a str>,
    pub install_command: Option<&'a str>,
    pub build_command: Option<&'a str>,
    pub output_directory: Option<&'a str>,
    pub start_command: Option<&'a str>,
    pub port: Option<i64>,
    pub container_port: Option<i64>,
    pub retention_count: i64,
}

pub async fn create<'a>(
    executor: impl SqliteExecutor<'a>,
    p: NewProject<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "insert into projects
            (id, name, source_kind, image, repo_url, branch, framework, root_dir,
             install_command, build_command, output_directory, start_command, port, container_port, retention_count)
        values (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        p.id,
        p.name,
        p.source_kind,
        p.image,
        p.repo_url,
        p.branch,
        p.framework,
        p.root_dir,
        p.install_command,
        p.build_command,
        p.output_directory,
        p.start_command,
        p.port,
        p.container_port,
        p.retention_count
    )
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn fetch_all(pool: &SqlitePool) -> Result<Vec<Project>> {
    sqlx::query_as!(
        Project,
        r#"select
            id as "id!",
            name as "name!",
            source_kind as "source_kind!: ProjectSourceKind",
            image,
            repo_url,
            branch,
            framework as "framework: Framework",
            root_dir,
            install_command,
            build_command,
            output_directory,
            start_command,
            port,
            container_port,
            retention_count as "retention_count!",
            created_at as "created_at!",
            updated_at as "updated_at!"
        from projects
        order by name"#
    )
    .fetch_all(pool)
    .await
    .context("failed to fetch projects")
}

pub async fn fetch_one(pool: &SqlitePool, id: &str) -> Result<Option<Project>> {
    sqlx::query_as!(
        Project,
        r#"select
            id as "id!",
            name as "name!",
            source_kind as "source_kind!: ProjectSourceKind",
            image,
            repo_url,
            branch,
            framework as "framework: Framework",
            root_dir,
            install_command,
            build_command,
            output_directory,
            start_command,
            port,
            container_port,
            retention_count as "retention_count!",
            created_at as "created_at!",
            updated_at as "updated_at!"
        from projects
        where id = ?"#,
        id
    )
    .fetch_optional(pool)
    .await
    .context("failed to fetch project")
}

pub struct ProjectUpdate<'a> {
    pub name: &'a str,
    pub framework: Option<Framework>,
    pub root_dir: Option<&'a str>,
    pub install_command: Option<&'a str>,
    pub build_command: Option<&'a str>,
    pub output_directory: Option<&'a str>,
    pub start_command: Option<&'a str>,
    pub port: Option<i64>,
    pub container_port: Option<i64>,
    pub retention_count: i64,
}

pub async fn update<'a>(
    executor: impl SqliteExecutor<'a>,
    id: &str,
    p: ProjectUpdate<'_>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "update projects set
            name = ?, framework = ?, root_dir = ?, install_command = ?, build_command = ?,
            output_directory = ?, start_command = ?, port = ?, container_port = ?,
            retention_count = ?, updated_at = unixepoch()
        where id = ?",
        p.name,
        p.framework,
        p.root_dir,
        p.install_command,
        p.build_command,
        p.output_directory,
        p.start_command,
        p.port,
        p.container_port,
        p.retention_count,
        id
    )
    .execute(executor)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn delete(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!("delete from projects where id = ?", id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct EnvEntry {
    pub env_name: String,
    pub parameter_key: String,
}

pub async fn fetch_env(pool: &SqlitePool, project_id: &str) -> Result<Vec<EnvEntry>> {
    sqlx::query_as!(
        EnvEntry,
        r#"select
            env_name as "env_name!",
            parameter_key as "parameter_key!"
        from project_env
        where project_id = ?
        order by env_name"#,
        project_id
    )
    .fetch_all(pool)
    .await
    .context("failed to fetch project env")
}

pub async fn delete_env_all<'a>(
    executor: impl SqliteExecutor<'a>,
    project_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!("delete from project_env where project_id = ?", project_id)
        .execute(executor)
        .await?;

    Ok(())
}

pub async fn insert_env<'a>(
    executor: impl SqliteExecutor<'a>,
    project_id: &str,
    env_name: &str,
    parameter_key: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "insert into project_env (project_id, env_name, parameter_key) values (?, ?, ?)",
        project_id,
        env_name,
        parameter_key
    )
    .execute(executor)
    .await?;

    Ok(())
}
