pub const DEFAULT_PROJECT_RETENTION_COUNT: i64 = 3;

pub const CONTAINER_NETWORK_NAME: &str = "vulpecula";
pub const REGISTRY_CONTAINER_NAME: &str = "vulpecula-registry";
pub const REGISTRY_IMAGE: &str = "registry:3";
pub const REGISTRY_PORT: u16 = 5000;

pub const PROXY_CONTAINER_NAME: &str = "vulpecula-proxy";
pub const PROXY_IMAGE: &str = "caddybuilds/caddy-cloudflare:latest";
pub const PROXY_HTTP_PORT: u16 = 80;
pub const PROXY_HTTPS_PORT: u16 = 443;
pub const PROXY_ADMIN_PORT: u16 = 2019;
pub const PROXY_ADMIN_ADDR: &str = "127.0.0.1:2019";
pub const PROXY_DATA_VOLUME: &str = "vulpecula-caddy-data";
pub const PROXY_CONFIG_VOLUME: &str = "vulpecula-caddy-config";

pub struct SeedPackage {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub apt_name: Option<&'static str>,
    pub rpm_name: Option<&'static str>,
}

pub const SEED_PACKAGES: &[&SeedPackage] = &[
    &SeedPackage {
        id: "curl",
        name: "curl",
        description: "command line tool for transferring data to and from internet servers",
        apt_name: Some("curl"),
        rpm_name: Some("curl"),
    },
    &SeedPackage {
        id: "curl-minimal",
        name: "curl-minimal",
        description: "curl-minimal is a lightweight alternative to the standard curl package, designed to provide a smaller installation footprint by restricting supported protocols to standard HTTP, HTTPS, and FTP, stripping out less frequently used, semi-obsolete protocols",
        apt_name: None,
        rpm_name: Some("curl-minimal"),
    },
    &SeedPackage {
        id: "git",
        name: "git",
        description: "Git is a fast, scalable, distributed revision control system with an
        unusually rich command set that provides both high-level operations
        and full access to internals.",
        apt_name: Some("git"),
        rpm_name: Some("git"),
    },
    &SeedPackage {
        id: "htop",
        name: "htop",
        description: "interactive text-mode process viewer for Linux",
        apt_name: Some("htop"),
        rpm_name: Some("htop"),
    },
];
