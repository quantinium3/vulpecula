use axum::{
    Json,
    extract::{Path, State},
};
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::{
    db::queries::{
        project,
        route::{self, DesiredState, Route},
    },
    state::AppState,
    utils::api_response::{ApiError, ApiResponse},
};

fn default_path_prefix() -> String {
    "/".to_string()
}

#[derive(Deserialize, Validate)]
pub struct CreateRouteRequest {
    #[validate(length(min = 1, message = "project_id must not be empty"))]
    project_id: String,
    #[validate(length(min = 1, message = "domain must not be empty"))]
    domain: String,
    #[serde(default = "default_path_prefix")]
    path_prefix: String,
}

pub async fn create_route(
    State(state): State<AppState>,
    Json(body): Json<CreateRouteRequest>,
) -> Result<ApiResponse<Route>, ApiError> {
    body.validate()
        .map_err(|e| ApiError::bad_request(e.to_string()))?;

    project::fetch_one(&state.db, &body.project_id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found(format!("project {} not found", body.project_id)))?;

    let id = Uuid::now_v7().to_string();

    match route::create(
        &state.db,
        route::NewRoute {
            id: &id,
            project_id: &body.project_id,
            domain: &body.domain,
            path_prefix: &body.path_prefix,
        },
    )
    .await
    {
        Ok(()) => {}
        Err(err) if err.as_database_error().is_some_and(|e| e.is_unique_violation()) => {
            return Err(ApiError::conflict(format!(
                "project is already linked to {}{}",
                body.domain, body.path_prefix
            )));
        }
        Err(err) => return Err(ApiError::internal(err.into())),
    }

    state.proxy_reconcile_notify.notify_one();

    let created = route::fetch_one(&state.db, &id)
        .await
        .map_err(|e| ApiError::internal(e.into()))?
        .ok_or_else(|| ApiError::internal(anyhow::anyhow!("route {id} vanished after create")))?;

    Ok(ApiResponse::created(created, "route created"))
}

pub async fn list_routes(State(state): State<AppState>) -> Result<ApiResponse<Vec<Route>>, ApiError> {
    let routes = route::fetch_all(&state.db).await.map_err(ApiError::internal)?;

    Ok(ApiResponse::ok(routes, "routes fetched"))
}

pub async fn get_route(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<ApiResponse<Route>, ApiError> {
    let route = route::fetch_one(&state.db, &id)
        .await
        .map_err(|e| ApiError::internal(e.into()))?
        .ok_or_else(|| ApiError::not_found(format!("route {id} not found")))?;

    Ok(ApiResponse::ok(route, "route fetched"))
}

pub async fn delete_route(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<ApiResponse<()>, ApiError> {
    let found = route::set_desired_state(&state.db, &id, DesiredState::Removed)
        .await
        .map_err(|e| ApiError::internal(e.into()))?;

    if !found {
        return Err(ApiError::not_found(format!("route {id} not found")));
    }

    state.proxy_reconcile_notify.notify_one();
    Ok(ApiResponse::accepted((), "route deletion requested"))
}
