use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    pub os: String,
    pub arch: String,
}
impl Target {
    pub fn host() -> Self {
        Self {
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
        }
    }
    pub fn platform(&self) -> Result<&'static str> {
        match (self.os.as_str(), self.arch.as_str()) {
            ("windows", "x86_64") => Ok("win32-x64"),
            ("linux", "x86_64") => Ok("linux-x64"),
            ("macos", "x86_64") => Ok("darwin-x64"),
            ("macos", "aarch64") => Ok("darwin-arm64"),
            _ => Err(Error::new(
                "HostIncompatible",
                format!("unsupported target {}/{}", self.os, self.arch),
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Artifact {
    pub channel: String,
    pub version: String,
    pub platform: String,
    pub name: String,
    pub url: String,
    pub sha256: String,
}
impl Artifact {
    pub fn validate(&self) -> Result<()> {
        if !crate::util::is_digest(&self.sha256) {
            return Err(Error::new(
                "MissingChecksum",
                format!(
                    "{}/{} has no valid official SHA-256",
                    self.version, self.platform
                ),
            ));
        }
        let url = reqwest::Url::parse(&self.url)
            .map_err(|e| Error::new("InvalidCatalog", e.to_string()))?;
        if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
            return Err(Error::new(
                "InvalidCatalog",
                "official asset must use HTTPS without credentials",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub sha256: Option<String>,
    pub link: Option<String>,
    pub size: u64,
    pub mode: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub schema_version: u32,
    pub recipe: u32,
    pub bundle_id: String,
    pub artifact: Artifact,
    pub provenance: String,
    pub source: PathBuf,
    pub runtime_dir: String,
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalSource {
    pub kind: String,
    pub path: PathBuf,
    pub status: String,
    pub bytes: Option<u64>,
    pub usage: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct LocalEntry {
    pub version: String,
    pub platform: String,
    pub sha256: String,
    pub sources: Vec<LocalSource>,
}
#[derive(Debug, Default, Serialize)]
pub struct LocalReport {
    pub schema_version: u32,
    pub entries: Vec<LocalEntry>,
    pub warnings: Vec<String>,
}
