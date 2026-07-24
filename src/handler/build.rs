use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;

use crate::{
    db::queries::{
        build::{self, Build, BuildStatus, NewBuild},
        project,
    },
    state::AppState,
    utils::api_response::{ApiError, ApiResponse},
};

#[derive(Deserialize)]
pub struct CreateBuildRequest {
    status: BuildStatus,
    tag: Option<String>,
    commit_sha: Option<String>,
    error: Option<String>,
    #[serde(default)]
    log: String,
    started_at: i64,
    finished_at: i64,
}

pub async fn create_build(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(body): Json<CreateBuildRequest>,
) -> Result<ApiResponse<Build>, ApiError> {
    let exists = project::fetch_one(&state.db, &project_id)
        .await
        .map_err(ApiError::internal)?
        .is_some();

    if !exists {
        return Err(ApiError::not_found(format!(
            "project {project_id} not found"
        )));
    }

    let created = build::create(
        &state.db,
        NewBuild {
            project_id: &project_id,
            status: body.status,
            tag: body.tag.as_deref(),
            commit_sha: body.commit_sha.as_deref(),
            error: body.error.as_deref(),
            log: &body.log,
            started_at: body.started_at,
            finished_at: body.finished_at,
        },
    )
    .await
    .map_err(ApiError::internal)?;

    Ok(ApiResponse::created(created, "build recorded"))
}

pub async fn list_builds(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<ApiResponse<Vec<Build>>, ApiError> {
    let builds = build::list(&state.db, &project_id)
        .await
        .map_err(ApiError::internal)?;

    Ok(ApiResponse::ok(builds, "builds fetched"))
}

pub async fn get_build(
    State(state): State<AppState>,
    Path((project_id, build_id)): Path<(String, String)>,
) -> Result<ApiResponse<Build>, ApiError> {
    let build = build::get(&state.db, &project_id, &build_id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found(format!("build {build_id} not found")))?;

    Ok(ApiResponse::ok(build, "build fetched"))
}
