use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use serde_json::{Value, json};
use sqlx::SqlitePool;
use tokio::sync::Notify;

use crate::{
    db::queries::{
        container,
        parameter::{self, ParameterType},
        project,
        proxy::{self, DesiredState as ProxyDesiredState, DnsProvider, ProxyStatus},
        route::{self, DesiredState as RouteDesiredState, Route, RouteStatus},
    },
    infra::{
        parameters::secrets::MasterKey,
        proxy::{admin::CaddyAdminClient, docker::ProxyDockerClient},
    },
};

const RESYNC_INTERVAL: Duration = Duration::from_secs(30);

pub async fn start(
    pool: &SqlitePool,
    notify: Arc<Notify>,
    docker: ProxyDockerClient,
    master_key: Arc<MasterKey>,
) -> Result<()> {
    let admin = CaddyAdminClient::new();

    reconcile(pool, &docker, &admin, &master_key)
        .await
        .context("failed initial proxy reconcile")?;

    let pool = pool.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(RESYNC_INTERVAL);
        loop {
            tokio::select! {
                _ = notify.notified() => {}
                _ = ticker.tick() => {}
            }
            if let Err(e) = reconcile(&pool, &docker, &admin, &master_key).await {
                tracing::error!(error = ?e, "proxy reconcile failed");
            }
        }
    });

    Ok(())
}

async fn reconcile(
    pool: &SqlitePool,
    docker: &ProxyDockerClient,
    admin: &CaddyAdminClient,
    master_key: &MasterKey,
) -> Result<()> {
    let settings = proxy::fetch(pool).await?;

    match settings.desired_state {
        ProxyDesiredState::Stopped => {
            if docker.is_running().await? {
                proxy::set_status(pool, ProxyStatus::Stopping).await?;
                docker.stop().await?;
            }
            proxy::set_status(pool, ProxyStatus::Stopped).await?;
            Ok(())
        }
        ProxyDesiredState::Running => {
            if !docker.is_running().await? {
                proxy::set_status(pool, ProxyStatus::Starting).await?;
                if let Err(e) = docker.ensure_running().await {
                    proxy::set_status(pool, ProxyStatus::Failed).await?;
                    return Err(e);
                }
            }

            let config = match build_config(pool, master_key, settings.dns_provider).await {
                Ok(config) => config,
                Err(e) => {
                    proxy::set_status(pool, ProxyStatus::Failed).await?;
                    return Err(e);
                }
            };

            if let Err(e) = admin.load(&config).await {
                proxy::set_status(pool, ProxyStatus::Failed).await?;
                return Err(e);
            }

            proxy::set_status(pool, ProxyStatus::Running).await?;
            Ok(())
        }
    }
}

async fn build_config(
    pool: &SqlitePool,
    master_key: &MasterKey,
    dns_provider: Option<DnsProvider>,
) -> Result<Value> {
    let all_routes = route::fetch_all(pool).await?;
    let mut caddy_routes = Vec::new();

    for r in all_routes
        .iter()
        .filter(|r| r.desired_state == RouteDesiredState::Active)
    {
        match resolve_upstream(pool, r).await {
            Ok(Some(dial)) => {
                caddy_routes.push(json!({
                    "match": [{ "host": [r.domain], "path": [format!("{}*", r.path_prefix)] }],
                    "handle": [{
                        "handler": "reverse_proxy",
                        "upstreams": [{ "dial": dial }]
                    }]
                }));
                route::set_status(pool, &r.id, RouteStatus::Synced).await?;
            }
            Ok(None) => {
                route::set_status(pool, &r.id, RouteStatus::Pending).await?;
            }
            Err(e) => {
                tracing::error!(route = %r.id, error = ?e, "failed to resolve route upstream");
                route::set_status(pool, &r.id, RouteStatus::Failed).await?;
            }
        }
    }

    for r in all_routes
        .iter()
        .filter(|r| r.desired_state == RouteDesiredState::Removed)
    {
        route::delete(pool, &r.id).await?;
    }

    let mut config = json!({
        "apps": {
            "http": {
                "servers": {
                    "vulpecula": {
                        "listen": [":80", ":443"],
                        "routes": caddy_routes,
                    }
                }
            }
        }
    });

    if let Some(provider) = dns_provider {
        let credentials = resolve_credentials(pool, master_key).await?;
        let tls_policy = build_tls_policy(provider, &credentials)?;
        config["apps"]["tls"] = json!({ "automation": { "policies": [tls_policy] } });
    }

    Ok(config)
}

async fn resolve_upstream(pool: &SqlitePool, r: &Route) -> Result<Option<String>> {
    let Some(project) = project::fetch_one(pool, &r.project_id).await? else {
        return Ok(None);
    };
    let Some(container_port) = project.container_port else {
        return Ok(None);
    };
    let Some(c) = container::fetch_for_project(pool, &r.project_id).await? else {
        return Ok(None);
    };
    if c.docker_container_id.is_none() {
        return Ok(None);
    }

    Ok(Some(format!("vulpecula-{}:{container_port}", c.id)))
}

async fn resolve_credentials(
    pool: &SqlitePool,
    master_key: &MasterKey,
) -> Result<HashMap<String, String>> {
    let credentials = proxy::fetch_credentials(pool).await?;
    let mut resolved = HashMap::with_capacity(credentials.len());

    for credential in credentials {
        let parameter = parameter::fetch_one(pool, &credential.parameter_key)
            .await?
            .with_context(|| format!("parameter {} not found", credential.parameter_key))?;

        let value = match parameter.type_ {
            ParameterType::String => parameter.value.with_context(|| {
                format!("string parameter {} has no value", credential.parameter_key)
            })?,
            ParameterType::SecureString => {
                let encrypted = parameter::fetch_encrypted(pool, &credential.parameter_key)
                    .await?
                    .with_context(|| {
                        format!(
                            "secure parameter {} has no ciphertext",
                            credential.parameter_key
                        )
                    })?;
                master_key.decrypt(&encrypted)?.to_string()
            }
        };

        resolved.insert(credential.credential_name, value);
    }

    Ok(resolved)
}

fn build_tls_policy(provider: DnsProvider, credentials: &HashMap<String, String>) -> Result<Value> {
    match provider {
        DnsProvider::Cloudflare => {
            let api_token = credentials
                .get("api_token")
                .context("cloudflare dns provider requires an api_token credential")?;

            Ok(json!({
                "issuers": [{
                    "module": "acme.dns.cloudflare",
                    "dns": { "api_token": api_token }
                }]
            }))
        }
    }
}
