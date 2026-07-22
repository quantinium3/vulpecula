use anyhow::{Result, bail};

use super::{iptables, nftables};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Tcp,
    Udp,
}

impl Protocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Protocol::Tcp => "tcp",
            Protocol::Udp => "udp",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllowedPort {
    pub port: u16,
    pub protocol: Protocol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirewallBackend {
    IpTables,
    NfTables,
}

impl FirewallBackend {
    pub async fn detect() -> Result<Self> {
        if which::which("iptables").is_ok() {
            return Ok(FirewallBackend::IpTables);
        }
        if which::which("nft").is_ok() {
            return Ok(FirewallBackend::NfTables);
        }
        bail!("neither iptables nor nft found on this host")
    }

    pub async fn apply(self, allowed: &[AllowedPort]) -> Result<()> {
        match self {
            FirewallBackend::IpTables => iptables::apply(allowed).await,
            FirewallBackend::NfTables => nftables::apply(allowed).await,
        }
    }

    pub async fn teardown(self) -> Result<()> {
        match self {
            FirewallBackend::IpTables => iptables::teardown().await,
            FirewallBackend::NfTables => nftables::teardown().await,
        }
    }
}
