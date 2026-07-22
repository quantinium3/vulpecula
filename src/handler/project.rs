use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    constant::{DEFAULT_PROJECT_RETENTION_COUNT, REGISTRY_PORT},
    db::queries::{
        container::{self, ContainerSpec, DesiredState},
        project::{self, EnvEntry, Framework, NewProject, Project, ProjectSourceKind},
    },
    state::AppState,
    utils::api_response::{ApiError, ApiResponse},
};

fn default_retention_count() -> i64 {
    DEFAULT_PROJECT_RETENTION_COUNT
}

#[derive(Deserialize, Validate)]
pub struct CreateProjectRequest {
    #[validate(length(min = 1, message = "name must not be empty"))]
    name: String,
    #[serde(flatten)]
    source: ProjectSource,
    env: Option<Vec<EnvRef>>,
    #[validate(range(min = 1, max = 65535, message = "port must be a valid TCP port"))]
    port: Option<u16>,
    #[validate(range(
        min = 1,
        max = 65535,
        message = "container_port must be a valid TCP port"
    ))]
    container_port: Option<u16>,
    #[serde(default = "default_retention_count")]
    #[validate(range(min = 0, message = "retention_count must not be negative"))]
    retention_count: i64,
}

#[derive(Deserialize)]
struct EnvRef {
    env_name: String,
    parameter_key: String,
}

#[derive(Deserialize)]
#[serde(tag = "source_kind", rename_all = "snake_case")]
enum ProjectSource {
    DockerImage {
        image: String,
    },
    GitRepo {
        repo_url: String,
        branch: String,
        framework: Framework,
        root_dir: Option<String>,
        install_command: Option<String>,
        build_command: Option<String>,
        output_directory: Option<String>,
        start_command: Option<String>,
    },
    LocalRepo {
        framework: Framework,
        root_dir: Option<String>,
        install_command: Option<String>,
        build_command: Option<String>,
        output_directory: Option<String>,
        start_command: Option<String>,
    },
}

impl ProjectSource {
    fn kind(&self) -> ProjectSourceKind {
        match self {
            ProjectSource::DockerImage { .. } => ProjectSourceKind::DockerImage,
            ProjectSource::GitRepo { .. } => ProjectSourceKind::GitRepo,
            ProjectSource::LocalRepo { .. } => ProjectSourceKind::LocalRepo,
        }
    }

    fn framework(&self) -> Option<Framework> {
        match self {
            ProjectSource::DockerImage { .. } => None,
            ProjectSource::GitRepo { framework, .. }
            | ProjectSource::LocalRepo { framework, .. } => Some(*framework),
        }
    }
}

fn reject_if_set(
    field: &str,
    value: &Option<impl Sized>,
    framework: Framework,
) -> Result<(), ApiError> {
    if value.is_some() {
        return Err(ApiError::bad_request(format!(
            "{field} is not valid for framework {framework:?}"
        )));
    }
    Ok(())
}

fn validate_framework_fields(
    framework: Framework,
    install_command: &Option<String>,
    build_command: &Option<String>,
    output_directory: &Option<String>,
    start_command: &Option<String>,
) -> Result<(), ApiError> {
    match framework {
        Framework::Dockerfile => {
            reject_if_set("install_command", install_command, framework)?;
            reject_if_set("build_command", build_command, framework)?;
            reject_if_set("output_directory", output_directory, framework)?;
            reject_if_set("start_command", start_command, framework)?;
        }
        Framework::React | Framework::Svelte => {
            reject_if_set("start_command", start_command, framework)?;
        }
        Framework::Static => {
            reject_if_set("install_command", install_command, framework)?;
            reject_if_set("build_command", build_command, framework)?;
            reject_if_set("start_command", start_command, framework)?;
        }
        Framework::Express => {
            reject_if_set("output_directory", output_directory, framework)?;
        }
    }
    Ok(())
}

