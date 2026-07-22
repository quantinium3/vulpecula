use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use sqlx::SqlitePool;
use tokio::sync::Notify;

use crate::{
    db::queries::{
        container::{self, Container, ContainerSpec, ContainerStatus, DesiredState},
        parameter::{self, ParameterType},
        project::{self, EnvEntry},
    },
    infra::{containers::docker::DockerClient, parameters::secrets::MasterKey},
};

const RESYNC_INTERVAL: Duration = Duration::from_secs(30);

pub async fn start(
    pool: &SqlitePool,
    notify: Arc<Notify>,
    docker: DockerClient,
    master_key: Arc<MasterKey>,
) -> Result<()> {
    reconcile(pool, &docker, &master_key)
        .await
        .context("failed initial container reconcile")?;

    let pool = pool.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(RESYNC_INTERVAL);
        loop {
            tokio::select! {
                _ = notify.notified() => {}
                _ = ticker.tick() => {}
            }
            if let Err(e) = reconcile(&pool, &docker, &master_key).await {
                tracing::error!(error = ?e, "container reconcile failed");
            }
        }
    });

    Ok(())
}

async fn reconcile(pool: &SqlitePool, docker: &DockerClient, master_key: &MasterKey) -> Result<()> {
    let containers = container::fetch_all(pool).await?;

    for c in containers {
        if let Err(e) = reconcile_one(pool, docker, master_key, &c).await {
            tracing::error!(container = %c.id, error = ?e, "failed to reconcile container");
        }
    }

    Ok(())
}

async fn reconcile_one(
    pool: &SqlitePool,
    docker: &DockerClient,
    master_key: &MasterKey,
    c: &Container,
) -> Result<()> {
    match (c.desired_state, c.status, &c.docker_container_id) {
        (
            DesiredState::Removed,
            ContainerStatus::Running
            | ContainerStatus::Failed
            | ContainerStatus::Stopped
            | ContainerStatus::Pending,
            _,
        ) => remove(pool, docker, c).await,
        (
            DesiredState::Running,
            ContainerStatus::Pending | ContainerStatus::Stopped | ContainerStatus::Failed,
            None,
        ) => deploy(pool, docker, master_key, c).await,
        (DesiredState::Running, ContainerStatus::Running, Some(_)) => {
            let latest = container::latest_revision(pool, &c.id).await?;
            if latest > c.current_revision {
                cutover(pool, docker, master_key, c, latest).await
            } else {
                Ok(())
            }
        }
        (DesiredState::Stopped, ContainerStatus::Running | ContainerStatus::Failed, Some(_)) => {
            stop(pool, docker, c).await
        }
        _ => Ok(()),
    }
}

async fn load_spec(
    pool: &SqlitePool,
    master_key: &MasterKey,
    container_id: &str,
    revision: i64,
) -> Result<(String, Option<u16>, Option<u16>, Vec<String>)> {
    let spec_json = container::fetch_revision_spec(pool, container_id, revision)
        .await?
        .context("revision has no spec")?;
    let ContainerSpec::DockerImage { image, port, env } = serde_json::from_str(&spec_json)?;
    let resolved_env = resolve_env(pool, master_key, &env).await?;

    let container_port = port.as_ref().map(|p| p.container_port);
    let host_port = port.and_then(|p| p.host_port);

    Ok((image, container_port, host_port, resolved_env))
}

async fn deploy(
    pool: &SqlitePool,
    docker: &DockerClient,
    master_key: &MasterKey,
    c: &Container,
) -> Result<()> {
    let revision = container::latest_revision(pool, &c.id).await?;
    if revision == 0 {
        return Ok(());
    }

    let (image, container_port, host_port, resolved_env) =
        load_spec(pool, master_key, &c.id, revision).await?;

    container::transition(pool, c, ContainerStatus::Creating, None).await?;
    let c = &Container {
        status: ContainerStatus::Creating,
        ..c.clone()
    };

    let docker_name = format!("vulpecula-{}", c.id);
    match docker
        .create_and_start(
            &docker_name,
            &image,
            container_port,
            host_port,
            resolved_env,
        )
        .await
    {
        Ok(docker_container_id) => {
            container::set_docker_container_id(pool, &c.id, &docker_container_id).await?;
            container::bump_current_revision(pool, &c.id, revision).await?;
            container::transition(pool, c, ContainerStatus::Running, None).await?;
        }
        Err(e) => container::transition_failed(pool, c, &e).await?,
    }

    Ok(())
}

