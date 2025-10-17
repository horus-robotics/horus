use anyhow::{Context, Result};
use colored::Colorize;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tar::Builder;

#[derive(Serialize, Deserialize, Debug)]
struct DeployResponse {
    deployment_id: String,
    status: String,
    pid: Option<u32>,
    message: String,
}

pub fn execute_remote(robot_addr: &str, file: Option<PathBuf>) -> Result<()> {
    println!("{} Deploying to remote robot: {}", "→".cyan(), robot_addr.yellow());

    let project_dir = std::env::current_dir()?;

    let entry_file = if let Some(f) = file {
        f
    } else {
        detect_entry_file(&project_dir)?
    };

    println!("{} Packaging {}...", "→".cyan(), entry_file.display());
    let tar_gz_data = package_project(&entry_file)?;

    let url = normalize_url(robot_addr);
    println!("{} Uploading to {}...", "→".cyan(), url);

    let response = upload_to_daemon(&url, tar_gz_data)?;

    println!("\n{}", "✅ Deployment successful!".green().bold());
    println!("   Deployment ID: {}", response.deployment_id.yellow());
    println!("   Status: {}", response.status.green());
    if let Some(pid) = response.pid {
        println!("   PID: {}", pid.to_string().yellow());
    }
    println!("   {}", response.message.dimmed());

    Ok(())
}

fn detect_entry_file(dir: &PathBuf) -> Result<PathBuf> {
    // Try standard main files first (priority: Rust > Python > C)
    let main_candidates = vec!["main.rs", "main.py", "main.c"];

    for candidate in main_candidates {
        let path = dir.join(candidate);
        if path.exists() {
            return Ok(path);
        }
    }

    // Fallback: find any supported file
    let source_files: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "rs" || ext == "py" || ext == "c")
                .unwrap_or(false)
        })
        .collect();

    if let Some(first_file) = source_files.first() {
        return Ok(first_file.clone());
    }

    anyhow::bail!("No source files found in current directory.\n\n\
        Supported files: .rs (Rust), .py (Python), .c (C)\n\
        Tip: Create a main.rs, main.py, or main.c file")
}

fn package_project(entry_file: &PathBuf) -> Result<Vec<u8>> {
    let tar_gz = Vec::new();
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut tar = Builder::new(enc);

    let file_name = entry_file
        .file_name()
        .context("Invalid file name")?
        .to_str()
        .context("Invalid UTF-8 in filename")?;

    tar.append_path_with_name(entry_file, file_name)?;

    let enc = tar.into_inner()?;
    let tar_gz_data = enc.finish()?;

    Ok(tar_gz_data)
}

fn normalize_url(addr: &str) -> String {
    if addr.starts_with("http://") || addr.starts_with("https://") {
        if addr.contains("/deploy") {
            addr.to_string()
        } else {
            format!("{}/deploy", addr.trim_end_matches('/'))
        }
    } else {
        let normalized = if addr.contains(':') {
            addr.to_string()
        } else {
            format!("{}:8080", addr)
        };
        format!("http://{}/deploy", normalized)
    }
}

fn upload_to_daemon(url: &str, data: Vec<u8>) -> Result<DeployResponse> {
    let client = reqwest::blocking::Client::new();

    let response = client
        .post(url)
        .header("Content-Type", "application/gzip")
        .body(data)
        .send()
        .context("Failed to send deployment request")?;

    if !response.status().is_success() {
        anyhow::bail!("Deployment failed with status: {}", response.status());
    }

    let deploy_response: DeployResponse = response
        .json()
        .context("Failed to parse deployment response")?;

    Ok(deploy_response)
}
