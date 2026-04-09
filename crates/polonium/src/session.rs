use std::str::FromStr;

use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce, aead::AeadMut};
use anyhow::Context;
use d3d12::AdapterIterator;
use rand::Rng;
use rsa::{Pkcs1v15Encrypt, RsaPublicKey, pkcs1::DecodeRsaPublicKey};
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

    pub program: String,
    pub version: String,
    pub exe_hash: String,
    pub has_pdb: bool,

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

        program: "deimos".to_string(),
        version,
        exe_hash,
        has_pdb: std::env::current_exe()
            .ok()
            .and_then(|path| std::fs::exists(path.with_extension("pdb")).ok())
            .unwrap_or(false),

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

const POLONIUM_PUB_PKCS: &str = include_str!("../polonium_pub.pem");

pub fn upload_session(session: SessionRecord) -> anyhow::Result<()> {
    if session.install_id.to_u128_le() == 0x77777777_7777_7777_7777_777777777777u128 {
        anyhow::bail!("skipping session creation");
    }

    let session_json = serde_json::to_string(&session).context("session size -1")?;
    let session_encrypted = encrypt_hybrid(session_json.as_bytes()).context("session size -2")?;

    let request = ehttp::Request::post("https://polonium.cohae.dev/m", session_encrypted)
        .with_header(
            "User-Agent",
            format!(
                "polonium/{} (Deimos {})",
                env!("CARGO_PKG_VERSION"),
                session.version,
            ),
        );

    match ehttp::fetch_blocking(&request) {
        Ok(o) => {
            if !(200..=299).contains(&o.status) {
                return Err(anyhow::anyhow!("session size -{}", o.status));
            }
            Ok(())
        }
        Err(e) => Err(anyhow::anyhow!(e)),
    }
}

fn encrypt_hybrid(plaintext: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut rng = rand::rng();
    let pubkey =
        RsaPublicKey::from_pkcs1_pem(POLONIUM_PUB_PKCS).context("failed to parse public key")?;

    let mut aes_key_bytes = [0u8; 32];
    rng.fill_bytes(&mut aes_key_bytes);

    let mut nonce_bytes = [0u8; 12];
    rng.fill_bytes(&mut nonce_bytes);

    let aes_key = Key::<Aes256Gcm>::from_slice(&aes_key_bytes);
    let mut cipher = Aes256Gcm::new(aes_key);
    let aes_ciphertext = match cipher.encrypt(Nonce::from_slice(&nonce_bytes), plaintext) {
        Ok(c) => c,
        Err(e) => return Err(anyhow::anyhow!(e)),
    };

    let aes_key_encrypted = pubkey.encrypt(&mut rng, Pkcs1v15Encrypt, &aes_key_bytes)?;

    let mut encrypted = Vec::new();
    encrypted.extend_from_slice(&aes_key_encrypted);
    encrypted.extend_from_slice(&nonce_bytes);
    encrypted.extend_from_slice(&aes_ciphertext);

    Ok(encrypted)
}
