use axum::{Router, routing::get};

use crate::{handler, state::AppState};

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/", get(|| async { "Welcome to Vulpecula" }))
        .route("/healthz", get(handler::health::health_handler))
        .route("/version", get(handler::version::server_version_handler))
        .with_state(state)
}