fn validate_source(source: &ProjectSource) -> Result<(), ApiError> {
    match source {
        ProjectSource::DockerImage { .. } => Ok(()),
        ProjectSource::GitRepo {
            framework,
            install_command,
            build_command,
            output_directory,
            start_command,
            ..
        }
        | ProjectSource::LocalRepo {
            framework,
            install_command,
            build_command,
            output_directory,
            start_command,
            ..
        } => validate_framework_fields(
            *framework,
            install_command,
            build_command,
            output_directory,
            start_command,
        ),
    }
}

#[derive(Serialize)]
pub struct ProjectResponse {
    pub id: String,
    pub name: String,
    pub container_id: String,
}

pub async fn list_projects(
    State(state): State<AppState>,
) -> Result<ApiResponse<Vec<Project>>, ApiError> {
    let projects = project::fetch_all(&state.db)
        .await
        .map_err(ApiError::internal)?;

    Ok(ApiResponse::ok(projects, "projects fetched"))
}

#[derive(Serialize)]
pub struct ProjectDetail {
    #[serde(flatten)]
    pub project: Project,
    pub env: Vec<EnvEntry>,
}

pub async fn get_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<ApiResponse<ProjectDetail>, ApiError> {
    let project = project::fetch_one(&state.db, &id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found(format!("project {id} not found")))?;

    let env = project::fetch_env(&state.db, &id)
        .await
        .map_err(ApiError::internal)?;

    Ok(ApiResponse::ok(
        ProjectDetail { project, env },
        "project fetched",
    ))
}

pub async fn delete_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<ApiResponse<()>, ApiError> {
    let found = container::set_desired_state_for_project(&state.db, &id, DesiredState::Removed)
        .await
        .map_err(|e| ApiError::internal(e.into()))?;

    if !found {
        return Err(ApiError::not_found(format!("project {id} not found")));
    }

    state.container_reconcile_notify.notify_one();
    Ok(ApiResponse::accepted((), "project deletion requested"))
}

#[derive(Deserialize, Validate)]
pub struct UpdateProjectRequest {
    #[validate(length(min = 1, message = "name must not be empty"))]
    name: Option<String>,
    framework: Option<Framework>,
    root_dir: Option<String>,
    install_command: Option<String>,
    build_command: Option<String>,
    output_directory: Option<String>,
    start_command: Option<String>,
    #[validate(range(min = 1, max = 65535, message = "port must be a valid TCP port"))]
    port: Option<u16>,
    #[validate(range(
        min = 1,
        max = 65535,
        message = "container_port must be a valid TCP port"
    ))]
    container_port: Option<u16>,
    #[validate(range(min = 0, message = "retention_count must not be negative"))]
    retention_count: Option<i64>,
    env: Option<Vec<EnvRef>>,
}

