use anyhow::{Context, Result, ensure};
use std::{
    env,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
};

#[derive(Debug)]
pub struct Config {
    pub host: IpAddr,
    pub port: u16,
    pub database_url: String,
    pub master_key_path: PathBuf,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let host = env::var("VULPECULA_APP_HOST")
            .context("VULPECULA_APP_HOST must be set")?
            .parse::<IpAddr>()
            .context("VULPECULA_APP_HOST must be a valid IP address")?;
        let port = env::var("VULPECULA_APP_PORT")
            .context("VULPECULA_APP_PORT must be set")?
            .parse::<u16>()
            .context("VULPECULA_APP_PORT must be a valid TCP port")?;
        let database_url =
            env::var("VULPECULA_DATABASE_URL").context("VULPECULA_DATABASE_URL must be set")?;
        let raw_master_key_path = env::var("VULPECULA_MASTER_KEY_PATH")
            .context("VULPECULA_MASTER_KEY_PATH must be set")?;
        let master_key_path = PathBuf::from(raw_master_key_path);
        ensure!(
            master_key_path.is_absolute(),
            "VULPECULA_MASTER_KEY_PATH must be an absolute path got {}",
            master_key_path.display()
        );
        Ok(Self {
            host,
            port,
            database_url,
            master_key_path,
        })
    }

    pub fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.host, self.port)
    }
}
