use std::str::FromStr;

use d3d12::AdapterIterator;
use serde::Serialize;
use sysinfo::System;
use uuid::Uuid;

use crate::discord::get_discord_user;

const REG_DEIMOS: &str = "Software\\Deimos\\";

#[derive(Debug, Clone, Serialize)]
pub struct SessionRecord {
    pub install_id: Uuid,

    pub discord_id: Option<String>,
    pub discord_username: Option<String>,

    pub version: String,
    pub exe_hash: String,

    pub os_version: String,
    pub is_wine: bool,

    pub cpu_vendor: String,
    pub cpu_model: String,

    pub gpu_vendor: String,
    pub gpu_model: String,
}

pub fn get_session_info(version: String) -> SessionRecord {
    let exe_data = std::env::current_exe()
        .and_then(std::fs::read)
        .unwrap_or_default();
    let exe_hash = sha256::digest(exe_data);

    let install_id = if let Some(id) = windows_registry::CURRENT_USER
        .create(REG_DEIMOS)
        .ok()
        .and_then(|key| Uuid::from_str(&key.get_string("InstallId").unwrap_or_default()).ok())
    {
        id
    } else {
        let install_id = Uuid::new_v4();
        if let Ok(key) = windows_registry::CURRENT_USER.create(REG_DEIMOS) {
            key.set_string("InstallId", install_id.to_string()).ok();
        }
        install_id
    };

    let mut session = SessionRecord {
        install_id,

        discord_id: None,
        discord_username: None,

        version,
        exe_hash,

        os_version: System::long_os_version().unwrap_or_else(|| "unknown".to_string()),
        is_wine: std::fs::read_to_string("/proc/version")
            .unwrap_or_default()
            .contains("Linux"),

        cpu_vendor: "unknown_vendor".to_string(),
        cpu_model: "unknown_model".to_string(),

        gpu_vendor: "unknown_vendor".to_string(),
        gpu_model: "unknown_model".to_string(),
    };

    if let Ok(discord_user) = get_discord_user("1178403775711563906") {
        session.discord_id = Some(discord_user.id);
        session.discord_username = Some(discord_user.username);
    }

    let s = System::new_with_specifics(
        sysinfo::RefreshKind::nothing().with_cpu(sysinfo::CpuRefreshKind::everything()),
    );

    if let Some(cpu) = s.cpus().first() {
        session.cpu_vendor = cpu.vendor_id().trim().to_string();
        session.cpu_model = cpu.brand().trim().to_string();
    }

    if let Some(gpu) = AdapterIterator::new().ok().and_then(|mut iter| iter.next())
        && let Ok(desc) = gpu.desc()
    {
        session.gpu_vendor = match desc.vendor_id {
            0x10DE => "nvidia".to_string(),
            0x1002 => "amd".to_string(),
            0x8086 => "intel".to_string(),
            _ => format!("vendor_{:04x}", desc.vendor_id),
        };
        session.gpu_model = desc.description;
    }

    session
}
