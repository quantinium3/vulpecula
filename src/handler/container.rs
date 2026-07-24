use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};

use crate::{
    db::queries::container::{self, Container, ContainerStatus, DesiredState},
    state::AppState,
    utils::api_response::{ApiError, ApiResponse},
};

#[derive(Serialize)]
pub struct ContainerResponse {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub docker_container_id: Option<String>,
    pub desired_state: DesiredState,
    pub status: ContainerStatus,
    pub current_revision: i64,
}

impl From<Container> for ContainerResponse {
    fn from(c: Container) -> Self {
        Self {
            id: c.id,
            project_id: c.project_id,
            name: c.name,
            docker_container_id: c.docker_container_id,
            desired_state: c.desired_state,
            status: c.status,
            current_revision: c.current_revision,
        }
    }
}

pub async fn list_containers(
    State(state): State<AppState>,
) -> Result<ApiResponse<Vec<ContainerResponse>>, ApiError> {
    let containers = container::fetch_all(&state.db)
        .await
        .map_err(ApiError::internal)?
        .into_iter()
        .map(ContainerResponse::from)
        .collect();

    Ok(ApiResponse::ok(containers, "containers fetched"))
}

pub async fn get_container(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<ApiResponse<ContainerResponse>, ApiError> {
    let container = container::fetch_one(&state.db, &id)
        .await
        .map_err(|e| ApiError::internal(e.into()))?
        .ok_or_else(|| ApiError::not_found(format!("container {id} not found")))?;

    Ok(ApiResponse::ok(container.into(), "container fetched"))
}

#[derive(Serialize)]
pub struct ContainerLogsResponse {
    pub logs: String,
}

#[derive(Deserialize)]
pub struct LogsQuery {
    tail: Option<i64>,
}

pub async fn get_container_logs(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<LogsQuery>,
) -> Result<ApiResponse<ContainerLogsResponse>, ApiError> {
    let container = container::fetch_one(&state.db, &id)
        .await
        .map_err(|e| ApiError::internal(e.into()))?
        .ok_or_else(|| ApiError::not_found(format!("container {id} not found")))?;

    let logs = match container.docker_container_id {
        Some(docker_id) => state
            .docker
            .logs(&docker_id, query.tail.unwrap_or(200))
            .await
            .map_err(ApiError::internal)?,
        None => String::new(),
    };

    Ok(ApiResponse::ok(ContainerLogsResponse { logs }, "logs fetched"))
}

pub async fn start_container(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<ApiResponse<()>, ApiError> {
    let found = container::set_desired_state(&state.db, &id, DesiredState::Running)
        .await
        .map_err(|e| ApiError::internal(e.into()))?;

    if !found {
        return Err(ApiError::not_found(format!("container {id} not found")));
    }

    state.container_reconcile_notify.notify_one();
    Ok(ApiResponse::ok((), "start requested"))
}

pub async fn stop_container(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<ApiResponse<()>, ApiError> {
    let found = container::set_desired_state(&state.db, &id, DesiredState::Stopped)
        .await
        .map_err(|e| ApiError::internal(e.into()))?;

    if !found {
        return Err(ApiError::not_found(format!("container {id} not found")));
    }

    state.container_reconcile_notify.notify_one();
    Ok(ApiResponse::ok((), "stop requested"))
}
