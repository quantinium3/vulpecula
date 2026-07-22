use anyhow::{Context, Result, bail};
use bollard::{
    Docker,
    errors::Error,
    models::{ContainerCreateBody, HostConfig, NetworkCreateRequest, PortBinding, RestartPolicy, RestartPolicyNameEnum},
    query_parameters::{
        CreateContainerOptionsBuilder, CreateImageOptionsBuilder, InspectContainerOptionsBuilder,
        ListNetworksOptionsBuilder, RemoveContainerOptionsBuilder, StopContainerOptionsBuilder,
    },
};
use futures_util::TryStreamExt;
use std::collections::HashMap;

use crate::constant::{CONTAINER_NETWORK_NAME, REGISTRY_CONTAINER_NAME, REGISTRY_IMAGE, REGISTRY_PORT};

#[derive(Clone)]
pub struct DockerClient(Docker);

impl DockerClient {
    pub fn connect() -> Result<Self> {
        Ok(Self(
            Docker::connect_with_local_defaults().context("failed to connect to docker daemon")?,
        ))
    }

    pub async fn pull_image(&self, image: &str) -> Result<()> {
        let options = CreateImageOptionsBuilder::default().from_image(image).build();

        let result = self.0.create_image(Some(options), None, None).try_collect::<Vec<_>>().await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => bail!("failed to pull image {image}: {e}"),
        }
    }

    pub async fn ensure_network(&self) -> Result<()> {
        let mut filters = HashMap::new();
        filters.insert("name", vec![CONTAINER_NETWORK_NAME]);
        let options = ListNetworksOptionsBuilder::default().filters(&filters).build();

        let networks = self
            .0
            .list_networks(Some(options))
            .await
            .context("failed to list docker networks")?;

        if networks
            .iter()
            .any(|n| n.name.as_deref() == Some(CONTAINER_NETWORK_NAME))
        {
            return Ok(());
        }

        self.0
            .create_network(NetworkCreateRequest {
                name: CONTAINER_NETWORK_NAME.to_string(),
                driver: Some("bridge".to_string()),
                ..Default::default()
            })
            .await
            .context("failed to create docker network")?;

        Ok(())
    }

    pub async fn ensure_registry_running(&self) -> Result<()> {
        let options = InspectContainerOptionsBuilder::default().build();
        match self.0.inspect_container(REGISTRY_CONTAINER_NAME, Some(options)).await {
            Ok(_) => return Ok(()),
            Err(Error::DockerResponseServerError { status_code: 404, .. }) => {}
            Err(e) => return Err(e).context("failed to inspect registry container"),
        }

        self.pull_image(REGISTRY_IMAGE).await?;

        let mut port_bindings = HashMap::new();
        port_bindings.insert(
            format!("{REGISTRY_PORT}/tcp"),
            Some(vec![PortBinding {
                host_ip: None,
                host_port: Some(REGISTRY_PORT.to_string()),
            }]),
        );

        let config = ContainerCreateBody {
            image: Some(REGISTRY_IMAGE.to_string()),
            host_config: Some(HostConfig {
                network_mode: Some(CONTAINER_NETWORK_NAME.to_string()),
                port_bindings: Some(port_bindings),
                restart_policy: Some(RestartPolicy {
                    name: Some(RestartPolicyNameEnum::ALWAYS),
                    maximum_retry_count: None,
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        let options = CreateContainerOptionsBuilder::default()
            .name(REGISTRY_CONTAINER_NAME)
            .build();

        self.0
            .create_container(Some(options), config)
            .await
            .context("failed to create registry container")?;

        self.0
            .start_container(REGISTRY_CONTAINER_NAME, None)
            .await
            .context("failed to start registry container")?;

        Ok(())
    }

    pub async fn create_and_start(
        &self,
        name: &str,
        image: &str,
        container_port: Option<u16>,
        host_port: Option<u16>,
        env: Vec<String>,
    ) -> Result<String> {
        self.pull_image(image).await?;

        let mut host_config = HostConfig {
            network_mode: Some(CONTAINER_NETWORK_NAME.to_string()),
            ..Default::default()
        };

        if let (Some(container_port), Some(host_port)) = (container_port, host_port) {
            let mut port_bindings = HashMap::new();
            port_bindings.insert(
                format!("{container_port}/tcp"),
                Some(vec![PortBinding {
                    host_ip: None,
                    host_port: Some(host_port.to_string()),
                }]),
            );
            host_config.port_bindings = Some(port_bindings);
        }

        let config = ContainerCreateBody {
            image: Some(image.to_string()),
            env: Some(env),
            host_config: Some(host_config),
            ..Default::default()
        };

        let options = CreateContainerOptionsBuilder::default().name(name).build();

        let response = self
            .0
            .create_container(Some(options), config)
            .await
            .with_context(|| format!("failed to create container {name}"))?;

        self.0
            .start_container(name, None)
            .await
            .with_context(|| format!("failed to start container {name}"))?;

        Ok(response.id)
    }

    pub async fn stop_and_remove(&self, docker_container_id: &str) -> Result<()> {
        match self
            .0
            .stop_container(docker_container_id, Some(StopContainerOptionsBuilder::default().build()))
            .await
        {
            Ok(()) | Err(Error::DockerResponseServerError { status_code: 404, .. }) => {}
            Err(e) => return Err(e).context("failed to stop container"),
        }

        match self
            .0
            .remove_container(
                docker_container_id,
                Some(RemoveContainerOptionsBuilder::default().force(true).build()),
            )
            .await
        {
            Ok(()) | Err(Error::DockerResponseServerError { status_code: 404, .. }) => Ok(()),
            Err(e) => Err(e).context("failed to remove container"),
        }
    }
}
