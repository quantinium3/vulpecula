use axum::{
    Router,
    routing::{get, post, put},
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

    let container_router = Router::new()
        .route("/", get(handler::container::list_containers))
        .route("/{id}", get(handler::container::get_container))
        .route("/{id}/logs", get(handler::container::get_container_logs))
        .route("/{id}/start", post(handler::container::start_container))
        .route("/{id}/stop", post(handler::container::stop_container));

    let project_router = Router::new()
        .route(
            "/",
            get(handler::project::list_projects).post(handler::project::create_project),
        )
        .route(
            "/{id}",
            get(handler::project::get_project)
                .patch(handler::project::update_project)
                .delete(handler::project::delete_project),
        )
        .route("/{id}/deploy", post(handler::project::deploy_project))
        .route("/{id}/revisions", get(handler::project::get_project_revisions))
        .route(
            "/{id}/build",
            get(handler::build::list_builds).post(handler::build::create_build),
        )
        .route("/{id}/build/{build_id}", get(handler::build::get_build));

    let route_router = Router::new()
        .route(
            "/",
            get(handler::route::list_routes).post(handler::route::create_route),
        )
        .route(
            "/{id}",
            get(handler::route::get_route).delete(handler::route::delete_route),
        );

    let proxy_router = Router::new()
        .route(
            "/",
            get(handler::proxy::get_proxy_settings).patch(handler::proxy::update_proxy_settings),
        )
        .route("/enable", post(handler::proxy::enable_proxy))
        .route("/disable", post(handler::proxy::disable_proxy))
        .route("/credentials", get(handler::proxy::list_dns_credentials))
        .route(
            "/credentials/{credential_name}",
            put(handler::proxy::put_dns_credential).delete(handler::proxy::delete_dns_credential),
        );

    let firewall_router = Router::new()
        .route("/", get(handler::firewall::get_firewall_settings))
        .route("/enable", post(handler::firewall::enable_firewall))
        .route("/disable", post(handler::firewall::disable_firewall))
        .route("/toggle", post(handler::firewall::toggle_firewall));

    let api_router = Router::new()
        .nest("/container", container_router)
        .nest("/firewall", firewall_router)
        .nest("/package", package_router)
        .nest("/parameter", parameter_router)
        .nest("/project", project_router)
        .nest("/proxy", proxy_router)
        .nest("/route", route_router)
        .nest("/sysinfo", sysinfo_router);

    Router::new()
        .route("/", get(|| async { "Welcome to Vulpecula" }))
        .route("/healthz", get(handler::health::get_health))
        .route("/version", get(handler::version::get_server_version))
        .nest("/api", api_router)
        .with_state(state)
}
