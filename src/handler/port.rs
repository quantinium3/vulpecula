use axum::extract::State;
use serde::Serialize;

use crate::{
    constant::{PROXY_ADMIN_PORT, PROXY_HTTP_PORT, PROXY_HTTPS_PORT, REGISTRY_PORT},
    db::queries::project,
    state::AppState,
    utils::api_response::{ApiError, ApiResponse},
};

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PortKind {
    Reserved,
    Project,
}

#[derive(Serialize)]
pub struct UsedPort {
    pub port: u16,
    pub name: String,
    pub kind: PortKind,
}

fn reserved(port: u16, name: &str) -> UsedPort {
    UsedPort {
        port,
        name: name.to_string(),
        kind: PortKind::Reserved,
    }
}

pub async fn list_used_ports(
    State(state): State<AppState>,
) -> Result<ApiResponse<Vec<UsedPort>>, ApiError> {
    let mut ports = vec![
        reserved(22, "ssh"),
        reserved(PROXY_HTTP_PORT, "proxy (http)"),
        reserved(PROXY_HTTPS_PORT, "proxy (https)"),
        reserved(PROXY_ADMIN_PORT, "proxy (admin)"),
        reserved(REGISTRY_PORT, "registry"),
        reserved(state.config.port, "vulpecula"),
    ];

    let project_ports = project::fetch_used_ports(&state.db)
        .await
        .map_err(ApiError::internal)?;

    for used in project_ports {
        ports.push(UsedPort {
            port: used.port as u16,
            name: used.name,
            kind: PortKind::Project,
        });
    }

    Ok(ApiResponse::ok(ports, "used ports fetched"))
}
