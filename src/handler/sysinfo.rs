use serde::Serialize;
use sysinfo::{Disks, System};

use crate::utils::api_response::{ApiError, ApiResponse};

#[derive(Serialize)]
struct DiskInfo {
    name: String,
    mount_point: String,
    file_system: String,
    total_space: u64,
    available_space: u64,
    is_removable: bool,
}

#[derive(Serialize)]
pub struct SysinfoResponse {
    name: String,
    kernel_version: String,
    os_version: String,
    long_os_version: String,
    long_kernel_version: String,
    distribution_id: String,
    distribution_id_like: Vec<String>,
    hostname: String,
    cpu_arch: String,
    physical_core_count: usize,
    total_memory: u64,
    total_swap: u64,
    disks: Vec<DiskInfo>,
}

pub async fn get_sysinfo() -> Result<ApiResponse<SysinfoResponse>, ApiError> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let disks = Disks::new_with_refreshed_list()
        .list()
        .iter()
        .map(|disk| DiskInfo {
            name: disk.name().to_string_lossy().into_owned(),
            mount_point: disk.mount_point().to_string_lossy().into_owned(),
            file_system: disk.file_system().to_string_lossy().into_owned(),
            total_space: disk.total_space(),
            available_space: disk.available_space(),
            is_removable: disk.is_removable(),
        })
        .collect();

    Ok(ApiResponse::ok(
        SysinfoResponse {
            name: System::name().unwrap_or_else(|| "<unknown>".to_string()),
            kernel_version: System::kernel_version().unwrap_or_else(|| "<unknown>".to_string()),
            os_version: System::os_version().unwrap_or_else(|| "<unknown>".to_string()),
            long_os_version: System::long_os_version().unwrap_or_else(|| "<unknown>".to_string()),
            long_kernel_version: System::kernel_long_version(),
            distribution_id: System::distribution_id(),
            distribution_id_like: System::distribution_id_like(),
            hostname: System::host_name().unwrap_or_else(|| "<unknown>".to_string()),
            cpu_arch: System::cpu_arch(),
            physical_core_count: System::physical_core_count().unwrap_or(0),
            total_memory: sys.total_memory(),
            total_swap: sys.total_swap(),
            disks,
        },
        "sysinfo fetched successfully",
    ))
}
