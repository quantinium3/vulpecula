pub async fn server_version_handler() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