pub async fn update_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateProjectRequest>,
) -> Result<ApiResponse<Project>, ApiError> {
    body.validate()
        .map_err(|e| ApiError::bad_request(e.to_string()))?;

    let existing = project::fetch_one(&state.db, &id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found(format!("project {id} not found")))?;

    let name = body.name.unwrap_or(existing.name);
    let framework = body.framework.or(existing.framework);
    let root_dir = body.root_dir.or(existing.root_dir);
    let install_command = body.install_command.or(existing.install_command);
    let build_command = body.build_command.or(existing.build_command);
    let output_directory = body.output_directory.or(existing.output_directory);
    let start_command = body.start_command.or(existing.start_command);
    let port = body.port.map(i64::from).or(existing.port);
    let container_port = body
        .container_port
        .map(i64::from)
        .or(existing.container_port);
    let retention_count = body.retention_count.unwrap_or(existing.retention_count);

    if port.is_some() && container_port.is_none() {
        return Err(ApiError::bad_request(
            "container_port is required when port is set",
        ));
    }

    if let Some(framework) = framework {
        validate_framework_fields(
            framework,
            &install_command,
            &build_command,
            &output_directory,
            &start_command,
        )?;

        if framework == Framework::Static && body.env.is_some() {
            return Err(ApiError::bad_request(
                "env is not valid for framework Static",
            ));
        }
    }

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| ApiError::internal(e.into()))?;

    let found = project::update(
        &mut *tx,
        &id,
        project::ProjectUpdate {
            name: &name,
            framework,
            root_dir: root_dir.as_deref(),
            install_command: install_command.as_deref(),
            build_command: build_command.as_deref(),
            output_directory: output_directory.as_deref(),
            start_command: start_command.as_deref(),
            port,
            container_port,
            retention_count,
        },
    )
    .await
    .map_err(|e| ApiError::internal(e.into()))?;

    if !found {
        return Err(ApiError::not_found(format!("project {id} not found")));
    }

    if let Some(env_refs) = &body.env {
        project::delete_env_all(&mut *tx, &id)
            .await
            .map_err(|e| ApiError::internal(e.into()))?;

        for env_ref in env_refs {
            project::insert_env(&mut *tx, &id, &env_ref.env_name, &env_ref.parameter_key)
                .await
                .map_err(|e| ApiError::internal(e.into()))?;
        }
    }

    tx.commit()
        .await
        .map_err(|e| ApiError::internal(e.into()))?;

    let updated = project::fetch_one(&state.db, &id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::internal(anyhow::anyhow!("project {id} vanished after update")))?;

    Ok(ApiResponse::ok(updated, "project updated"))
}

pub async fn create_project(
    State(state): State<AppState>,
    Json(body): Json<CreateProjectRequest>,
) -> Result<ApiResponse<ProjectResponse>, ApiError> {
    body.validate()
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    validate_source(&body.source)?;

    if body.source.framework() == Some(Framework::Static) {
        reject_if_set("env", &body.env, Framework::Static)?;
    }

    if body.port.is_some() && body.container_port.is_none() {
        return Err(ApiError::bad_request(
            "container_port is required when port is set",
        ));
    }

    let project_id = Uuid::now_v7().to_string();
    let container_id = Uuid::now_v7().to_string();
    let source_kind = body.source.kind();
    let port = body.port.map(i64::from);
    let container_port = body.container_port.map(i64::from);

    let (
        image,
        repo_url,
        branch,
        framework,
        root_dir,
        install_command,
        build_command,
        output_directory,
        start_command,
    ) = match &body.source {
        ProjectSource::DockerImage { image } => (
            Some(image.as_str()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ),
        ProjectSource::GitRepo {
            repo_url,
            branch,
            framework,
            root_dir,
            install_command,
            build_command,
            output_directory,
            start_command,
        } => (
            None,
            Some(repo_url.as_str()),
            Some(branch.as_str()),
            Some(*framework),
            root_dir.as_deref(),
            install_command.as_deref(),
            build_command.as_deref(),
            output_directory.as_deref(),
            start_command.as_deref(),
        ),
        ProjectSource::LocalRepo {
            framework,
            root_dir,
            install_command,
            build_command,
            output_directory,
            start_command,
        } => (
            None,
            None,
            None,
            Some(*framework),
            root_dir.as_deref(),
            install_command.as_deref(),
            build_command.as_deref(),
            output_directory.as_deref(),
            start_command.as_deref(),
        ),
    };

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| ApiError::internal(e.into()))?;

    let new_project = NewProject {
        id: &project_id,
        name: &body.name,
        source_kind,
        image,
        repo_url,
        branch,
        framework,
        root_dir,
        install_command,
        build_command,
        output_directory,
        start_command,
        port,
        container_port,
        retention_count: body.retention_count,
    };

    match project::create(&mut *tx, new_project).await {
        Ok(()) => {}
        Err(err)
            if err
                .as_database_error()
                .is_some_and(|e| e.is_unique_violation()) =>
        {
            return Err(ApiError::conflict(format!(
                "project {} already exists",
                body.name
            )));
        }
        Err(err) => return Err(ApiError::internal(err.into())),
    }

    if let Some(env_refs) = &body.env {
        for env_ref in env_refs {
            project::insert_env(
                &mut *tx,
                &project_id,
                &env_ref.env_name,
                &env_ref.parameter_key,
            )
            .await
            .map_err(|e| ApiError::internal(e.into()))?;
        }
    }

    container::create_for_project(&mut *tx, &container_id, &project_id, &body.name)
        .await
        .map_err(|e| ApiError::internal(e.into()))?;

    tx.commit()
        .await
        .map_err(|e| ApiError::internal(e.into()))?;

    Ok(ApiResponse::created(
        ProjectResponse {
            id: project_id,
            name: body.name,
            container_id,
        },
        "project created",
    ))
}

