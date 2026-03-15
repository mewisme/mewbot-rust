//! Auto-update: fetch latest.json, compare version, download matching binary, replace self, relaunch.

use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use sha2::Digest;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

const LATEST_JSON_URL: &str =
    "https://github.com/mewisme/mewbot-rust/releases/latest/download/latest.json";

#[derive(Debug, Deserialize)]
pub struct LatestRelease {
    #[serde(alias = "currentVersion")]
    #[allow(dead_code)]
    pub current_version: Option<String>,
    pub version: String,
    #[allow(dead_code)]
    pub tag: Option<String>,
    #[serde(alias = "downloadUrlTemplate")]
    pub download_url_template: String,
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub os: String,
    pub arch: String,
    #[allow(dead_code)]
    pub size: u64,
    pub sha256: String,
}

/// Map `std::env::consts::OS` to the value used in latest.json `files[].os`.
pub fn current_os() -> &'static str {
    match env::consts::OS {
        "windows" => "windows",
        "linux" => "linux",
        "macos" => "macos",
        _ => env::consts::OS,
    }
}

/// Map `std::env::consts::ARCH` to the value used in latest.json `files[].arch`.
pub fn current_arch() -> &'static str {
    match env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" | "arm64" => "aarch64",
        _ => env::consts::ARCH,
    }
}

/// Find the asset that matches the current OS and architecture.
pub fn find_asset_for_current_platform(files: &[FileEntry]) -> Option<&FileEntry> {
    let os = current_os();
    let arch = current_arch();
    files
        .iter()
        .find(|f| f.os.eq_ignore_ascii_case(os) && f.arch.eq_ignore_ascii_case(arch))
}

/// Returns true if `remote` is a strictly newer version than `current` (semver).
pub fn is_newer(current: &str, remote: &str) -> bool {
    let cur = semver::Version::parse(current).ok();
    let rem = semver::Version::parse(remote).ok();
    match (cur, rem) {
        (Some(c), Some(r)) => r > c,
        _ => false,
    }
}

/// Fetch and parse latest.json from the default URL.
pub async fn fetch_latest() -> Result<LatestRelease> {
    fetch_latest_from_url(LATEST_JSON_URL).await
}

/// Fetch and parse latest.json from a given URL.
pub async fn fetch_latest_from_url(url: &str) -> Result<LatestRelease> {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .context("build reqwest client")?;
    let resp = client
        .get(url)
        .send()
        .await
        .context("fetch latest.json")?;
    let release: LatestRelease = resp.json().await.context("parse latest.json")?;
    Ok(release)
}

/// Download a URL to a temporary file and return its path.
pub async fn download_to_temp(url: &str) -> Result<PathBuf> {
    let client = Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .context("build reqwest client")?;
    let resp = client.get(url).send().await.context("download binary")?;
    let body = resp.bytes().await.context("download body")?;
    let dir = env::temp_dir();
    let filename = url
        .split('/')
        .last()
        .and_then(|s| s.split('?').next())
        .unwrap_or("mewbot-update");
    let path = dir.join(filename);
    fs::write(&path, &body).context("write temp file")?;
    Ok(path)
}

/// Verify that the file at `path` has the given SHA-256 hash (hex string).
pub fn verify_sha256(path: &std::path::Path, expected_hex: &str) -> Result<()> {
    use std::io::Read;
    let mut f = fs::File::open(path).context("open file for sha256")?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).context("read file for sha256")?;
    let digest = sha2::Sha256::digest(&buf);
    let hex = format!("{:x}", digest);
    if hex.eq_ignore_ascii_case(expected_hex) {
        Ok(())
    } else {
        anyhow::bail!(
            "sha256 mismatch: expected {}, got {}",
            expected_hex,
            hex
        )
    }
}

/// Build download URL from template and asset name.
fn download_url(template: &str, filename: &str) -> String {
    template.replace("{filename}", filename)
}

/// Perform the update: download new binary, verify, run shutdown future, replace current exe, relaunch.
/// The shutdown future should e.g. call shard_manager.shutdown_all() so the bot disconnects cleanly.
pub async fn run_update<Fut>(
    release: &LatestRelease,
    asset: &FileEntry,
    shutdown: Fut,
) -> Result<()>
where
    Fut: std::future::Future<Output = ()>,
{
    let url = download_url(&release.download_url_template, &asset.name);
    crate::info!("Downloading {}...", asset.name);
    let temp_path = download_to_temp(&url).await.context("download new binary")?;
    if let Err(e) = verify_sha256(&temp_path, &asset.sha256) {
        let _ = fs::remove_file(&temp_path);
        return Err(e).context("verify sha256");
    }
    shutdown.await;
    let current_exe = env::current_exe().context("current exe path")?;
    self_replace::self_replace(&temp_path).context("replace executable")?;
    let _ = fs::remove_file(&temp_path);
    // Relaunch the new binary with same args (skip first which is program name).
    let args: Vec<String> = env::args().skip(1).collect();
    let mut cmd = Command::new(&current_exe);
    cmd.args(&args).stdin(Stdio::null()).stdout(Stdio::inherit()).stderr(Stdio::inherit());
    cmd.spawn().context("relaunch process")?;
    std::process::exit(0);
}
