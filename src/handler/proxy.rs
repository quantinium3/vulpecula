use axum::{
    Json,
    extract::{Path, State},
};
use serde::Deserialize;
use validator::Validate;

use crate::{
    db::queries::{
        parameter,
        proxy::{self, DesiredState, DnsCredential, DnsProvider, ProxySettings},
    },
    state::AppState,
    utils::api_response::{ApiError, ApiResponse},
};

pub async fn get_proxy_settings(
    State(state): State<AppState>,
) -> Result<ApiResponse<ProxySettings>, ApiError> {
    let settings = proxy::fetch(&state.db).await.map_err(ApiError::internal)?;

    Ok(ApiResponse::ok(settings, "proxy settings fetched"))
}

pub async fn enable_proxy(State(state): State<AppState>) -> Result<ApiResponse<()>, ApiError> {
    proxy::set_desired_state(&state.db, DesiredState::Running)
        .await
        .map_err(|e| ApiError::internal(e.into()))?;

    state.proxy_reconcile_notify.notify_one();
    Ok(ApiResponse::accepted((), "proxy enable requested"))
}

pub async fn disable_proxy(State(state): State<AppState>) -> Result<ApiResponse<()>, ApiError> {
    proxy::set_desired_state(&state.db, DesiredState::Stopped)
        .await
        .map_err(|e| ApiError::internal(e.into()))?;

    state.proxy_reconcile_notify.notify_one();
    Ok(ApiResponse::accepted((), "proxy disable requested"))
}

#[derive(Deserialize)]
pub struct UpdateProxyRequest {
    dns_provider: Option<DnsProvider>,
}

pub async fn update_proxy_settings(
    State(state): State<AppState>,
    Json(body): Json<UpdateProxyRequest>,
) -> Result<ApiResponse<ProxySettings>, ApiError> {
    proxy::set_dns_provider(&state.db, body.dns_provider)
        .await
        .map_err(|e| ApiError::internal(e.into()))?;

    state.proxy_reconcile_notify.notify_one();

    let settings = proxy::fetch(&state.db).await.map_err(ApiError::internal)?;
    Ok(ApiResponse::ok(settings, "proxy settings updated"))
}

pub async fn list_dns_credentials(
    State(state): State<AppState>,
) -> Result<ApiResponse<Vec<DnsCredential>>, ApiError> {
    let credentials = proxy::fetch_credentials(&state.db)
        .await
        .map_err(ApiError::internal)?;

    Ok(ApiResponse::ok(
        credentials,
        "proxy dns credentials fetched",
    ))
}

#[derive(Deserialize, Validate)]
pub struct PutDnsCredentialRequest {
    #[validate(length(min = 1, message = "parameter_key must not be empty"))]
    parameter_key: String,
}

pub async fn put_dns_credential(
    State(state): State<AppState>,
    Path(credential_name): Path<String>,
    Json(body): Json<PutDnsCredentialRequest>,
) -> Result<ApiResponse<()>, ApiError> {
    body.validate()
        .map_err(|e| ApiError::bad_request(e.to_string()))?;

    parameter::fetch_one(&state.db, &body.parameter_key)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| {
            ApiError::not_found(format!("parameter {} not found", body.parameter_key))
        })?;

    proxy::upsert_credential(&state.db, &credential_name, &body.parameter_key)
        .await
        .map_err(|e| ApiError::internal(e.into()))?;

    state.proxy_reconcile_notify.notify_one();
    Ok(ApiResponse::ok((), "proxy dns credential saved"))
}

pub async fn delete_dns_credential(
    State(state): State<AppState>,
    Path(credential_name): Path<String>,
) -> Result<ApiResponse<()>, ApiError> {
    let found = proxy::delete_credential(&state.db, &credential_name)
        .await
        .map_err(|e| ApiError::internal(e.into()))?;

    if !found {
        return Err(ApiError::not_found(format!(
            "dns credential {credential_name} not found"
        )));
    }

    state.proxy_reconcile_notify.notify_one();
    Ok(ApiResponse::ok((), "proxy dns credential deleted"))
}
