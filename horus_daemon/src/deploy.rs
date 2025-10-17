use axum::{body::Bytes, http::StatusCode, Json};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use tar::Archive;
use uuid::Uuid;
use horus_core::core::log_buffer::{publish_log, LogEntry, LogType};
use chrono::Local;
use crate::process::ProcessRegistry;

#[derive(Serialize, Deserialize)]
pub struct DeployResponse {
    pub deployment_id: String,
    pub status: String,
    pub pid: Option<u32>,
    pub message: String,
}

// Helper to log deployment events
fn log_deployment(deployment_id: &str, log_type: LogType, message: String) {
    publish_log(LogEntry {
        timestamp: Local::now().format("%H:%M:%S%.3f").to_string(),
        node_name: format!("deploy-{}", deployment_id),
        log_type,
        topic: None,
        message,
        tick_us: 0,
        ipc_ns: 0,  // No IPC for daemon logs
    });
}

pub async fn handle_deploy(body: Bytes, registry: Arc<ProcessRegistry>) -> Result<Json<DeployResponse>, StatusCode> {
    let deployment_id = Uuid::new_v4().to_string();

    log_deployment(&deployment_id, LogType::RemoteDeploy, "Received deployment request".to_string());
    tracing::info!("📦 Received deployment request: {}", deployment_id);

    match deploy_internal(body, &deployment_id, registry).await {
        Ok(response) => {
            log_deployment(&deployment_id, LogType::RemoteDeploy, "Deployment successful".to_string());
            tracing::info!("✅ Deployment {} successful", deployment_id);
            Ok(Json(response))
        }
        Err(e) => {
            log_deployment(&deployment_id, LogType::Error, format!("Deployment failed: {}", e));
            tracing::error!("❌ Deployment {} failed: {}", deployment_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn deploy_internal(body: Bytes, deployment_id: &str, registry: Arc<ProcessRegistry>) -> anyhow::Result<DeployResponse> {
    let deploy_dir = PathBuf::from(format!("/tmp/horus/deploy-{}", deployment_id));
    std::fs::create_dir_all(&deploy_dir)?;

    tracing::debug!("📂 Created deployment directory: {}", deploy_dir.display());

    let tar_gz = body.to_vec();
    let tar = GzDecoder::new(&tar_gz[..]);
    let mut archive = Archive::new(tar);
    archive.unpack(&deploy_dir)?;

    tracing::debug!("📦 Extracted archive to {}", deploy_dir.display());

    let entrypoint = find_entrypoint(&deploy_dir)?;
    log_deployment(deployment_id, LogType::RemoteDeploy, format!("Found entrypoint: {}", entrypoint.display()));
    tracing::info!("🎯 Found entrypoint: {}", entrypoint.display());

    let executable = compile_if_needed(&entrypoint, &deploy_dir, deployment_id)?;
    tracing::info!("✅ Ready to execute: {}", executable.display());

    let pid = execute_file(&executable, deployment_id)?;

    // Detect language from entrypoint extension
    let language = entrypoint
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| match ext {
            "py" => "Python",
            "rs" => "Rust",
            "c" => "C",
            _ => "Unknown",
        })
        .unwrap_or("Unknown")
        .to_string();

    // Register the process
    registry.register(
        deployment_id.to_string(),
        pid,
        language.clone(),
        entrypoint.display().to_string(),
    );

    Ok(DeployResponse {
        deployment_id: deployment_id.to_string(),
        status: "running".to_string(),
        pid: Some(pid),
        message: format!("Successfully deployed and started {}", entrypoint.display()),
    })
}

fn find_entrypoint(deploy_dir: &PathBuf) -> anyhow::Result<PathBuf> {
    let source_files: Vec<PathBuf> = std::fs::read_dir(deploy_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "py" || ext == "rs" || ext == "c")
                .unwrap_or(false)
        })
        .collect();

    // Priority: main.* > first file found
    if let Some(main_file) = source_files.iter().find(|p| {
        p.file_stem()
            .and_then(|n| n.to_str())
            .map(|n| n == "main")
            .unwrap_or(false)
    }) {
        return Ok(main_file.clone());
    }

    source_files
        .first()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("No source files found (looking for .py, .rs, .c)"))
}

fn compile_if_needed(source: &PathBuf, deploy_dir: &PathBuf, deployment_id: &str) -> anyhow::Result<PathBuf> {
    let extension = source
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    match extension {
        "py" => {
            // Python doesn't need compilation
            Ok(source.clone())
        }
        "rs" => compile_rust(source, deploy_dir, deployment_id),
        "c" => compile_c(source, deploy_dir, deployment_id),
        _ => anyhow::bail!("Unsupported file type: {}", extension),
    }
}

fn compile_rust(source: &PathBuf, deploy_dir: &PathBuf, deployment_id: &str) -> anyhow::Result<PathBuf> {
    log_deployment(deployment_id, LogType::RemoteCompile, format!("Compiling Rust: {}", source.display()));
    tracing::info!("🔧 Compiling Rust: {}", source.display());

    let output_name = deploy_dir.join("output");

    let compile_result = Command::new("rustc")
        .arg(source)
        .arg("-o")
        .arg(&output_name)
        .arg("--edition")
        .arg("2021")
        .current_dir(deploy_dir)
        .output()?;

    if !compile_result.status.success() {
        let stderr = String::from_utf8_lossy(&compile_result.stderr);
        log_deployment(deployment_id, LogType::Error, format!("Rust compilation failed: {}", stderr));
        tracing::error!("Rust compilation failed:\n{}", stderr);
        anyhow::bail!("Rust compilation failed:\n{}", stderr);
    }

    log_deployment(deployment_id, LogType::RemoteCompile, "Rust compilation successful".to_string());
    tracing::info!("✅ Rust compilation successful");
    Ok(output_name)
}

fn compile_c(source: &PathBuf, deploy_dir: &PathBuf, deployment_id: &str) -> anyhow::Result<PathBuf> {
    log_deployment(deployment_id, LogType::RemoteCompile, format!("Compiling C: {}", source.display()));
    tracing::info!("🔧 Compiling C: {}", source.display());

    let output_name = deploy_dir.join("output");

    let compile_result = Command::new("gcc")
        .arg(source)
        .arg("-o")
        .arg(&output_name)
        .current_dir(deploy_dir)
        .output()?;

    if !compile_result.status.success() {
        let stderr = String::from_utf8_lossy(&compile_result.stderr);
        log_deployment(deployment_id, LogType::Error, format!("C compilation failed: {}", stderr));
        tracing::error!("C compilation failed:\n{}", stderr);
        anyhow::bail!("C compilation failed:\n{}", stderr);
    }

    log_deployment(deployment_id, LogType::RemoteCompile, "C compilation successful".to_string());
    tracing::info!("✅ C compilation successful");
    Ok(output_name)
}

fn execute_file(path: &PathBuf, deployment_id: &str) -> anyhow::Result<u32> {
    let is_binary = !path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext == "py")
        .unwrap_or(false);

    let child = if is_binary {
        // Execute compiled binary directly
        Command::new(path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    } else {
        // Execute Python script
        Command::new("python3")
            .arg(path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    };

    let pid = child.id();
    log_deployment(deployment_id, LogType::RemoteExecute, format!("Started process with PID: {}", pid));
    tracing::info!("🚀 Started process with PID: {}", pid);

    Ok(pid)
}
