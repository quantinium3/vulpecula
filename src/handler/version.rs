pub async fn get_server_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
