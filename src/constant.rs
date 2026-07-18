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
