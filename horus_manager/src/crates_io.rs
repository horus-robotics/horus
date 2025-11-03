use anyhow::{bail, Context, Result};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tar::Archive;

const CRATES_IO_API: &str = "https://crates.io/api/v1";
const CRATES_IO_CDN: &str = "https://static.crates.io/crates";

#[derive(Debug, Serialize, Deserialize)]
struct CrateInfo {
    #[serde(rename = "crate")]
    crate_info: CrateDetail,
}

#[derive(Debug, Serialize, Deserialize)]
struct CrateDetail {
    name: String,
    max_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CrateVersion {
    num: String,
    #[serde(default)]
    yanked: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct VersionsResponse {
    versions: Vec<CrateVersion>,
}

pub struct CratesIoClient {
    client: reqwest::blocking::Client,
}

impl CratesIoClient {
    pub fn new() -> Self {
        let client = reqwest::blocking::Client::builder()
            .user_agent("horus-cli (https://github.com/horus-robotics)")
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Get information about a crate from crates.io
    pub fn get_crate_info(&self, name: &str) -> Result<CrateInfo> {
        let url = format!("{}/crates/{}", CRATES_IO_API, name);

        let response = self.client
            .get(&url)
            .send()
            .context("Failed to query crates.io")?;

        if !response.status().is_success() {
            bail!("Crate '{}' not found on crates.io", name);
        }

        response.json::<CrateInfo>()
            .context("Failed to parse crate info")
    }

    /// Get latest non-yanked version of a crate
    pub fn get_latest_version(&self, name: &str) -> Result<String> {
        let info = self.get_crate_info(name)?;
        Ok(info.crate_info.max_version)
    }

    /// Download and extract a crate to the specified directory
    pub fn download_crate(&self, name: &str, version: &str, dest: &Path) -> Result<()> {
        // Create destination directory
        fs::create_dir_all(dest)?;

        // Download .crate file (which is a gzipped tarball)
        let url = format!("{}/{}/{}-{}.crate", CRATES_IO_CDN, name, name, version);

        eprintln!("  {} Downloading from crates.io...");
        let response = self.client
            .get(&url)
            .send()
            .context("Failed to download crate")?;

        if !response.status().is_success() {
            bail!("Failed to download crate (version may not exist)");
        }

        // Read response bytes
        let bytes = response.bytes()?;

        // Decompress gzip
        let tar = GzDecoder::new(&bytes[..]);
        let mut archive = Archive::new(tar);

        // Extract to destination
        eprintln!("  {} Extracting...");
        archive.unpack(dest)?;

        // The archive creates a directory named {name}-{version}
        // Move contents up one level
        let extracted_dir = dest.join(format!("{}-{}", name, version));
        if extracted_dir.exists() {
            // Move all contents to parent
            for entry in fs::read_dir(&extracted_dir)? {
                let entry = entry?;
                let file_name = entry.file_name();
                fs::rename(entry.path(), dest.join(file_name))?;
            }
            // Remove now-empty directory
            fs::remove_dir(&extracted_dir)?;
        }

        Ok(())
    }

    /// Compile a crate using cargo (isolated in global cache)
    fn compile_crate(&self, crate_dir: &Path, name: &str) -> Result<()> {
        use std::process::Command;

        eprintln!("  {} Compiling in global cache...");

        // Check if Cargo.toml exists
        let cargo_toml = crate_dir.join("Cargo.toml");
        if !cargo_toml.exists() {
            eprintln!("  {} No Cargo.toml found, skipping compilation");
            return Ok(());
        }

        // Build with cargo in the cache directory (isolated from user's project)
        let output = Command::new("cargo")
            .current_dir(crate_dir)
            .arg("build")
            .arg("--release")
            .arg("--lib")
            .env("CARGO_TARGET_DIR", crate_dir.join("target"))
            .output()
            .context("Failed to run cargo")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("  {} cargo build failed: {}", stderr);
            // Don't fail, just warn - crate is still usable as source
            return Ok(());
        }

        // Create lib directory and copy compiled artifacts
        let lib_dir = crate_dir.join("lib");
        fs::create_dir_all(&lib_dir)?;

        // Find and copy the compiled .rlib file
        let target_release = crate_dir.join("target/release");
        if let Ok(entries) = fs::read_dir(&target_release) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "rlib" {
                        let file_name = path.file_name().unwrap();
                        fs::copy(&path, lib_dir.join(file_name))?;
                    }
                }
            }
        }

        eprintln!("  {} Compiled successfully");
        Ok(())
    }

    /// Install a crate from crates.io to the HORUS cache
    pub fn install(&self, name: &str, version: Option<&str>, cache_dir: &Path) -> Result<PathBuf> {
        // Get version (latest if not specified)
        let version = match version {
            Some(v) => v.to_string(),
            None => self.get_latest_version(name)?,
        };

        let package_dir = cache_dir.join(format!("{}@{}", name, version));

        // Check if already installed
        if package_dir.exists() {
            eprintln!("  {} {}@{} already cached", name, version);
            return Ok(package_dir);
        }

        // Download and extract
        self.download_crate(name, &version, &package_dir)?;

        eprintln!("  {} Downloaded {}@{}", name, version);

        // Compile the crate
        self.compile_crate(&package_dir, name)?;

        Ok(package_dir)
    }

    /// Check if a crate exists on crates.io
    pub fn crate_exists(&self, name: &str) -> bool {
        self.get_crate_info(name).is_ok()
    }
}

// Add colored trait for colored output
use colored::Colorize;
