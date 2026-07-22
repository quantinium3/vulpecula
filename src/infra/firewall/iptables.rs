use anyhow::{Context, Result, ensure};
use std::process::Output;
use tokio::process::Command;

use super::backend::AllowedPort;

const CHAIN: &str = "VULPECULA-FW";
const HOOK: &str = "DOCKER-USER";

async fn run(args: &[&str]) -> Result<Output> {
    Command::new("iptables")
        .args(args)
        .output()
        .await
        .with_context(|| format!("failed to spawn iptables {args:?}"))
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).trim().to_string()
}

async fn chain_exists() -> Result<bool> {
    Ok(run(&["-nL", CHAIN]).await?.status.success())
}

async fn hook_installed() -> Result<bool> {
    Ok(run(&["-C", HOOK, "-j", CHAIN]).await?.status.success())
}

pub async fn apply(allowed: &[AllowedPort]) -> Result<()> {
    if !chain_exists().await? {
        let output = run(&["-N", CHAIN]).await?;
        ensure!(
            output.status.success(),
            "failed to create chain {CHAIN}: {}",
            stderr(&output)
        );
    }

    if !hook_installed().await? {
        let output = run(&["-I", HOOK, "1", "-j", CHAIN]).await?;
        ensure!(
            output.status.success(),
            "failed to hook {CHAIN} into {HOOK}: {}",
            stderr(&output)
        );
    }

    let output = run(&["-F", CHAIN]).await?;
    ensure!(
        output.status.success(),
        "failed to flush {CHAIN}: {}",
        stderr(&output)
    );

    for allowed_port in allowed {
        let port = allowed_port.port.to_string();
        let output = run(&[
            "-A",
            CHAIN,
            "-p",
            allowed_port.protocol.as_str(),
            "--dport",
            &port,
            "-j",
            "ACCEPT",
        ])
        .await?;
        ensure!(
            output.status.success(),
            "failed to allow {}/{port}: {}",
            allowed_port.protocol.as_str(),
            stderr(&output)
        );
    }

    let output = run(&["-A", CHAIN, "-j", "DROP"]).await?;
    ensure!(
        output.status.success(),
        "failed to install default-deny rule in {CHAIN}: {}",
        stderr(&output)
    );

    Ok(())
}

pub async fn teardown() -> Result<()> {
    let _ = run(&["-D", HOOK, "-j", CHAIN]).await;
    let _ = run(&["-F", CHAIN]).await;

    let output = run(&["-X", CHAIN]).await?;
    ensure!(
        output.status.success() || stderr(&output).contains("No chain"),
        "failed to remove chain {CHAIN}: {}",
        stderr(&output)
    );

    Ok(())
}