#[derive(Serialize)]
pub struct DeployResponse {
    pub revision: i64,
}

#[derive(Deserialize)]
pub struct DeployRequest {
    tag: Option<String>,
}

pub async fn deploy_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<DeployRequest>,
) -> Result<ApiResponse<DeployResponse>, ApiError> {
    let project = project::fetch_one(&state.db, &id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found(format!("project {id} not found")))?;

    let image = match project.source_kind {
        ProjectSourceKind::DockerImage => project.image.ok_or_else(|| {
            ApiError::internal(anyhow::anyhow!("docker_image project missing image"))
        })?,
        ProjectSourceKind::GitRepo | ProjectSourceKind::LocalRepo => {
            let tag = body.tag.ok_or_else(|| {
                ApiError::bad_request("tag is required for this project's source kind")
            })?;
            format!("localhost:{REGISTRY_PORT}/{}:{tag}", project.name)
        }
    };

    let container = container::fetch_for_project(&state.db, &id)
        .await
        .map_err(|e| ApiError::internal(e.into()))?
        .ok_or_else(|| ApiError::internal(anyhow::anyhow!("project {id} has no container")))?;

    let env = project::fetch_env(&state.db, &id)
        .await
        .map_err(ApiError::internal)?;

    let port = project
        .container_port
        .map(|container_port| container::PortMapping {
            host_port: project.port.map(|p| p as u16),
            container_port: container_port as u16,
        });

    let spec = ContainerSpec::DockerImage { image, port, env };
    let spec_json = serde_json::to_string(&spec).map_err(|e| ApiError::internal(e.into()))?;

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| ApiError::internal(e.into()))?;

    let revision = container::latest_revision(&mut *tx, &container.id)
        .await
        .map_err(|e| ApiError::internal(e.into()))?
        + 1;

    container::insert_revision(&mut *tx, &container.id, revision, &spec_json)
        .await
        .map_err(|e| ApiError::internal(e.into()))?;

    container::set_desired_state_for_project(&mut *tx, &id, DesiredState::Running)
        .await
        .map_err(|e| ApiError::internal(e.into()))?;

    tx.commit()
        .await
        .map_err(|e| ApiError::internal(e.into()))?;

    state.container_reconcile_notify.notify_one();

    Ok(ApiResponse::accepted(
        DeployResponse { revision },
        "deploy requested",
    ))
}

#[derive(Serialize)]
pub struct RevisionResponse {
    pub revision: i64,
    pub spec: ContainerSpec,
    pub created_at: i64,
}

pub async fn get_project_revisions(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<ApiResponse<Vec<RevisionResponse>>, ApiError> {
    let container = container::fetch_for_project(&state.db, &id)
        .await
        .map_err(|e| ApiError::internal(e.into()))?
        .ok_or_else(|| ApiError::not_found(format!("project {id} not found")))?;

    let revisions = container::fetch_revisions(&state.db, &container.id)
        .await
        .map_err(|e| ApiError::internal(e.into()))?
        .into_iter()
        .map(|r| {
            let spec =
                serde_json::from_str(&r.spec_json).map_err(|e| ApiError::internal(e.into()))?;
            Ok(RevisionResponse {
                revision: r.revision,
                spec,
                created_at: r.created_at,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;

    Ok(ApiResponse::ok(revisions, "revisions fetched"))
}
