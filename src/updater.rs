use std::{io::Cursor, path::Path};

use anyhow::Context;
use serde::Deserialize;
use zip::ZipArchive;

use crate::cli::DEIMOS_VERSION;

#[derive(Debug)]
pub struct AvailableUpdate {
    pub version: String,
    pub download_url: String,
    pub url: String,
    pub changelog: String,
}

const REPOSITORY: &str = "cohaereo/Deimos-Public";

const API_ENDPOINT: &str = "https://api.github.com";
const GET_RELEASES: &str = "/repos/%/releases";

fn github_get<P: AsRef<str>>(path: P) -> ehttp::Request {
    let url = format!("{}{}", API_ENDPOINT, path.as_ref());
    ehttp::Request::get(url)
        .with_header("User-Agent", "deimos")
        .with_header("Accept", "application/vnd.github.v3+json")
        .with_header("X-GitHub-Api-Version", "2022-11-28")
        .with_timeout(Some(std::time::Duration::from_secs(10)))
}

pub fn check_stable_release() -> anyhow::Result<Option<AvailableUpdate>> {
    #[derive(Deserialize, Debug)]
    struct ReleasePartial {
        pub tag_name: String,
        pub name: String,
        pub html_url: String,

        pub assets: Vec<AssetPartial>,
        pub body: String,
    }

    #[derive(Deserialize, Debug)]
    struct AssetPartial {
        pub name: String,
        pub browser_download_url: String,
    }

    let request = github_get(GET_RELEASES.replace('%', REPOSITORY));
    let releases: Vec<ReleasePartial> = match ehttp::fetch_blocking(&request) {
        Ok(response) => response.json()?,
        Err(e) => {
            anyhow::bail!("Failed to fetch releases: {e}")
        }
    };

    let release = releases.into_iter().next().context("No releases found")?;
    let release_semver = semver::Version::parse(&version_fixup(&release.tag_name))?;
    let current_semver = semver::Version::parse(&version_fixup(DEIMOS_VERSION))?;

    if release_semver <= current_semver {
        info!("No updates found");
        return Ok(None);
    }

    let download_url = release
        .assets
        .iter()
        .find(|asset| asset.name == "deimos.zip")
        .map(|asset| asset.browser_download_url.clone())
        .context("deimos.zip not found in release")?;

    Ok(Some(AvailableUpdate {
        version: release.name,
        download_url,
        url: release.html_url,
        changelog: release.body,
    }))
}

pub fn execute_update(zip_data: Vec<u8>) -> anyhow::Result<()> {
    let exe_path = std::env::current_exe().context("Failed to retrieve current executable path")?;
    move_to_old_if_exists(&exe_path).context("failed to move deimos.exe")?;
    move_to_old_if_exists(&exe_path.with_extension("pdb")).context("failed to move deimos.pdb")?;
    move_to_old_if_exists(&exe_path.with_file_name("SDL3.dll"))
        .context("failed to move SDL3.dll")?;

    let mut zip_reader = Cursor::new(zip_data);
    let exe_dir = exe_path
        .parent()
        .context("Exe does not have a parent directory??")?;
    let mut zip = ZipArchive::new(&mut zip_reader).context("Failed to read zip archive")?;
    zip.extract(exe_dir)?;

    if !exe_path.exists() {
        return Err(anyhow::anyhow!("deimos.exe does not exist in the zip"));
    }

    // Spawn the new process
    std::process::Command::new(exe_path)
        .args(std::env::args().skip(1))
        .spawn()
        .context("Failed to spawn the new deimos process")?;

    std::process::exit(0);
}

fn move_to_old_if_exists(path: &Path) -> anyhow::Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let old_path = path.with_file_name(format!(
        "{}.old",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("deimos")
    ));
    if old_path.exists() {
        std::fs::remove_file(&old_path).context("Failed to remove the old deimos executable")?;
    }
    std::fs::rename(path, &old_path).context("Failed to move the old deimos executable")?;
    Ok(())
}

/// Fixes version/tag strings to be compatible with semver
pub fn version_fixup(version: &str) -> String {
    let v = version.replace('v', "");
    if v.chars().filter(|c| *c == '.').count() == 1 {
        format!("{}.0", v)
    } else {
        v
    }
}
