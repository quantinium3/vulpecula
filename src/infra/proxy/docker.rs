use anyhow::{Context, Result, bail};
use bollard::{
    Docker,
    errors::Error,
    models::{
        ContainerCreateBody, HostConfig, Mount, MountType, PortBinding, RestartPolicy,
        RestartPolicyNameEnum,
    },
    query_parameters::{
        CreateContainerOptionsBuilder, CreateImageOptionsBuilder, InspectContainerOptionsBuilder,
        StopContainerOptionsBuilder,
    },
};
use futures_util::TryStreamExt;
use std::collections::HashMap;

use crate::constant::{
    CONTAINER_NETWORK_NAME, PROXY_ADMIN_PORT, PROXY_CONFIG_VOLUME, PROXY_CONTAINER_NAME,
    PROXY_DATA_VOLUME, PROXY_HTTP_PORT, PROXY_HTTPS_PORT, PROXY_IMAGE,
};

#[derive(Clone)]
pub struct ProxyDockerClient(Docker);

impl ProxyDockerClient {
    pub fn connect() -> Result<Self> {
        Ok(Self(
            Docker::connect_with_local_defaults().context("failed to connect to docker daemon")?,
        ))
    }

    async fn inspect(&self) -> Result<Option<bollard::models::ContainerInspectResponse>> {
        let options = InspectContainerOptionsBuilder::default().build();
        match self
            .0
            .inspect_container(PROXY_CONTAINER_NAME, Some(options))
            .await
        {
            Ok(resp) => Ok(Some(resp)),
            Err(Error::DockerResponseServerError {
                status_code: 404, ..
            }) => Ok(None),
            Err(e) => Err(e).context("failed to inspect proxy container"),
        }
    }

    pub async fn is_running(&self) -> Result<bool> {
        Ok(self
            .inspect()
            .await?
            .and_then(|c| c.state)
            .and_then(|s| s.running)
            .unwrap_or(false))
    }

    async fn pull_image(&self, image: &str) -> Result<()> {
        let options = CreateImageOptionsBuilder::default().from_image(image).build();

        match self.0.create_image(Some(options), None, None).try_collect::<Vec<_>>().await {
            Ok(_) => Ok(()),
            Err(e) => bail!("failed to pull image {image}: {e}"),
        }
    }

    pub async fn ensure_running(&self) -> Result<()> {
        if let Some(existing) = self.inspect().await? {
            if existing.state.and_then(|s| s.running).unwrap_or(false) {
                return Ok(());
            }

            self.0
                .start_container(PROXY_CONTAINER_NAME, None)
                .await
                .context("failed to start proxy container")?;
            return Ok(());
        }

        if self.0.inspect_image(PROXY_IMAGE).await.is_err() {
            self.pull_image(PROXY_IMAGE).await?;
        }

        let mut port_bindings = HashMap::new();
        port_bindings.insert(
            format!("{PROXY_HTTP_PORT}/tcp"),
            Some(vec![PortBinding {
                host_ip: None,
                host_port: Some(PROXY_HTTP_PORT.to_string()),
            }]),
        );
        port_bindings.insert(
            format!("{PROXY_HTTPS_PORT}/tcp"),
            Some(vec![PortBinding {
                host_ip: None,
                host_port: Some(PROXY_HTTPS_PORT.to_string()),
            }]),
        );
        port_bindings.insert(
            format!("{PROXY_HTTPS_PORT}/udp"),
            Some(vec![PortBinding {
                host_ip: None,
                host_port: Some(PROXY_HTTPS_PORT.to_string()),
            }]),
        );
        port_bindings.insert(
            format!("{PROXY_ADMIN_PORT}/tcp"),
            Some(vec![PortBinding {
                host_ip: Some("127.0.0.1".to_string()),
                host_port: Some(PROXY_ADMIN_PORT.to_string()),
            }]),
        );

        let mounts = vec![
            Mount {
                target: Some("/data".to_string()),
                source: Some(PROXY_DATA_VOLUME.to_string()),
                typ: Some(MountType::VOLUME),
                ..Default::default()
            },
            Mount {
                target: Some("/config".to_string()),
                source: Some(PROXY_CONFIG_VOLUME.to_string()),
                typ: Some(MountType::VOLUME),
                ..Default::default()
            },
        ];

        let config = ContainerCreateBody {
            image: Some(PROXY_IMAGE.to_string()),
            host_config: Some(HostConfig {
                network_mode: Some(CONTAINER_NETWORK_NAME.to_string()),
                port_bindings: Some(port_bindings),
                mounts: Some(mounts),
                cap_add: Some(vec!["NET_ADMIN".to_string()]),
                restart_policy: Some(RestartPolicy {
                    name: Some(RestartPolicyNameEnum::ALWAYS),
                    maximum_retry_count: None,
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        let options = CreateContainerOptionsBuilder::default()
            .name(PROXY_CONTAINER_NAME)
            .build();

        self.0
            .create_container(Some(options), config)
            .await
            .context("failed to create proxy container")?;

        self.0
            .start_container(PROXY_CONTAINER_NAME, None)
            .await
            .context("failed to start proxy container")?;

        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        match self
            .0
            .stop_container(
                PROXY_CONTAINER_NAME,
                Some(StopContainerOptionsBuilder::default().build()),
            )
            .await
        {
            Ok(())
            | Err(Error::DockerResponseServerError {
                status_code: 404, ..
            }) => Ok(()),
            Err(e) => Err(e).context("failed to stop proxy container"),
        }
    }
}