async fn cutover(
    pool: &SqlitePool,
    docker: &DockerClient,
    master_key: &MasterKey,
    c: &Container,
    revision: i64,
) -> Result<()> {
    let (image, container_port, host_port, resolved_env) =
        load_spec(pool, master_key, &c.id, revision).await?;

    container::transition(pool, c, ContainerStatus::CuttingOver, None).await?;
    let c = &Container {
        status: ContainerStatus::CuttingOver,
        ..c.clone()
    };

    if let Some(old_docker_container_id) = &c.docker_container_id {
        docker.stop_and_remove(old_docker_container_id).await?;
        container::clear_docker_container_id(pool, &c.id).await?;
    }

    let docker_name = format!("vulpecula-{}", c.id);
    let pending_docker_container_id = match docker
        .create_and_start(
            &docker_name,
            &image,
            container_port,
            host_port,
            resolved_env,
        )
        .await
    {
        Ok(id) => id,
        Err(e) => {
            container::transition_failed(pool, c, &e).await?;
            return Ok(());
        }
    };

    container::set_pending_docker_container_id(pool, &c.id, Some(&pending_docker_container_id))
        .await?;
    container::promote_pending_to_current(pool, &c.id).await?;
    container::bump_current_revision(pool, &c.id, revision).await?;
    container::transition(pool, c, ContainerStatus::Running, None).await?;

    Ok(())
}

async fn stop(pool: &SqlitePool, docker: &DockerClient, c: &Container) -> Result<()> {
    container::transition(pool, c, ContainerStatus::Stopping, None).await?;
    let c = &Container {
        status: ContainerStatus::Stopping,
        ..c.clone()
    };

    if let Some(docker_container_id) = &c.docker_container_id {
        docker.stop_and_remove(docker_container_id).await?;
        container::clear_docker_container_id(pool, &c.id).await?;
    }

    container::transition(pool, c, ContainerStatus::Stopped, None).await?;
    Ok(())
}

async fn remove(pool: &SqlitePool, docker: &DockerClient, c: &Container) -> Result<()> {
    container::transition(pool, c, ContainerStatus::Removing, None).await?;
    let c = &Container {
        status: ContainerStatus::Removing,
        ..c.clone()
    };

    if let Some(docker_container_id) = &c.docker_container_id {
        docker.stop_and_remove(docker_container_id).await?;
    }

    container::transition(pool, c, ContainerStatus::Removed, None).await?;
    project::delete(pool, &c.project_id).await?;
    Ok(())
}

async fn resolve_env(
    pool: &SqlitePool,
    master_key: &MasterKey,
    entries: &[EnvEntry],
) -> Result<Vec<String>> {
    let mut env = Vec::with_capacity(entries.len());

    for entry in entries {
        let parameter = parameter::fetch_one(pool, &entry.parameter_key)
            .await?
            .with_context(|| format!("parameter {} not found", entry.parameter_key))?;

        let value = match parameter.type_ {
            ParameterType::String => parameter.value.with_context(|| {
                format!("string parameter {} has no value", entry.parameter_key)
            })?,
            ParameterType::SecureString => {
                let encrypted = parameter::fetch_encrypted(pool, &entry.parameter_key)
                    .await?
                    .with_context(|| {
                        format!("secure parameter {} has no ciphertext", entry.parameter_key)
                    })?;
                master_key.decrypt(&encrypted)?.to_string()
            }
        };

        env.push(format!("{}={value}", entry.env_name));
    }

    Ok(env)
}
