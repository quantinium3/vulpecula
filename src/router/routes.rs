use axum::{
    Router,
    routing::{get, post},
};

use crate::{handler, state::AppState};

pub fn routes(state: AppState) -> Router {
    let sysinfo_router = Router::new().route("/", get(handler::sysinfo::get_sysinfo));
    let package_router = Router::new()
        .route("/", get(handler::package::get_packages))
        .route("/{id}/install", post(handler::package::install_package))
        .route("/{id}/remove", post(handler::package::remove_package));
    let parameter_router = Router::new()
        .route(
            "/",
            get(handler::parameter::list_parameters).post(handler::parameter::create_parameter),
        )
        .route(
            "/{*key}",
            get(handler::parameter::get_parameter)
                .put(handler::parameter::update_parameter)
                .delete(handler::parameter::delete_parameter),
        );

    let api_router = Router::new()
        .nest("/package", package_router)
        .nest("/parameter", parameter_router)
        .nest("/sysinfo", sysinfo_router);

    Router::new()
        .route("/", get(|| async { "Welcome to Vulpecula" }))
        .route("/healthz", get(handler::health::get_health))
        .route("/version", get(handler::version::get_server_version))
        .nest("/api", api_router)
        .with_state(state)
}
