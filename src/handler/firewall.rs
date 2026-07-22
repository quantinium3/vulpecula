use axum::extract::State;

use crate::{
    db::queries::firewall::{self, DesiredState, FirewallSettings},
    state::AppState,
    utils::api_response::{ApiError, ApiResponse},
};

pub async fn get_firewall_settings(
    State(state): State<AppState>,
) -> Result<ApiResponse<FirewallSettings>, ApiError> {
    let settings = firewall::fetch(&state.db).await.map_err(ApiError::internal)?;

    Ok(ApiResponse::ok(settings, "firewall settings fetched"))
}

pub async fn enable_firewall(State(state): State<AppState>) -> Result<ApiResponse<()>, ApiError> {
    firewall::set_desired_state(&state.db, DesiredState::Enabled)
        .await
        .map_err(|e| ApiError::internal(e.into()))?;

    state.firewall_reconcile_notify.notify_one();
    Ok(ApiResponse::accepted((), "firewall enable requested"))
}

pub async fn disable_firewall(State(state): State<AppState>) -> Result<ApiResponse<()>, ApiError> {
    firewall::set_desired_state(&state.db, DesiredState::Disabled)
        .await
        .map_err(|e| ApiError::internal(e.into()))?;

    state.firewall_reconcile_notify.notify_one();
    Ok(ApiResponse::accepted((), "firewall disable requested"))
}

pub async fn toggle_firewall(State(state): State<AppState>) -> Result<ApiResponse<FirewallSettings>, ApiError> {
    let current = firewall::fetch(&state.db).await.map_err(ApiError::internal)?;

    let next = match current.desired_state {
        DesiredState::Enabled => DesiredState::Disabled,
        DesiredState::Disabled => DesiredState::Enabled,
    };

    firewall::set_desired_state(&state.db, next)
        .await
        .map_err(|e| ApiError::internal(e.into()))?;

    state.firewall_reconcile_notify.notify_one();

    let settings = firewall::fetch(&state.db).await.map_err(ApiError::internal)?;
    Ok(ApiResponse::accepted(settings, "firewall toggle requested"))
}
