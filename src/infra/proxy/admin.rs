use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::constant::PROXY_ADMIN_ADDR;

#[derive(Clone)]
pub struct CaddyAdminClient {
    base_url: String,
    client: reqwest::Client,
}

impl CaddyAdminClient {
    pub fn new() -> Self {
        Self {
            base_url: format!("http://{PROXY_ADMIN_ADDR}"),
            client: reqwest::Client::new(),
        }
    }

    pub async fn load(&self, config: &Value) -> Result<()> {
        let response = self
            .client
            .post(format!("{}/load", self.base_url))
            .json(config)
            .send()
            .await
            .context("failed to reach caddy admin api")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("caddy /load failed with {status}: {body}");
        }

        Ok(())
    }
}

impl Default for CaddyAdminClient {
    fn default() -> Self {
        Self::new()
    }
}
