use anyhow::{Context, Result, ensure};
use std::process::Output;
use tokio::process::Command;

use super::backend::AllowedPort;

const TABLE: &str = "vulpecula_fw";
const CHAIN: &str = "prerouting";
const PRIORITY: i32 = -150;

async fn run(args: &[&str]) -> Result<Output> {
    Command::new("nft")
        .args(args)
        .output()
        .await
        .with_context(|| format!("failed to spawn nft {args:?}"))
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).trim().to_string()
}

pub async fn apply(allowed: &[AllowedPort]) -> Result<()> {
    let output = run(&["add", "table", "inet", TABLE]).await?;
    ensure!(
        output.status.success(),
        "failed to create table {TABLE}: {}",
        stderr(&output)
    );

    let priority = PRIORITY.to_string();
    let output = run(&[
        "add",
        "chain",
        "inet",
        TABLE,
        CHAIN,
        &format!("{{ type filter hook prerouting priority {priority}; }}"),
    ])
    .await?;
    ensure!(
        output.status.success(),
        "failed to create chain {CHAIN}: {}",
        stderr(&output)
    );

    let output = run(&["flush", "chain", "inet", TABLE, CHAIN]).await?;
    ensure!(
        output.status.success(),
        "failed to flush chain {CHAIN}: {}",
        stderr(&output)
    );

    for allowed_port in allowed {
        let port = allowed_port.port.to_string();
        let output = run(&[
            "add",
            "rule",
            "inet",
            TABLE,
            CHAIN,
            allowed_port.protocol.as_str(),
            "dport",
            &port,
            "accept",
        ])
        .await?;
        ensure!(
            output.status.success(),
            "failed to allow {}/{port}: {}",
            allowed_port.protocol.as_str(),
            stderr(&output)
        );
    }

    let output = run(&["add", "rule", "inet", TABLE, CHAIN, "drop"]).await?;
    ensure!(
        output.status.success(),
        "failed to install default-deny rule in {CHAIN}: {}",
        stderr(&output)
    );

    Ok(())
}

pub async fn teardown() -> Result<()> {
    let output = run(&["delete", "table", "inet", TABLE]).await?;
    ensure!(
        output.status.success() || stderr(&output).contains("No such file"),
        "failed to remove table {TABLE}: {}",
        stderr(&output)
    );

    Ok(())
}
