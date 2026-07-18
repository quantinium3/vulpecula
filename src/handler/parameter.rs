use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};

use crate::{
    db::queries::parameter::{self, ParameterType},
    state::AppState,
    utils::api_response::{ApiError, ApiResponse},
};

#[derive(Deserialize)]
pub struct CreateParameterRequest {
    key: String,
    #[serde(rename = "type")]
    type_: ParameterType,
    value: String,
}

#[derive(Serialize)]
pub struct ParameterResponse {
    pub key: String,
    #[serde(rename = "type")]
    pub type_: ParameterType,
    pub value: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Deserialize)]
pub struct GetParameterQuery {
    #[serde(default)]
    with_decryption: bool,
}

#[derive(Deserialize)]
pub struct ListParametersQuery {
    path: Option<String>,
}

fn under_path(path: &str, key: &str) -> bool {
    key == path || key.starts_with(&format!("{path}/"))
}

pub async fn list_parameters(
    State(state): State<AppState>,
    Query(query): Query<ListParametersQuery>,
) -> Result<ApiResponse<Vec<ParameterResponse>>, ApiError> {
    let parameters = parameter::fetch_all(&state.db)
        .await
        .map_err(ApiError::internal)?
        .into_iter()
        .filter(|p| {
            query
                .path
                .as_deref()
                .is_none_or(|path| under_path(path, &p.key))
        })
        .map(|p| ParameterResponse {
            key: p.key,
            type_: p.type_,
            value: p.value,
            created_at: p.created_at,
            updated_at: p.updated_at,
        })
        .collect();

    Ok(ApiResponse::ok(parameters, "parameters fetched"))
}

pub async fn get_parameter(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<GetParameterQuery>,
) -> Result<ApiResponse<ParameterResponse>, ApiError> {
    let key = format!("/{key}");
    let parameter = parameter::fetch_one(&state.db, &key)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found(format!("parameter {key} not found")))?;

    let value = match parameter.type_ {
        ParameterType::String => parameter.value,
        ParameterType::SecureString if query.with_decryption => {
            let encrypted = parameter::fetch_encrypted(&state.db, &key)
                .await
                .map_err(ApiError::internal)?
                .ok_or_else(|| {
                    ApiError::internal(anyhow::anyhow!(
                        "parameter {key} vanished between fetch_one and fetch_encrypted"
                    ))
                })?;

            let plaintext = state
                .master_key
                .decrypt(&encrypted)
                .map_err(ApiError::internal)?;

            tracing::info!(key = %key, "secret parameter decrypted");

            Some(plaintext.to_string())
        }
        ParameterType::SecureString => None,
    };

    Ok(ApiResponse::ok(
        ParameterResponse {
            key: parameter.key,
            type_: parameter.type_,
            value,
            created_at: parameter.created_at,
            updated_at: parameter.updated_at,
        },
        "parameter fetched",
    ))
}

pub async fn create_parameter(
    State(state): State<AppState>,
    Json(body): Json<CreateParameterRequest>,
) -> Result<ApiResponse<()>, ApiError> {
    let key = format!("/{}", body.key);
    match body.type_ {
        ParameterType::String => {
            match parameter::create_string(&state.db, &key, &body.value).await {
                Ok(()) => Ok(ApiResponse::created((), "parameter created")),
                Err(err)
                    if err
                        .as_database_error()
                        .is_some_and(|e| e.is_unique_violation()) =>
                {
                    Err(ApiError::conflict(format!(
                        "parameter {key} already exists"
                    )))
                }
                Err(err) => Err(ApiError::internal(err.into())),
            }
        }
        ParameterType::SecureString => {
            let encrypted = state
                .master_key
                .encrypt(&body.value)
                .map_err(ApiError::internal)?;

            match parameter::create_secure_string(&state.db, &key, &encrypted).await {
                Ok(()) => Ok(ApiResponse::created((), "parameter created")),
                Err(err)
                    if err
                        .as_database_error()
                        .is_some_and(|e| e.is_unique_violation()) =>
                {
                    Err(ApiError::conflict(format!(
                        "parameter {key} already exists"
                    )))
                }
                Err(err) => Err(ApiError::internal(err.into())),
            }
        }
    }
}

#[derive(Deserialize)]
pub struct UpdateParameterRequest {
    value: String,
}

pub async fn update_parameter(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(body): Json<UpdateParameterRequest>,
) -> Result<ApiResponse<()>, ApiError> {
    let key = format!("/{key}");
    let existing = parameter::fetch_one(&state.db, &key)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found(format!("parameter {key} not found")))?;

    let updated = match existing.type_ {
        ParameterType::String => parameter::update_string(&state.db, &key, &body.value)
            .await
            .map_err(ApiError::internal)?,
        ParameterType::SecureString => {
            let encrypted = state
                .master_key
                .encrypt(&body.value)
                .map_err(ApiError::internal)?;

            parameter::update_secure_string(&state.db, &key, &encrypted)
                .await
                .map_err(ApiError::internal)?
        }
    };

    if !updated {
        return Err(ApiError::internal(anyhow::anyhow!(
            "parameter {key} vanished during update"
        )));
    }

    Ok(ApiResponse::ok((), "parameter updated"))
}

pub async fn delete_parameter(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<ApiResponse<()>, ApiError> {
    let key = format!("/{key}");
    let found = parameter::delete(&state.db, &key)
        .await
        .map_err(ApiError::internal)?;

    if !found {
        return Err(ApiError::not_found(format!("parameter {key} not found")));
    }

    Ok(ApiResponse::ok((), "parameter deleted"))
}
