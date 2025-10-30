use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use std::net::UdpSocket;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AppState {
    port: u16,
    params: Arc<horus_core::RuntimeParams>,
}

/// Get local IP address for network access
fn get_local_ip() -> Option<String> {
    // Create a UDP socket to determine local IP
    // This doesn't actually send data, just connects to determine routing
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|addr| addr.ip().to_string())
}

/// Run the web dashboard server
pub async fn run(port: u16) -> anyhow::Result<()> {
    eprintln!("📋 Dashboard will read logs from shared memory ring buffer at /dev/shm/horus_logs");

    let params = Arc::new(
        horus_core::RuntimeParams::init().unwrap_or_else(|_| horus_core::RuntimeParams::default()),
    );

    let state = Arc::new(AppState { port, params });

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/status", get(status_handler))
        .route("/api/nodes", get(nodes_handler))
        .route("/api/topics", get(topics_handler))
        .route("/api/graph", get(graph_handler))
        .route("/api/logs/all", get(logs_all_handler))
        .route("/api/logs/node/:name", get(logs_node_handler))
        .route("/api/logs/topic/:name", get(logs_topic_handler))
        .route("/api/packages/registry", get(packages_registry_handler))
        .route(
            "/api/packages/environments",
            get(packages_environments_handler),
        )
        .route("/api/packages/install", post(packages_install_handler))
        .route("/api/packages/uninstall", post(packages_uninstall_handler))
        .route("/api/packages/publish", post(packages_publish_handler))
        .route("/api/remote/deploy", post(remote_deploy_handler))
        .route("/api/remote/deployments", post(remote_deployments_handler))
        .route("/api/remote/hardware", post(remote_hardware_handler))
        .route("/api/remote/stop", post(remote_stop_handler))
        .route("/api/params", get(params_list_handler))
        .route("/api/params/:key", get(params_get_handler))
        .route("/api/params/:key", post(params_set_handler))
        .route(
            "/api/params/:key",
            axum::routing::delete(params_delete_handler),
        )
        .route("/api/params/export", post(params_export_handler))
        .route("/api/params/import", post(params_import_handler))
        .route("/api/ws", get(websocket_handler))
        .with_state(state)
        .layer(CorsLayer::permissive());

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;

    // Get local IP addresses
    let local_ip = get_local_ip();

    println!("HORUS Web Dashboard is running!");
    println!("\nAccess from:");
    println!("   • Local:    http://localhost:{}", port);
    if let Some(ip) = local_ip {
        println!("   • Network:  http://{}:{}", ip, port);
    }
    println!("\nFeatures:");
    println!("   • Real-time node monitoring");
    println!("   • Topic visualization");
    println!("   • Performance metrics");
    println!("   • Accessible from any device on your network");
    println!("\n   Press Ctrl+C to stop");

    axum::serve(listener, app).await?;

    Ok(())
}

async fn index_handler(State(state): State<Arc<AppState>>) -> Response {
    Html(generate_html(state.port)).into_response()
}

async fn status_handler() -> impl IntoResponse {
    use horus_core::core::HealthStatus;

    // Get all nodes and their health
    let nodes = crate::commands::monitor::discover_nodes().unwrap_or_default();
    let nodes_count = nodes.len();

    let topics_count = crate::commands::monitor::discover_shared_memory()
        .map(|t| t.len())
        .unwrap_or(0);

    // Calculate system-wide health by aggregating all node health
    let (system_status, system_health, health_color) = if nodes_count == 0 {
        ("Idle".to_string(), "No nodes running".to_string(), "gray")
    } else {
        // Count nodes by health status
        let mut healthy = 0;
        let mut warning = 0;
        let mut error = 0;
        let mut critical = 0;
        let mut unknown = 0;

        for node in &nodes {
            match node.health {
                HealthStatus::Healthy => healthy += 1,
                HealthStatus::Warning => warning += 1,
                HealthStatus::Error => error += 1,
                HealthStatus::Critical => critical += 1,
                HealthStatus::Unknown => unknown += 1,
            }
        }

        // System health is determined by worst node health
        let (status, color) = if critical > 0 {
            ("Critical", "red")
        } else if error > 0 {
            ("Degraded", "orange")
        } else if warning > 0 {
            ("Warning", "yellow")
        } else if unknown > 0 && healthy == 0 {
            ("Unknown", "gray")
        } else {
            ("Healthy", "green")
        };

        // Build detailed health summary
        let mut details = Vec::new();
        if critical > 0 {
            details.push(format!("{} critical", critical));
        }
        if error > 0 {
            details.push(format!("{} error", error));
        }
        if warning > 0 {
            details.push(format!("{} warning", warning));
        }
        if healthy > 0 {
            details.push(format!("{} healthy", healthy));
        }
        if unknown > 0 {
            details.push(format!("{} unknown", unknown));
        }

        let health_summary = if details.is_empty() {
            format!("{} nodes", nodes_count)
        } else {
            details.join(", ")
        };

        (status.to_string(), health_summary, color)
    };

    (
        StatusCode::OK,
        serde_json::json!({
            "status": system_status,
            "health": system_health,
            "health_color": health_color,
            "version": "0.1.0",
            "nodes": nodes_count,
            "topics": topics_count
        })
        .to_string(),
    )
}

async fn nodes_handler() -> impl IntoResponse {
    // Use unified backend from monitor module
    let nodes = crate::commands::monitor::discover_nodes()
        .unwrap_or_default()
        .into_iter()
        .map(|n| {
            serde_json::json!({
                "name": n.name,
                "pid": n.process_id,
                "status": n.status,
                "health": n.health.as_str(),
                "health_color": n.health.color(),
                "cpu": format!("{:.1}%", n.cpu_usage),
                "memory": format!("{} MB", n.memory_usage / 1024 / 1024),
                "tick_count": n.tick_count,
                "error_count": n.error_count,
                "tick_rate": n.actual_rate_hz,
            })
        })
        .collect::<Vec<_>>();

    (
        StatusCode::OK,
        serde_json::json!({
            "nodes": nodes
        })
        .to_string(),
    )
}

async fn topics_handler() -> impl IntoResponse {
    // Use unified backend from monitor module
    let topics = crate::commands::monitor::discover_shared_memory()
        .unwrap_or_default()
        .into_iter()
        .map(|t| {
            // Convert IPC name to original format for display
            // horus_sensors_lidar -> sensors/lidar
            let display_name = t
                .topic_name
                .strip_prefix("horus_")
                .unwrap_or(&t.topic_name)
                .replace("_", "/");

            serde_json::json!({
                "name": display_name,
                "size": format!("{} KB", t.size_bytes / 1024),
                "active": t.active,
                "processes": t.accessing_processes.len(),
            })
        })
        .collect::<Vec<_>>();

    (
        StatusCode::OK,
        serde_json::json!({
            "topics": topics
        })
        .to_string(),
    )
}

async fn graph_handler() -> impl IntoResponse {
    // Use graph module to get nodes and edges
    let (nodes, edges) = crate::graph::discover_graph_data();

    let graph_nodes = nodes
        .into_iter()
        .map(|n| {
            serde_json::json!({
                "id": n.id,
                "label": n.label,
                "type": match n.node_type {
                    crate::graph::NodeType::Process => "process",
                    crate::graph::NodeType::Topic => "topic",
                },
                "pid": n.pid,
                "active": n.active,
            })
        })
        .collect::<Vec<_>>();

    let graph_edges = edges
        .into_iter()
        .map(|e| {
            serde_json::json!({
                "from": e.from,
                "to": e.to,
                "type": match e.edge_type {
                    crate::graph::EdgeType::Publish => "publish",
                    crate::graph::EdgeType::Subscribe => "subscribe",
                },
                "active": e.active,
            })
        })
        .collect::<Vec<_>>();

    (
        StatusCode::OK,
        serde_json::json!({
            "nodes": graph_nodes,
            "edges": graph_edges
        })
        .to_string(),
    )
}

async fn logs_all_handler() -> impl IntoResponse {
    use horus_core::core::log_buffer::GLOBAL_LOG_BUFFER;

    let logs = GLOBAL_LOG_BUFFER.get_all();

    (
        StatusCode::OK,
        serde_json::json!({
            "logs": logs
        })
        .to_string(),
    )
}

async fn logs_node_handler(Path(node_name): Path<String>) -> impl IntoResponse {
    use horus_core::core::log_buffer::GLOBAL_LOG_BUFFER;

    eprintln!(" API: Fetching logs for node '{}'", node_name);
    let logs = GLOBAL_LOG_BUFFER.get_for_node(&node_name);
    eprintln!("[#] API: Found {} logs for '{}'", logs.len(), node_name);

    (
        StatusCode::OK,
        serde_json::json!({
            "node": node_name,
            "logs": logs
        })
        .to_string(),
    )
}

async fn logs_topic_handler(Path(topic_name): Path<String>) -> impl IntoResponse {
    use horus_core::core::log_buffer::GLOBAL_LOG_BUFFER;

    // Convert IPC topic name back to original format
    // horus_sensors_lidar -> sensors/lidar
    let original_topic = topic_name
        .strip_prefix("horus_")
        .unwrap_or(&topic_name)
        .replace("_", "/");

    eprintln!(
        " API: Fetching logs for topic '{}' (original: '{}')",
        topic_name, original_topic
    );
    let logs = GLOBAL_LOG_BUFFER.get_for_topic(&original_topic);
    eprintln!("[#] API: Found {} logs for '{}'", logs.len(), original_topic);

    (
        StatusCode::OK,
        serde_json::json!({
            "topic": topic_name,
            "logs": logs
        })
        .to_string(),
    )
}

// Marketplace handlers
#[derive(serde::Deserialize)]
struct SearchQuery {
    q: String,
}

// Registry: Search available packages from remote registry
async fn packages_registry_handler(Query(query): Query<SearchQuery>) -> impl IntoResponse {
    use crate::registry::RegistryClient;

    let result = tokio::task::spawn_blocking(move || {
        let client = RegistryClient::new();
        client.search(&query.q)
    })
    .await;

    match result {
        Ok(Ok(packages)) => {
            let pkgs = packages
                .into_iter()
                .map(|p| {
                    serde_json::json!({
                        "name": p.name,
                        "version": p.version,
                        "description": p.description.unwrap_or_default(),
                    })
                })
                .collect::<Vec<_>>();

            (
                StatusCode::OK,
                serde_json::json!({
                    "packages": pkgs
                })
                .to_string(),
            )
        }
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({
                "error": "Failed to search packages"
            })
            .to_string(),
        ),
    }
}

// Environments: Show global packages and local environments
async fn packages_environments_handler() -> impl IntoResponse {
    use std::fs;
    use std::path::PathBuf;

    let result = tokio::task::spawn_blocking(move || {
        let mut global_packages = Vec::new();
        let mut local_envs = Vec::new();

        // 1. Global Environment: All packages in ~/.horus/cache
        if let Some(home) = dirs::home_dir() {
            let global_cache = home.join(".horus/cache");
            if global_cache.exists() {
                if let Ok(entries) = fs::read_dir(&global_cache) {
                    for entry in entries.flatten() {
                        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                            let name = entry.file_name().to_string_lossy().to_string();

                            // Try to read metadata
                            let metadata_path = entry.path().join("metadata.json");
                            let version = if metadata_path.exists() {
                                fs::read_to_string(&metadata_path)
                                    .ok()
                                    .and_then(|s| {
                                        serde_json::from_str::<serde_json::Value>(&s).ok()
                                    })
                                    .and_then(|j| {
                                        j.get("version")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string())
                                    })
                                    .unwrap_or_else(|| "unknown".to_string())
                            } else {
                                "unknown".to_string()
                            };

                            global_packages.push(serde_json::json!({
                                "name": name,
                                "version": version,
                            }));
                        }
                    }
                }
            }
        }

        // 2. Local Environments: Find all directories with .horus/ subdirectory
        // Search in current dir and home dir
        let search_paths = vec![PathBuf::from("."), dirs::home_dir().unwrap_or_default()];

        for base_path in search_paths {
            if !base_path.exists() {
                continue;
            }

            // Walk through directories to find .horus/ folders
            if let Ok(entries) = fs::read_dir(&base_path) {
                for entry in entries.flatten() {
                    if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        continue;
                    }

                    let horus_dir = entry.path().join(".horus");
                    if horus_dir.exists() && horus_dir.is_dir() {
                        let env_name = entry.file_name().to_string_lossy().to_string();
                        let env_path = entry.path();

                        // Get packages inside this environment
                        let packages_dir = horus_dir.join("packages");
                        let mut packages = Vec::new();

                        if packages_dir.exists() {
                            if let Ok(pkg_entries) = fs::read_dir(&packages_dir) {
                                for pkg_entry in pkg_entries.flatten() {
                                    if pkg_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                                        let pkg_name =
                                            pkg_entry.file_name().to_string_lossy().to_string();

                                        // Try to get version from metadata.json
                                        let metadata_path = pkg_entry.path().join("metadata.json");
                                        let version = if metadata_path.exists() {
                                            fs::read_to_string(&metadata_path)
                                                .ok()
                                                .and_then(|s| {
                                                    serde_json::from_str::<serde_json::Value>(&s)
                                                        .ok()
                                                })
                                                .and_then(|j| {
                                                    j.get("version")
                                                        .and_then(|v| v.as_str())
                                                        .map(|s| s.to_string())
                                                })
                                                .unwrap_or_else(|| "unknown".to_string())
                                        } else {
                                            "unknown".to_string()
                                        };

                                        // Scan for installed packages inside this package's .horus/packages/
                                        let nested_packages_dir = pkg_entry.path().join(".horus/packages");
                                        let mut installed_packages = Vec::new();

                                        if nested_packages_dir.exists() {
                                            if let Ok(nested_entries) = fs::read_dir(&nested_packages_dir) {
                                                for nested_entry in nested_entries.flatten() {
                                                    if nested_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                                                        let nested_name = nested_entry.file_name().to_string_lossy().to_string();

                                                        // Try to get version
                                                        let nested_metadata_path = nested_entry.path().join("metadata.json");
                                                        let nested_version = if nested_metadata_path.exists() {
                                                            fs::read_to_string(&nested_metadata_path)
                                                                .ok()
                                                                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                                                                .and_then(|j| j.get("version").and_then(|v| v.as_str()).map(|s| s.to_string()))
                                                                .unwrap_or_else(|| "unknown".to_string())
                                                        } else {
                                                            "unknown".to_string()
                                                        };

                                                        installed_packages.push(serde_json::json!({
                                                            "name": nested_name,
                                                            "version": nested_version,
                                                        }));
                                                    }
                                                }
                                            }
                                        }

                                        packages.push(serde_json::json!({
                                            "name": pkg_name,
                                            "version": version,
                                            "installed_packages": installed_packages,
                                        }));
                                    }
                                }
                            }
                        }

                        local_envs.push(serde_json::json!({
                            "name": env_name,
                            "path": env_path.to_string_lossy(),
                            "packages": packages,
                            "package_count": packages.len(),
                        }));
                    }
                }
            }
        }

        serde_json::json!({
            "global": global_packages,
            "local": local_envs
        })
    })
    .await;

    match result {
        Ok(data) => (StatusCode::OK, data.to_string()),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({
                "error": "Failed to list environments"
            })
            .to_string(),
        ),
    }
}

#[derive(serde::Deserialize)]
struct InstallRequest {
    package: String,
    #[serde(default)]
    target: Option<String>,
}

async fn packages_install_handler(Json(req): Json<InstallRequest>) -> impl IntoResponse {
    use crate::registry::RegistryClient;
    use std::path::PathBuf;

    let package_name = req.package.clone();
    let target = req.target.clone();

    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<(String, String)> {
        let client = RegistryClient::new();

        // Determine target based on input and horus.yaml path
        let (_install_result, horus_yaml_path) = if let Some(target_str) = &target {
            if target_str == "global" {
                // Install globally - no horus.yaml to update
                let result = client.install_to_target(&req.package, None, crate::workspace::InstallTarget::Global)?;
                (result, None)
            } else {
                // Use specified path - find horus.yaml in parent package
                let target_path = PathBuf::from(target_str);

                // Extract parent package path (remove /.horus/packages/package_name)
                // target_path format: /path/to/project/.horus/packages/parent_package
                let parent_path = if target_path.ends_with(".horus/packages") {
                    target_path.parent().and_then(|p| p.parent())
                } else {
                    // Likely: /path/.horus/packages/parent_package
                    target_path.parent()
                        .and_then(|p| p.parent()) // Remove parent_package
                        .and_then(|p| p.parent()) // Remove packages
                        .and_then(|p| p.parent()) // Remove .horus
                };

                let yaml_path = parent_path.map(|p| p.join("horus.yaml"));

                let result = client.install_to_target(&req.package, None, crate::workspace::InstallTarget::Local(target_path))?;
                (result, yaml_path)
            }
        } else {
            // Default: auto-detect - look for horus.yaml in current dir
            let yaml_path = PathBuf::from("horus.yaml");
            let yaml_path = if yaml_path.exists() { Some(yaml_path) } else { None };
            let result = client.install(&req.package, None)?;
            (result, yaml_path)
        };

        // Get installed version (try to read from metadata.json)
        let version = "latest".to_string(); // TODO: Get actual version from install result

        // Update horus.yaml if path exists
        if let Some(yaml_path) = horus_yaml_path {
            if yaml_path.exists() {
                crate::yaml_utils::add_dependency_to_horus_yaml(&yaml_path, &req.package, &version)?;
            }
        }

        Ok((req.package.clone(), version))
    })
    .await;

    match result {
        Ok(Ok(_)) => (
            StatusCode::OK,
            serde_json::json!({
                "success": true,
                "message": format!("Successfully installed {}", package_name)
            })
            .to_string(),
        ),
        Ok(Err(e)) => (
            StatusCode::BAD_REQUEST,
            serde_json::json!({
                "success": false,
                "error": e.to_string()
            })
            .to_string(),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({
                "success": false,
                "error": format!("Task failed: {}", e)
            })
            .to_string(),
        ),
    }
}

#[derive(serde::Deserialize)]
struct UninstallRequest {
    parent_package: String,
    package: String,
}

async fn packages_uninstall_handler(Json(req): Json<UninstallRequest>) -> impl IntoResponse {
    use std::fs;
    use std::path::PathBuf;

    let parent_package = req.parent_package.clone();
    let package = req.package.clone();
    let parent_package_msg = parent_package.clone();
    let package_msg = package.clone();

    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        // Find the parent package in local environments
        let search_paths = vec![PathBuf::from("."), dirs::home_dir().unwrap_or_default()];

        for base_path in search_paths {
            if !base_path.exists() {
                continue;
            }

            if let Ok(entries) = fs::read_dir(&base_path) {
                for entry in entries.flatten() {
                    if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        continue;
                    }

                    let horus_dir = entry.path().join(".horus");
                    if !horus_dir.exists() {
                        continue;
                    }

                    // Check if this environment has the parent package
                    let parent_pkg_path = horus_dir.join("packages").join(&parent_package);
                    if !parent_pkg_path.exists() {
                        continue;
                    }

                    // Found the parent package, now uninstall the nested package
                    let nested_pkg_path = parent_pkg_path.join(".horus/packages").join(&package);
                    if nested_pkg_path.exists() {
                        fs::remove_dir_all(&nested_pkg_path)?;

                        // Update horus.yaml of the parent package
                        // The parent package directory structure is: <project_root>/.horus/packages/<parent_package>
                        // We need to go up to the project root and find horus.yaml
                        let project_root = parent_pkg_path.parent().and_then(|p| p.parent());
                        if let Some(root) = project_root {
                            let horus_yaml_path = root.join("horus.yaml");
                            if horus_yaml_path.exists() {
                                // Ignore errors in updating horus.yaml - package is already uninstalled
                                let _ = crate::yaml_utils::remove_dependency_from_horus_yaml(&horus_yaml_path, &package);
                            }
                        }

                        return Ok(());
                    }
                }
            }
        }

        anyhow::bail!("Package not found")
    })
    .await;

    match result {
        Ok(Ok(_)) => (
            StatusCode::OK,
            serde_json::json!({
                "success": true,
                "message": format!("Successfully uninstalled {} from {}", package_msg, parent_package_msg)
            })
            .to_string(),
        ),
        Ok(Err(e)) => (
            StatusCode::BAD_REQUEST,
            serde_json::json!({
                "success": false,
                "error": e.to_string()
            })
            .to_string(),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({
                "success": false,
                "error": format!("Task failed: {}", e)
            })
            .to_string(),
        ),
    }
}

async fn packages_publish_handler() -> impl IntoResponse {
    use crate::registry::RegistryClient;

    let result = tokio::task::spawn_blocking(move || {
        let client = RegistryClient::new();
        client.publish(None)
    })
    .await;

    match result {
        Ok(Ok(_)) => (
            StatusCode::OK,
            serde_json::json!({
                "success": true,
                "message": "Package published successfully"
            })
            .to_string(),
        ),
        Ok(Err(e)) => (
            StatusCode::BAD_REQUEST,
            serde_json::json!({
                "success": false,
                "error": e.to_string()
            })
            .to_string(),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({
                "success": false,
                "error": format!("Task failed: {}", e)
            })
            .to_string(),
        ),
    }
}

// Remote deployment handler
#[derive(serde::Deserialize)]
struct DeployRequest {
    robot_addr: String,
    file: Option<String>,
}

async fn remote_deploy_handler(Json(req): Json<DeployRequest>) -> impl IntoResponse {
    use crate::commands::remote::execute_remote;
    use std::path::PathBuf;

    let file = req.file.map(PathBuf::from);
    let robot_addr = req.robot_addr.clone();

    let result = tokio::task::spawn_blocking(move || execute_remote(&robot_addr, file)).await;

    match result {
        Ok(Ok(_)) => (
            StatusCode::OK,
            serde_json::json!({
                "success": true,
                "message": format!("Successfully deployed to {}", req.robot_addr)
            })
            .to_string(),
        ),
        Ok(Err(e)) => (
            StatusCode::BAD_REQUEST,
            serde_json::json!({
                "success": false,
                "error": e.to_string()
            })
            .to_string(),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({
                "success": false,
                "error": format!("Task failed: {}", e)
            })
            .to_string(),
        ),
    }
}

// Remote deployments list handler
#[derive(serde::Deserialize)]
struct RobotRequest {
    robot_addr: String,
}

async fn remote_deployments_handler(Json(req): Json<RobotRequest>) -> impl IntoResponse {
    let url = normalize_daemon_url(&req.robot_addr, "/deployments");

    match reqwest::get(&url).await {
        Ok(response) => match response.json::<serde_json::Value>().await {
            Ok(data) => (StatusCode::OK, serde_json::to_string(&data).unwrap()),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({
                    "success": false,
                    "error": format!("Failed to parse response: {}", e)
                })
                .to_string(),
            ),
        },
        Err(e) => (
            StatusCode::BAD_REQUEST,
            serde_json::json!({
                "success": false,
                "error": format!("Failed to connect to robot: {}", e)
            })
            .to_string(),
        ),
    }
}

// Remote hardware info handler
async fn remote_hardware_handler(Json(req): Json<RobotRequest>) -> impl IntoResponse {
    let url = normalize_daemon_url(&req.robot_addr, "/hardware");

    match reqwest::get(&url).await {
        Ok(response) => match response.json::<serde_json::Value>().await {
            Ok(data) => (StatusCode::OK, serde_json::to_string(&data).unwrap()),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({
                    "success": false,
                    "error": format!("Failed to parse response: {}", e)
                })
                .to_string(),
            ),
        },
        Err(e) => (
            StatusCode::BAD_REQUEST,
            serde_json::json!({
                "success": false,
                "error": format!("Failed to connect to robot: {}", e)
            })
            .to_string(),
        ),
    }
}

// Remote stop deployment handler
#[derive(serde::Deserialize)]
struct StopRequest {
    robot_addr: String,
    deployment_id: String,
}

async fn remote_stop_handler(Json(req): Json<StopRequest>) -> impl IntoResponse {
    let url = normalize_daemon_url(
        &req.robot_addr,
        &format!("/deployments/{}/stop", req.deployment_id),
    );

    let client = reqwest::Client::new();
    match client.post(&url).send().await {
        Ok(response) => match response.json::<serde_json::Value>().await {
            Ok(data) => (StatusCode::OK, serde_json::to_string(&data).unwrap()),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({
                    "success": false,
                    "error": format!("Failed to parse response: {}", e)
                })
                .to_string(),
            ),
        },
        Err(e) => (
            StatusCode::BAD_REQUEST,
            serde_json::json!({
                "success": false,
                "error": format!("Failed to stop deployment: {}", e)
            })
            .to_string(),
        ),
    }
}

fn normalize_daemon_url(addr: &str, path: &str) -> String {
    let base = if addr.starts_with("http://") || addr.starts_with("https://") {
        addr.to_string()
    } else {
        format!("http://{}", addr)
    };

    // Add port if not present
    let base_with_port = if !base.contains(":808") && !base.contains("/") {
        format!("{}:8080", base)
    } else {
        base
    };

    format!("{}{}", base_with_port, path)
}

// === Parameter Management Handlers ===

/// List all parameters
async fn params_list_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let params_map = state.params.get_all();

    let params_list: Vec<_> = params_map
        .iter()
        .map(|(key, value)| {
            serde_json::json!({
                "key": key,
                "value": value,
                "type": match value {
                    serde_json::Value::Number(_) => "number",
                    serde_json::Value::String(_) => "string",
                    serde_json::Value::Bool(_) => "boolean",
                    serde_json::Value::Array(_) => "array",
                    serde_json::Value::Object(_) => "object",
                    serde_json::Value::Null => "null",
                }
            })
        })
        .collect();

    (
        StatusCode::OK,
        serde_json::json!({
            "success": true,
            "params": params_list,
            "count": params_list.len()
        })
        .to_string(),
    )
}

/// Get a specific parameter
async fn params_get_handler(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    match state.params.get::<serde_json::Value>(&key) {
        Some(value) => (
            StatusCode::OK,
            serde_json::json!({
                "success": true,
                "key": key,
                "value": value
            })
            .to_string(),
        ),
        None => (
            StatusCode::NOT_FOUND,
            serde_json::json!({
                "success": false,
                "error": format!("Parameter '{}' not found", key)
            })
            .to_string(),
        ),
    }
}

#[derive(serde::Deserialize)]
struct SetParamRequest {
    value: serde_json::Value,
}

/// Set a parameter
async fn params_set_handler(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
    Json(req): Json<SetParamRequest>,
) -> impl IntoResponse {
    match state.params.set(&key, req.value.clone()) {
        Ok(_) => {
            // Save to disk
            let _ = state.params.save_to_disk();

            (
                StatusCode::OK,
                serde_json::json!({
                    "success": true,
                    "message": format!("Parameter '{}' updated", key),
                    "key": key,
                    "value": req.value
                })
                .to_string(),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({
                "success": false,
                "error": e.to_string()
            })
            .to_string(),
        ),
    }
}

/// Delete a parameter
async fn params_delete_handler(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    match state.params.remove(&key) {
        Some(old_value) => {
            // Save to disk
            let _ = state.params.save_to_disk();

            (
                StatusCode::OK,
                serde_json::json!({
                    "success": true,
                    "message": format!("Parameter '{}' deleted", key),
                    "key": key,
                    "old_value": old_value
                })
                .to_string(),
            )
        }
        None => (
            StatusCode::NOT_FOUND,
            serde_json::json!({
                "success": false,
                "error": format!("Parameter '{}' not found", key)
            })
            .to_string(),
        ),
    }
}

/// Export all parameters
async fn params_export_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let params = state.params.get_all();

    match serde_yaml::to_string(&params) {
        Ok(yaml) => (
            StatusCode::OK,
            serde_json::json!({
                "success": true,
                "format": "yaml",
                "data": yaml
            })
            .to_string(),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({
                "success": false,
                "error": e.to_string()
            })
            .to_string(),
        ),
    }
}

#[derive(serde::Deserialize)]
struct ImportParamsRequest {
    data: String,
    format: String, // "yaml" or "json"
}

/// Import parameters
async fn params_import_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ImportParamsRequest>,
) -> impl IntoResponse {
    let import_result: Result<
        std::collections::BTreeMap<String, serde_json::Value>,
        Box<dyn std::error::Error>,
    > =
        match req.format.as_str() {
            "yaml" => serde_yaml::from_str(&req.data)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>),
            "json" => serde_json::from_str(&req.data)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>),
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    serde_json::json!({
                        "success": false,
                        "error": "Invalid format. Use 'yaml' or 'json'"
                    })
                    .to_string(),
                );
            }
        };

    match import_result {
        Ok(params_map) => {
            let mut count = 0;
            for (key, value) in params_map {
                if state.params.set(&key, value).is_ok() {
                    count += 1;
                }
            }

            // Save to disk
            let _ = state.params.save_to_disk();

            (
                StatusCode::OK,
                serde_json::json!({
                    "success": true,
                    "message": format!("Imported {} parameters", count),
                    "count": count
                })
                .to_string(),
            )
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            serde_json::json!({
                "success": false,
                "error": format!("Failed to parse {}: {}", req.format, e)
            })
            .to_string(),
        ),
    }
}

// WebSocket handler for real-time updates
async fn websocket_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_websocket)
}

async fn handle_websocket(socket: WebSocket) {
    let (mut sender, _receiver) = socket.split();

    // Stream updates every 50ms (20 FPS) for real-time monitoring
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(50));

    loop {
        interval.tick().await;

        // Gather all data in parallel
        let (nodes_result, topics_result, graph_result) = tokio::join!(
            tokio::task::spawn_blocking(|| {
                crate::commands::monitor::discover_nodes()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|n| {
                        serde_json::json!({
                            "name": n.name,
                            "pid": n.process_id,
                            "status": n.status,
                            "health": n.health.as_str(),
                            "health_color": n.health.color(),
                            "cpu": format!("{:.1}%", n.cpu_usage),
                            "memory": format!("{} MB", n.memory_usage / 1024 / 1024),
                        })
                    })
                    .collect::<Vec<_>>()
            }),
            tokio::task::spawn_blocking(|| {
                crate::commands::monitor::discover_shared_memory()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|t| {
                        serde_json::json!({
                            "name": t.topic_name,
                            "size": format!("{} KB", t.size_bytes / 1024),
                            "active": t.active,
                            "processes": t.accessing_processes.len(),
                        })
                    })
                    .collect::<Vec<_>>()
            }),
            tokio::task::spawn_blocking(|| {
                let (nodes, edges) = crate::graph::discover_graph_data();
                (nodes, edges)
            })
        );

        // Unwrap results
        let nodes = nodes_result.unwrap_or_default();
        let topics = topics_result.unwrap_or_default();
        let (graph_nodes, graph_edges) = graph_result.unwrap_or_default();

        // Convert graph data
        let graph_nodes_json = graph_nodes
            .into_iter()
            .map(|n| {
                serde_json::json!({
                    "id": n.id,
                    "label": n.label,
                    "type": match n.node_type {
                        crate::graph::NodeType::Process => "process",
                        crate::graph::NodeType::Topic => "topic",
                    },
                    "pid": n.pid,
                    "active": n.active,
                })
            })
            .collect::<Vec<_>>();

        let graph_edges_json = graph_edges
            .into_iter()
            .map(|e| {
                serde_json::json!({
                    "from": e.from,
                    "to": e.to,
                    "type": match e.edge_type {
                        crate::graph::EdgeType::Publish => "publish",
                        crate::graph::EdgeType::Subscribe => "subscribe",
                    },
                    "active": e.active,
                })
            })
            .collect::<Vec<_>>();

        // Build update message
        let update = serde_json::json!({
            "type": "update",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "data": {
                "nodes": nodes,
                "topics": topics,
                "graph": {
                    "nodes": graph_nodes_json,
                    "edges": graph_edges_json
                }
            }
        });

        // Send to client
        if sender
            .send(Message::Text(update.to_string()))
            .await
            .is_err()
        {
            break; // Client disconnected
        }
    }
}

fn generate_html(port: u16) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>HORUS Dashboard</title>
    <style>
        @import url('https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600;700&display=swap');

        :root {{
            --primary: #0F172A;
            --accent: #00D4FF;
            --success: #00FF88;
            --gray: #64748B;
            --dark-bg: #0A0B0D;
            --card-bg: #16181C;
            --surface: #16181C;
            --surface-hover: #1F2229;
            --border: rgba(0, 212, 255, 0.1);
            --text-primary: #E2E8F0;
            --text-secondary: #94A3B8;
            --text-tertiary: #64748B;
        }}

        /* Light theme variables */
        [data-theme="light"] {{
            --primary: #1E293B;
            --accent: #0369A1;
            --success: #059669;
            --gray: #64748B;
            --dark-bg: #F8FAFC;
            --card-bg: #FFFFFF;
            --surface: #FFFFFF;
            --surface-hover: #F1F5F9;
            --border: rgba(3, 105, 161, 0.2);
            --text-primary: #1E293B;
            --text-secondary: #475569;
            --text-tertiary: #64748B;
        }}

        [data-theme="light"] body {{
            background-image: repeating-linear-gradient(
                0deg,
                transparent,
                transparent 2px,
                rgba(3, 105, 161, 0.05) 2px,
                rgba(3, 105, 161, 0.05) 4px
            );
        }}

        [data-theme="light"] .logo h1 {{
            background: linear-gradient(135deg, #0369A1, #EA580C);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
        }}

        [data-theme="light"] .status-value {{
            color: #EA580C;
            text-shadow: 0 0 10px rgba(0, 0, 0, 0.4), 0 0 20px rgba(0, 0, 0, 0.2);
        }}

        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}

        body {{
            font-family: 'JetBrains Mono', monospace;
            background-color: var(--dark-bg);
            background-image: repeating-linear-gradient(
                0deg,
                transparent,
                transparent 2px,
                rgba(0, 212, 255, 0.03) 2px,
                rgba(0, 212, 255, 0.03) 4px
            );
            color: var(--text-primary);
            min-height: 100vh;
            animation: scan-bg 8s linear infinite;
            transition: background-color 0.3s ease, color 0.3s ease;
        }}

        @keyframes scan-bg {{
            0% {{ background-position: 0 0; }}
            100% {{ background-position: 0 10px; }}
        }}

        .container {{
            display: flex;
            min-height: 100vh;
            padding: 0;
        }}

        .sidebar {{
            width: 250px;
            background: rgba(22, 24, 28, 0.9);
            backdrop-filter: blur(10px);
            border-right: 1px solid var(--border);
            padding: 2rem 0;
            position: fixed;
            height: 100vh;
            overflow-y: auto;
            transition: background-color 0.3s ease;
        }}

        [data-theme="light"] .sidebar {{
            background: rgba(248, 250, 252, 0.9);
        }}

        .logo {{
            padding: 0 1.5rem;
            margin-bottom: 2rem;
            display: flex;
            align-items: center;
            gap: 0.75rem;
        }}

        .logo h1 {{
            font-size: 1.5rem;
            font-weight: 800;
            background: linear-gradient(135deg, #00D4FF, #00FF88);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
            transition: all 0.3s ease;
        }}

        .main-content {{
            margin-left: 250px;
            flex: 1;
            padding: 2rem;
        }}

        h1 {{
            font-size: 2rem;
            font-weight: 800;
            color: var(--text-primary);
            margin-bottom: 1.5rem;
        }}

        .status-bar {{
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 1.5rem;
            margin-bottom: 2rem;
            display: flex;
            gap: 2rem;
            align-items: center;
        }}

        .status-item {{
            display: flex;
            flex-direction: column;
        }}

        .status-label {{
            color: var(--text-secondary);
            font-size: 0.875rem;
            margin-bottom: 0.5rem;
        }}

        .status-value {{
            color: var(--success);
            font-size: 1.5rem;
            font-weight: 600;
            text-shadow: 0 0 20px var(--success);
        }}

        /* Status item with tooltip */
        .status-item-with-tooltip {{
            position: relative;
            cursor: pointer;
        }}

        .status-item-with-tooltip:hover {{
            background: var(--surface-hover);
            border-radius: 8px;
            padding: 0.5rem;
            margin: -0.5rem;
        }}

        /* Tooltip container */
        .status-tooltip {{
            display: none;
            position: absolute;
            top: 100%;
            left: 50%;
            transform: translateX(-50%);
            margin-top: 0.75rem;
            background: var(--card-bg);
            border: 1px solid var(--accent);
            border-radius: 8px;
            box-shadow: 0 4px 20px rgba(0, 212, 255, 0.2);
            padding: 0;
            min-width: 250px;
            max-width: 350px;
            z-index: 1000;
            animation: tooltipFadeIn 0.2s ease-out;
        }}

        @keyframes tooltipFadeIn {{
            from {{
                opacity: 0;
                transform: translateX(-50%) translateY(-5px);
            }}
            to {{
                opacity: 1;
                transform: translateX(-50%) translateY(0);
            }}
        }}

        .status-item-with-tooltip:hover .status-tooltip {{
            display: block;
        }}

        .tooltip-header {{
            background: var(--accent);
            color: var(--primary);
            padding: 0.75rem 1rem;
            font-weight: 600;
            font-size: 0.875rem;
            border-radius: 8px 8px 0 0;
        }}

        .tooltip-content {{
            padding: 0.75rem;
            max-height: 300px;
            overflow-y: auto;
        }}

        .tooltip-node-item, .tooltip-topic-item {{
            display: flex;
            align-items: center;
            gap: 0.5rem;
            padding: 0.5rem;
            border-radius: 4px;
            margin-bottom: 0.25rem;
            font-size: 0.875rem;
        }}

        .tooltip-node-item:hover, .tooltip-topic-item:hover {{
            background: var(--surface-hover);
        }}

        .tooltip-node-health {{
            width: 8px;
            height: 8px;
            border-radius: 50%;
            flex-shrink: 0;
        }}

        .tooltip-node-health.health-green {{ background: #00FF88; box-shadow: 0 0 8px #00FF88; }}
        .tooltip-node-health.health-yellow {{ background: #FFC107; box-shadow: 0 0 8px #FFC107; }}
        .tooltip-node-health.health-orange {{ background: #FF9800; box-shadow: 0 0 8px #FF9800; }}
        .tooltip-node-health.health-red {{ background: #F44336; box-shadow: 0 0 8px #F44336; }}
        .tooltip-node-health.health-gray {{ background: #9E9E9E; box-shadow: 0 0 8px #9E9E9E; }}

        .tooltip-node-name {{
            color: var(--text-primary);
            font-weight: 500;
            flex: 1;
        }}

        .tooltip-node-status {{
            color: var(--text-secondary);
            font-size: 0.75rem;
        }}

        .tooltip-topic-bullet {{
            color: var(--accent);
            font-weight: bold;
        }}

        .tooltip-topic-name {{
            color: var(--text-primary);
        }}

        .tooltip-loading {{
            color: var(--text-secondary);
            font-style: italic;
            text-align: center;
            padding: 1rem;
        }}

        .grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
            gap: 1.5rem;
            margin-bottom: 2rem;
        }}

        .card {{
            background: var(--surface);
            border: 1px solid rgba(0, 212, 255, 0.2);
            border-radius: 8px;
            padding: 1.5rem;
            transition: all 0.3s;
            position: relative;
            overflow: hidden;
        }}

        .card::before {{
            content: '';
            position: absolute;
            top: 0;
            left: 0;
            width: 100%;
            height: 2px;
            background: linear-gradient(90deg, transparent, #00D4FF, transparent);
            animation: scan 3s linear infinite;
        }}

        @keyframes scan {{
            0% {{ transform: translateX(-100%); }}
            100% {{ transform: translateX(100%); }}
        }}

        .card:hover {{
            transform: translateY(-5px);
            border-color: var(--accent);
            box-shadow: 0 10px 30px var(--border);
        }}

        .card h2 {{
            color: var(--accent);
            font-size: 1.5rem;
            margin-bottom: 1rem;
            border-bottom: 2px solid var(--border);
            padding-bottom: 0.5rem;
        }}

        .placeholder {{
            color: var(--text-secondary);
            font-style: italic;
            padding: 2rem;
            text-align: center;
        }}

        .pulse {{
            display: inline-block;
            width: 8px;
            height: 8px;
            background: var(--success);
            border-radius: 50%;
            animation: pulse 2s ease-in-out infinite;
            margin-right: 0.5rem;
        }}

        @keyframes pulse {{
            0%, 100% {{ opacity: 1; }}
            50% {{ opacity: 0.3; }}
        }}

        .command {{
            background: var(--dark-bg);
            border: 1px solid var(--border);
            border-radius: 4px;
            padding: 1rem;
            font-family: 'JetBrains Mono', monospace;
            color: var(--text-secondary);
            margin-top: 1rem;
            cursor: pointer;
            transition: all 0.2s;
        }}

        .command:hover {{
            border-color: var(--success);
            background: var(--surface-hover);
        }}

        .command-prompt {{
            color: var(--success);
            margin-right: 0.5rem;
        }}

        [data-theme="light"] .command-prompt {{
            color: #EA580C;
            text-shadow: 0 0 8px rgba(0, 0, 0, 0.5), 0 0 16px rgba(0, 0, 0, 0.3);
        }}

        .theme-toggle {{
            position: fixed;
            bottom: 2rem;
            left: 1rem;
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 6px;
            padding: 0.5rem;
            cursor: pointer;
            font-size: 1.5rem;
            transition: all 0.3s;
            z-index: 1001;
            width: 48px;
            height: 48px;
            display: flex;
            align-items: center;
            justify-content: center;
            color: var(--text-secondary);
        }}

        .theme-toggle:hover {{
            background: var(--surface-hover);
            border-color: var(--accent);
            color: var(--accent);
        }}

        /* Help Button */
        .help-button {{
            position: fixed;
            bottom: 2rem;
            left: 5rem;
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 6px;
            padding: 0.5rem;
            cursor: pointer;
            font-size: 1.5rem;
            font-weight: bold;
            transition: all 0.3s;
            z-index: 1001;
            width: 48px;
            height: 48px;
            display: flex;
            align-items: center;
            justify-content: center;
            color: var(--text-secondary);
        }}

        .help-button:hover {{
            background: var(--surface-hover);
            border-color: var(--accent);
            color: var(--accent);
            transform: scale(1.05);
        }}

        /* Help Modal */
        .help-modal {{
            display: none;
            position: fixed;
            z-index: 2000;
            left: 0;
            top: 0;
            width: 100%;
            height: 100%;
            overflow: auto;
            background-color: rgba(0, 0, 0, 0.7);
            animation: fadeIn 0.3s;
        }}

        .help-modal.active {{
            display: block;
        }}

        @keyframes fadeIn {{
            from {{ opacity: 0; }}
            to {{ opacity: 1; }}
        }}

        .help-modal-content {{
            background-color: var(--card-bg);
            margin: 3% auto;
            border: 1px solid var(--border);
            border-radius: 12px;
            width: 90%;
            max-width: 900px;
            max-height: 85vh;
            display: flex;
            flex-direction: column;
            animation: slideDown 0.3s;
        }}

        @keyframes slideDown {{
            from {{
                transform: translateY(-50px);
                opacity: 0;
            }}
            to {{
                transform: translateY(0);
                opacity: 1;
            }}
        }}

        .help-modal-header {{
            background: linear-gradient(135deg, var(--accent), #00A8CC);
            color: var(--primary);
            padding: 1.5rem 2rem;
            border-radius: 12px 12px 0 0;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }}

        .help-modal-header h2 {{
            margin: 0;
            font-size: 1.75rem;
            font-weight: 700;
        }}

        .help-close {{
            background: none;
            border: none;
            color: var(--primary);
            font-size: 2.5rem;
            cursor: pointer;
            transition: transform 0.2s;
            padding: 0;
            width: 40px;
            height: 40px;
            display: flex;
            align-items: center;
            justify-content: center;
        }}

        .help-close:hover {{
            transform: rotate(90deg);
        }}

        .help-modal-body {{
            padding: 2rem;
            overflow-y: auto;
            flex: 1;
        }}

        .help-section {{
            margin-bottom: 2.5rem;
        }}

        .help-section h3 {{
            color: var(--accent);
            margin-bottom: 1.5rem;
            font-size: 1.5rem;
            border-bottom: 2px solid var(--border);
            padding-bottom: 0.5rem;
        }}

        .help-item {{
            margin-bottom: 1.5rem;
            padding: 1rem;
            background: var(--surface);
            border-radius: 8px;
            border-left: 3px solid var(--accent);
        }}

        .help-status {{
            display: flex;
            align-items: center;
            gap: 0.75rem;
            margin-bottom: 0.5rem;
        }}

        .status-dot {{
            width: 12px;
            height: 12px;
            border-radius: 50%;
            flex-shrink: 0;
        }}

        .status-dot.health-green {{ background: #00FF88; box-shadow: 0 0 10px #00FF88; }}
        .status-dot.health-yellow {{ background: #FFC107; box-shadow: 0 0 10px #FFC107; }}
        .status-dot.health-orange {{ background: #FF9800; box-shadow: 0 0 10px #FF9800; }}
        .status-dot.health-red {{ background: #F44336; box-shadow: 0 0 10px #F44336; }}
        .status-dot.health-gray {{ background: #9E9E9E; box-shadow: 0 0 10px #9E9E9E; }}

        .help-item strong {{
            color: var(--text-primary);
            font-size: 1.1rem;
        }}

        .help-item p {{
            color: var(--text-secondary);
            margin: 0.5rem 0;
        }}

        .help-item ul, .help-item ol {{
            margin: 0.75rem 0;
            padding-left: 1.5rem;
        }}

        .help-item li {{
            color: var(--text-secondary);
            margin: 0.5rem 0;
        }}

        .help-item code {{
            background: var(--dark-bg);
            border: 1px solid var(--border);
            padding: 0.25rem 0.5rem;
            border-radius: 4px;
            font-family: 'JetBrains Mono', monospace;
            font-size: 0.9rem;
            color: var(--accent);
        }}

        .help-item kbd {{
            background: var(--surface-hover);
            border: 1px solid var(--border);
            border-radius: 4px;
            padding: 0.25rem 0.5rem;
            font-family: 'JetBrains Mono', monospace;
            font-size: 0.9rem;
            color: var(--accent);
            box-shadow: 0 2px 0 var(--border);
        }}

        .nav {{
            list-style: none;
            padding: 0;
            margin: 0;
        }}

        .nav-item {{
            display: block;
            width: 100%;
            padding: 1rem 1.5rem;
            background: transparent;
            border: none;
            border-left: 3px solid transparent;
            color: var(--text-secondary);
            cursor: pointer;
            font-family: 'Courier New', monospace;
            font-size: 1rem;
            transition: all 0.3s;
            text-align: left;
        }}

        .nav-item:hover {{
            background: var(--dark-bg);
            color: var(--accent);
            border-left-color: var(--accent);
        }}

        .nav-item.active {{
            background: var(--dark-bg);
            color: var(--accent);
            border-left-color: var(--accent);
        }}

        .tab-content {{
            display: none;
        }}

        .tab-content.active {{
            display: block;
        }}

        .view-selector {{
            display: flex;
            gap: 1rem;
            margin-bottom: 1.5rem;
            padding: 0.5rem;
            background: var(--surface);
            border-radius: 8px;
            border: 1px solid var(--border);
        }}

        .view-btn {{
            padding: 0.75rem 1.5rem;
            background: transparent;
            border: none;
            color: var(--text-secondary);
            font-size: 0.875rem;
            font-weight: 600;
            cursor: pointer;
            border-radius: 6px;
            transition: all 0.3s ease;
            font-family: 'JetBrains Mono', monospace;
        }}

        .view-btn:hover {{
            background: var(--surface-hover);
            color: var(--accent);
        }}

        .view-btn.active {{
            background: var(--accent);
            color: var(--primary);
            box-shadow: 0 0 20px rgba(0, 212, 255, 0.3);
        }}

        .refresh-btn {{
            margin-left: auto;
            background: rgba(0, 212, 255, 0.1);
            border: 1px solid var(--accent);
            color: var(--accent);
        }}

        .refresh-btn:hover {{
            background: rgba(0, 212, 255, 0.2);
            box-shadow: 0 0 15px rgba(0, 212, 255, 0.2);
        }}

        .monitor-view {{
            display: none;
        }}

        .monitor-view.active {{
            display: flex;
            flex-direction: column;
            height: calc(100vh - 200px);
        }}

        .graph-card {{
            display: flex;
            flex-direction: column;
            height: 100%;
        }}

        .graph-card h2 {{
            margin-bottom: 1rem;
            flex-shrink: 0;
        }}

        .graph-card canvas {{
            flex: 1;
            min-height: 0;
        }}

        .remote-view-btn {{
            padding: 0.75rem 1.5rem;
            background: transparent;
            border: none;
            color: var(--text-secondary);
            font-size: 0.875rem;
            font-weight: 600;
            cursor: pointer;
            border-radius: 6px;
            transition: all 0.3s ease;
            font-family: 'JetBrains Mono', monospace;
        }}

        .remote-view-btn:hover {{
            background: var(--surface-hover);
            color: var(--accent);
        }}

        .remote-view-btn.active {{
            background: var(--accent);
            color: var(--primary);
            box-shadow: 0 0 20px rgba(0, 212, 255, 0.3);
        }}

        .remote-view {{
            display: none;
        }}

        .remote-view.active {{
            display: block;
        }}

        .node-item, .topic-item {{
            background: var(--dark-bg);
            border: 1px solid var(--border);
            border-radius: 4px;
            padding: 1rem;
            margin-bottom: 0.5rem;
            cursor: pointer;
            transition: all 0.3s ease;
            user-select: none;
        }}

        .node-item:hover, .topic-item:hover {{
            border-color: var(--accent);
            background: var(--surface);
            box-shadow: 2px 0 8px rgba(0, 212, 255, 0.3);
        }}

        .node-header, .topic-header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 0.5rem;
        }}

        .node-name, .topic-name {{
            color: var(--accent);
            font-weight: 600;
        }}

        .node-status {{
            padding: 0.25rem 0.75rem;
            border-radius: 4px;
            font-size: 0.875rem;
        }}

        .status-running {{
            background: rgba(0, 255, 136, 0.2);
            color: var(--success);
        }}

        /* Health status colors */
        .status-green {{
            background: rgba(0, 255, 136, 0.2);
            color: #00FF88;
        }}
        .status-yellow {{
            background: rgba(255, 193, 7, 0.2);
            color: #FFC107;
        }}
        .status-orange {{
            background: rgba(255, 152, 0, 0.2);
            color: #FF9800;
        }}
        .status-red {{
            background: rgba(244, 67, 54, 0.2);
            color: #F44336;
        }}
        .status-gray {{
            background: rgba(158, 158, 158, 0.2);
            color: #9E9E9E;
        }}

        .node-details, .topic-details {{
            display: flex;
            gap: 1.5rem;
            font-size: 0.875rem;
            color: var(--text-secondary);
        }}

        /* Mobile Responsive Styles */
        @media (max-width: 768px) {{
            .container {{
                flex-direction: column;
            }}

            .sidebar {{
                width: 100%;
                height: auto;
                position: relative;
                border-right: none;
                border-bottom: 1px solid var(--border);
                padding: 1rem 0;
            }}

            .logo {{
                padding: 0 1rem;
                margin-bottom: 1rem;
            }}

            .logo h1 {{
                font-size: 1.25rem;
            }}

            .nav {{
                display: flex;
                flex-direction: row;
                overflow-x: auto;
                padding: 0 1rem;
                gap: 0.5rem;
            }}

            .nav li {{
                margin: 0;
            }}

            .nav-item {{
                white-space: nowrap;
                padding: 0.75rem 1rem;
                font-size: 0.875rem;
            }}

            .main-content {{
                margin-left: 0;
                padding: 1rem;
            }}

            .status-bar {{
                flex-wrap: wrap;
                gap: 1rem;
                padding: 1rem;
            }}

            .status-item {{
                flex: 1;
                min-width: calc(50% - 0.5rem);
            }}

            .grid {{
                grid-template-columns: 1fr;
                gap: 1rem;
            }}

            .view-selector {{
                padding: 0.25rem;
                gap: 0.5rem;
            }}

            .view-btn {{
                padding: 0.5rem 1rem;
                font-size: 0.75rem;
            }}

            /* Graph canvas height now managed by flexbox */

            .card {{
                padding: 1rem;
            }}

            .card h2 {{
                font-size: 1.25rem;
            }}

            .node-details, .topic-details {{
                flex-direction: column;
                gap: 0.5rem;
            }}
        }}

        /* iPhone specific optimizations */
        @media (max-width: 430px) {{
            .logo h1 {{
                font-size: 1rem;
            }}

            .status-item {{
                min-width: 100%;
            }}

            .status-value {{
                font-size: 1.25rem;
            }}

            .nav-item {{
                padding: 0.5rem 0.75rem;
                font-size: 0.75rem;
            }}

            .view-btn {{
                padding: 0.5rem 0.75rem;
                font-size: 0.7rem;
            }}

            /* Graph canvas height now managed by flexbox */
        }}

        /* Log Panel - Slides from right */
        .log-panel {{
            position: fixed;
            top: 0;
            right: -500px;
            width: 500px;
            height: 100vh;
            background: var(--surface);
            border-left: 2px solid var(--border);
            box-shadow: -5px 0 20px rgba(0, 0, 0, 0.5);
            transition: right 0.15s ease-out;
            z-index: 1000;
            display: flex;
            flex-direction: column;
            pointer-events: none;
        }}

        .log-panel.open {{
            right: 0;
            pointer-events: auto;
        }}

        .log-panel-header {{
            padding: 1.5rem;
            border-bottom: 1px solid var(--border);
            display: flex;
            justify-content: space-between;
            align-items: center;
            background: var(--dark-bg);
        }}

        .log-panel-title {{
            font-size: 1.2rem;
            font-weight: 600;
            color: var(--accent);
        }}

        .log-panel-close {{
            background: transparent;
            border: 1px solid var(--border);
            color: var(--text-secondary);
            padding: 0.5rem 1rem;
            border-radius: 4px;
            cursor: pointer;
            transition: all 0.3s ease;
        }}

        .log-panel-close:hover {{
            border-color: var(--accent);
            color: var(--accent);
        }}

        .log-panel-content {{
            flex: 1;
            overflow-y: auto;
            padding: 1rem;
        }}

        .log-entry {{
            background: var(--dark-bg);
            border: 1px solid var(--border);
            border-radius: 4px;
            padding: 0.75rem;
            margin-bottom: 0.5rem;
            font-size: 0.85rem;
        }}

        .log-entry-header {{
            display: flex;
            justify-content: space-between;
            margin-bottom: 0.5rem;
            font-size: 0.75rem;
        }}

        .log-timestamp {{
            color: var(--text-tertiary);
        }}

        .log-type {{
            padding: 0.2rem 0.5rem;
            border-radius: 3px;
            font-size: 0.7rem;
            font-weight: 600;
        }}

        .log-type-publish {{ background: rgba(0, 212, 255, 0.2); color: var(--accent); }}
        .log-type-subscribe {{ background: rgba(0, 255, 136, 0.2); color: var(--success); }}
        .log-type-info {{ background: rgba(100, 116, 139, 0.2); color: var(--text-secondary); }}
        .log-type-warning {{ background: rgba(255, 165, 0, 0.2); color: #ffa500; }}
        .log-type-error {{ background: rgba(255, 68, 68, 0.2); color: #ff4444; }}
        .log-type-topicread {{ background: rgba(0, 255, 136, 0.2); color: var(--success); }}
        .log-type-topicwrite {{ background: rgba(0, 212, 255, 0.2); color: var(--accent); }}
        .log-type-topicmap {{ background: rgba(138, 43, 226, 0.2); color: #c792ea; }}
        .log-type-topicunmap {{ background: rgba(255, 136, 0, 0.2); color: #ff8800; }}

        .log-message {{
            color: var(--text-primary);
            margin-top: 0.5rem;
            word-break: break-word;
        }}

        @media (max-width: 768px) {{
            .log-panel {{
                width: 100%;
                right: -100%;
            }}
        }}

        /* Install Dialog Modal */
        .install-dialog {{
            display: none;
            position: fixed;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            background: rgba(0, 0, 0, 0.7);
            z-index: 2000;
            align-items: center;
            justify-content: center;
        }}

        .install-dialog.active {{
            display: flex;
        }}

        .install-dialog-content {{
            background: var(--surface);
            border: 2px solid var(--accent);
            border-radius: 12px;
            width: 90%;
            max-width: 500px;
            max-height: 80vh;
            display: flex;
            flex-direction: column;
            box-shadow: 0 10px 50px rgba(0, 212, 255, 0.3);
        }}

        .install-dialog-header {{
            padding: 1.5rem;
            border-bottom: 1px solid var(--border);
            display: flex;
            justify-content: space-between;
            align-items: center;
        }}

        .install-dialog-header h3 {{
            color: var(--accent);
            margin: 0;
            font-size: 1.3rem;
        }}

        .install-dialog-body {{
            padding: 1.5rem;
            overflow-y: auto;
        }}

        .install-dialog-footer {{
            padding: 1rem 1.5rem;
            border-top: 1px solid var(--border);
            display: flex;
            gap: 10px;
            justify-content: flex-end;
        }}

        .install-option {{
            padding: 1rem;
            background: var(--dark-bg);
            border: 2px solid var(--border);
            border-radius: 8px;
            cursor: pointer;
            transition: all 0.3s;
            margin-bottom: 10px;
        }}

        .install-option:hover {{
            border-color: var(--accent);
            background: var(--primary);
        }}

        .install-option.selected {{
            border-color: var(--accent);
            background: var(--primary);
        }}

        .install-option input[type="radio"] {{
            margin-right: 10px;
        }}

        .local-packages-select {{
            width: 100%;
            padding: 10px;
            background: var(--dark-bg);
            border: 1px solid var(--border);
            border-radius: 6px;
            color: var(--text-primary);
            font-family: 'JetBrains Mono', monospace;
            margin-top: 10px;
        }}
    </style>
</head>
<body>
    <button class="theme-toggle" onclick="toggleTheme()" id="theme-toggle">
        🌙
    </button>

    <button class="help-button" onclick="toggleHelp()" id="help-button" title="Help (Press ?)">
        ?
    </button>

    <!-- Help Modal -->
    <div class="help-modal" id="help-modal">
        <div class="help-modal-content">
            <div class="help-modal-header">
                <h2>HORUS Dashboard Guide</h2>
                <button class="help-close" onclick="toggleHelp()">&times;</button>
            </div>
            <div class="help-modal-body">
                <!-- Health Status Section -->
                <div class="help-section">
                    <h3> Node Health Statuses</h3>
                    <div class="help-item">
                        <div class="help-status">
                            <span class="status-dot health-green"></span>
                            <strong>Healthy</strong>
                        </div>
                        <p>Node operating normally with no issues</p>
                        <ul>
                            <li>No errors detected</li>
                            <li>Fast execution (< 100ms per tick)</li>
                            <li>All systems functioning as expected</li>
                        </ul>
                    </div>

                    <div class="help-item">
                        <div class="help-status">
                            <span class="status-dot health-yellow"></span>
                            <strong>Warning</strong>
                        </div>
                        <p>Performance degraded, attention recommended</p>
                        <ul>
                            <li>Slow tick execution (> 100ms)</li>
                            <li>Missed deadlines or timing issues</li>
                            <li>System still functional but not optimal</li>
                        </ul>
                    </div>

                    <div class="help-item">
                        <div class="help-status">
                            <span class="status-dot health-orange"></span>
                            <strong>Error</strong>
                        </div>
                        <p>Errors occurring, investigation needed</p>
                        <ul>
                            <li>3-10 errors have occurred</li>
                            <li>Node still running but unreliable</li>
                            <li>Check logs for error details</li>
                        </ul>
                    </div>

                    <div class="help-item">
                        <div class="help-status">
                            <span class="status-dot health-red"></span>
                            <strong>Critical</strong>
                        </div>
                        <p>Severe issues - immediate action required</p>
                        <ul>
                            <li>10+ errors detected</li>
                            <li>Node may crash or become unresponsive</li>
                            <li>System stability at risk</li>
                        </ul>
                    </div>

                    <div class="help-item">
                        <div class="help-status">
                            <span class="status-dot health-gray"></span>
                            <strong>Unknown</strong>
                        </div>
                        <p>Unable to determine health status</p>
                        <ul>
                            <li>No heartbeat received (> 5 seconds)</li>
                            <li>Node may be frozen or deadlocked</li>
                            <li>Process might need restart</li>
                        </ul>
                    </div>
                </div>

                <!-- Dashboard Features Section -->
                <div class="help-section">
                    <h3>[#] Dashboard Features</h3>

                    <div class="help-item">
                        <strong>Status Bar</strong>
                        <p>Top bar showing system overview:</p>
                        <ul>
                            <li><strong>Active Nodes</strong> - Hover to see node list with health indicators</li>
                            <li><strong>Active Topics</strong> - Hover to see all topic names</li>
                            <li><strong>Port</strong> - Dashboard server port number</li>
                        </ul>
                    </div>

                    <div class="help-item">
                        <strong>Monitor Tab</strong>
                        <p>View running nodes and topics</p>
                        <ul>
                            <li><strong>List View</strong> - Detailed list of nodes/topics with stats</li>
                            <li><strong>Graph View</strong> - Visual network topology showing connections</li>
                            <li><strong>Click nodes/topics</strong> - View detailed logs</li>
                        </ul>
                    </div>

                    <div class="help-item">
                        <strong>Parameters Tab</strong>
                        <p>Manage runtime parameters</p>
                        <ul>
                            <li>Add, edit, or delete parameters</li>
                            <li>Export parameters to YAML/JSON</li>
                            <li>Import parameter configurations</li>
                        </ul>
                    </div>

                    <div class="help-item">
                        <strong>Packages Tab</strong>
                        <p>Manage HORUS packages</p>
                        <ul>
                            <li><strong>Search</strong> - Find packages in registry</li>
                            <li><strong>Global</strong> - View globally installed packages</li>
                            <li><strong>Local</strong> - View local environment packages</li>
                            <li><strong>Registry</strong> - Search and install packages</li>
                        </ul>
                    </div>

                    <div class="help-item">
                        <strong>Remote Tab</strong>
                        <p>Deploy and manage remote robots</p>
                        <ul>
                            <li>Connect to robot hardware</li>
                            <li>Deploy nodes remotely</li>
                            <li>Monitor remote deployments</li>
                            <li>View hardware information</li>
                        </ul>
                    </div>
                </div>

                <!-- Tips Section -->
                <div class="help-section">
                    <h3> Tips & Tricks</h3>

                    <div class="help-item">
                        <strong>Real-time Updates</strong>
                        <p>Dashboard updates automatically via WebSocket (20 FPS). No refresh needed!</p>
                    </div>

                    <div class="help-item">
                        <strong>View Logs</strong>
                        <p>Click any node or topic in the Monitor tab to see detailed logs</p>
                    </div>

                    <div class="help-item">
                        <strong>Health Indicators</strong>
                        <p>Colored dots show node health at a glance. Hover over "Active Nodes" for quick status check</p>
                    </div>

                    <div class="help-item">
                        <strong>Dark/Light Theme</strong>
                        <p>Toggle between themes using the moon/sun button (bottom left)</p>
                    </div>
                </div>

                <!-- Keyboard Shortcuts Section -->
                <div class="help-section">
                    <h3>⌨️ Keyboard Shortcuts</h3>

                    <div class="help-item">
                        <ul>
                            <li><kbd>?</kbd> - Open/close this help guide</li>
                            <li><kbd>Esc</kbd> - Close help modal</li>
                        </ul>
                    </div>
                </div>

                <!-- Getting Started Section -->
                <div class="help-section">
                    <h3> Getting Started</h3>

                    <div class="help-item">
                        <strong>Running Your First Node</strong>
                        <ol>
                            <li>Create a HORUS node file (e.g., <code>my_node.rs</code>)</li>
                            <li>Run: <code>horus run my_node.rs</code></li>
                            <li>Watch it appear in the Monitor tab with health status</li>
                            <li>Click the node to view logs in real-time</li>
                        </ol>
                    </div>
                </div>
            </div>
        </div>
    </div>

    <div class="container">
        <!-- Left Sidebar Navigation -->
        <nav class="sidebar">
            <div class="logo">
                <h1>HORUS DASHBOARD</h1>
            </div>

            <ul class="nav">
                <li><button class="nav-item active" onclick="switchTab('monitor')">Monitor</button></li>
                <li><button class="nav-item" onclick="switchTab('params')">Parameters</button></li>
                <li><button class="nav-item" onclick="switchTab('packages')">Packages</button></li>
                <li><button class="nav-item" onclick="switchTab('remote')">Remote</button></li>
            </ul>
        </nav>

        <!-- Main Content Area -->
        <div class="main-content">
            <div class="status-bar">
                <div class="status-item status-item-with-tooltip" id="nodes-status-item">
                    <div class="status-label">Active Nodes</div>
                    <div class="status-value">
                        <span class="pulse"></span>
                        <span id="node-count">0</span>
                    </div>
                    <!-- Tooltip for node list -->
                    <div class="status-tooltip" id="nodes-tooltip">
                        <div class="tooltip-header">Active Nodes</div>
                        <div class="tooltip-content" id="nodes-tooltip-content">
                            <div class="tooltip-loading">No nodes running</div>
                        </div>
                    </div>
                </div>
                <div class="status-item status-item-with-tooltip" id="topics-status-item">
                    <div class="status-label">Active Topics</div>
                    <div class="status-value">
                        <span id="topic-count">0</span>
                    </div>
                    <!-- Tooltip for topic list -->
                    <div class="status-tooltip" id="topics-tooltip">
                        <div class="tooltip-header">Active Topics</div>
                        <div class="tooltip-content" id="topics-tooltip-content">
                            <div class="tooltip-loading">No topics available</div>
                        </div>
                    </div>
                </div>
                <div class="status-item">
                    <div class="status-label">Port</div>
                    <div class="status-value">{port}</div>
                </div>
            </div>

        <!-- Monitor Tab -->
        <div id="tab-monitor" class="tab-content active">
            <!-- View Selector -->
            <div class="view-selector">
                <button class="view-btn active" onclick="switchMonitorView('list')">List View</button>
                <button class="view-btn" onclick="switchMonitorView('graph')">Graph View</button>
                <button class="view-btn refresh-btn" onclick="refreshMonitorData()">Refresh</button>
            </div>

            <!-- List View -->
            <div id="monitor-view-list" class="monitor-view active">
                <div class="grid">
                    <div class="card">
                        <h2>Nodes</h2>
                        <div id="nodes-list"></div>
                    </div>

                    <div class="card">
                        <h2>Topics</h2>
                        <div id="topics-list"></div>
                    </div>
                </div>
            </div>

            <!-- Graph View -->
            <div id="monitor-view-graph" class="monitor-view">
                <div class="card graph-card">
                    <h2>System Graph</h2>
                    <canvas id="graph-canvas" width="1200" height="500" style="width: 100%; height: 100%; background: var(--dark-bg); border-radius: 4px; border: 1px solid var(--border);"></canvas>
                </div>
            </div>
        </div>

        <!-- Parameters Tab -->
        <div id="tab-params" class="tab-content">
            <div class="card">
                <h2>Runtime Parameters</h2>

                <!-- Actions Bar -->
                <div style="display: flex; gap: 1rem; margin-bottom: 1.5rem; flex-wrap: wrap;">
                    <input
                        type="text"
                        id="param-search"
                        placeholder="Search parameters..."
                        style="flex: 1; min-width: 200px; padding: 0.5rem 1rem; border: 1px solid var(--border); border-radius: 8px; background: var(--surface); color: var(--text-primary); font-family: 'JetBrains Mono', monospace;"
                    />
                    <button onclick="refreshParams()" style="padding: 0.5rem 1.5rem; background: var(--accent); color: white; border: none; border-radius: 8px; cursor: pointer; font-weight: 600;">
                        Refresh
                    </button>
                    <button onclick="showAddParamDialog()" style="padding: 0.5rem 1.5rem; background: var(--success); color: white; border: none; border-radius: 8px; cursor: pointer; font-weight: 600;">
                        Add Parameter
                    </button>
                    <button onclick="exportParams()" style="padding: 0.5rem 1.5rem; background: var(--warning); color: white; border: none; border-radius: 8px; cursor: pointer; font-weight: 600;">
                        Export
                    </button>
                    <button onclick="showImportDialog()" style="padding: 0.5rem 1.5rem; background: var(--info); color: white; border: none; border-radius: 8px; cursor: pointer; font-weight: 600;">
                        Import
                    </button>
                </div>

                <!-- Parameters Table -->
                <div id="params-container" style="overflow-x: auto;">
                    <table style="width: 100%; border-collapse: collapse; font-family: 'JetBrains Mono', monospace; font-size: 0.9rem;">
                        <thead>
                            <tr style="border-bottom: 2px solid var(--border); text-align: left;">
                                <th style="padding: 0.75rem; color: var(--text-secondary); font-weight: 600;">Key</th>
                                <th style="padding: 0.75rem; color: var(--text-secondary); font-weight: 600;">Value</th>
                                <th style="padding: 0.75rem; color: var(--text-secondary); font-weight: 600;">Type</th>
                                <th style="padding: 0.75rem; color: var(--text-secondary); font-weight: 600; text-align: right;">Actions</th>
                            </tr>
                        </thead>
                        <tbody id="params-table-body">
                            <tr>
                                <td colspan="4" style="padding: 2rem; text-align: center; color: var(--text-tertiary);">
                                    Loading parameters...
                                </td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            </div>

            <!-- Add/Edit Parameter Dialog -->
            <div id="param-dialog" style="display: none; position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.7); z-index: 1000; align-items: center; justify-content: center;">
                <div style="background: var(--surface); border-radius: 12px; padding: 2rem; max-width: 500px; width: 90%; border: 1px solid var(--border);">
                    <h3 id="dialog-title" style="margin-top: 0; color: var(--text-primary);">Add Parameter</h3>
                    <div style="margin-bottom: 1rem;">
                        <label style="display: block; margin-bottom: 0.5rem; color: var(--text-secondary); font-weight: 600;">Key</label>
                        <input type="text" id="param-key" placeholder="parameter_name" style="width: 100%; padding: 0.5rem; border: 1px solid var(--border); border-radius: 8px; background: var(--bg); color: var(--text-primary); font-family: 'JetBrains Mono', monospace;" />
                    </div>
                    <div style="margin-bottom: 1rem;">
                        <label style="display: block; margin-bottom: 0.5rem; color: var(--text-secondary); font-weight: 600;">Type</label>
                        <select id="param-type" onchange="updateValueInput()" style="width: 100%; padding: 0.5rem; border: 1px solid var(--border); border-radius: 8px; background: var(--bg); color: var(--text-primary);">
                            <option value="number">Number</option>
                            <option value="string">String</option>
                            <option value="boolean">Boolean</option>
                        </select>
                    </div>
                    <div style="margin-bottom: 1.5rem;">
                        <label style="display: block; margin-bottom: 0.5rem; color: var(--text-secondary); font-weight: 600;">Value</label>
                        <input type="text" id="param-value" placeholder="Enter value" style="width: 100%; padding: 0.5rem; border: 1px solid var(--border); border-radius: 8px; background: var(--bg); color: var(--text-primary); font-family: 'JetBrains Mono', monospace;" />
                    </div>
                    <div style="display: flex; gap: 1rem; justify-content: flex-end;">
                        <button onclick="closeParamDialog()" style="padding: 0.5rem 1.5rem; background: var(--surface); color: var(--text-primary); border: 1px solid var(--border); border-radius: 8px; cursor: pointer; font-weight: 600;">
                            Cancel
                        </button>
                        <button onclick="saveParam()" style="padding: 0.5rem 1.5rem; background: var(--accent); color: white; border: none; border-radius: 8px; cursor: pointer; font-weight: 600;">
                            Save
                        </button>
                    </div>
                </div>
            </div>

            <!-- Import Dialog -->
            <div id="import-dialog" style="display: none; position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.7); z-index: 1000; align-items: center; justify-content: center;">
                <div style="background: var(--surface); border-radius: 12px; padding: 2rem; max-width: 600px; width: 90%; border: 1px solid var(--border);">
                    <h3 style="margin-top: 0; color: var(--text-primary);">Import Parameters</h3>
                    <div style="margin-bottom: 1rem;">
                        <label style="display: block; margin-bottom: 0.5rem; color: var(--text-secondary); font-weight: 600;">Format</label>
                        <select id="import-format" style="width: 100%; padding: 0.5rem; border: 1px solid var(--border); border-radius: 8px; background: var(--bg); color: var(--text-primary);">
                            <option value="yaml">YAML</option>
                            <option value="json">JSON</option>
                        </select>
                    </div>
                    <div style="margin-bottom: 1.5rem;">
                        <label style="display: block; margin-bottom: 0.5rem; color: var(--text-secondary); font-weight: 600;">Data</label>
                        <textarea id="import-data" rows="10" placeholder="Paste YAML or JSON here..." style="width: 100%; padding: 0.5rem; border: 1px solid var(--border); border-radius: 8px; background: var(--bg); color: var(--text-primary); font-family: 'JetBrains Mono', monospace; resize: vertical;"></textarea>
                    </div>
                    <div style="display: flex; gap: 1rem; justify-content: flex-end;">
                        <button onclick="closeImportDialog()" style="padding: 0.5rem 1.5rem; background: var(--surface); color: var(--text-primary); border: 1px solid var(--border); border-radius: 8px; cursor: pointer; font-weight: 600;">
                            Cancel
                        </button>
                        <button onclick="importParams()" style="padding: 0.5rem 1.5rem; background: var(--accent); color: white; border: none; border-radius: 8px; cursor: pointer; font-weight: 600;">
                            Import
                        </button>
                    </div>
                </div>
            </div>
        </div>

        <!-- Packages Tab -->
        <div id="tab-packages" class="tab-content">
            <!-- View Selector -->
            <div class="view-selector">
                <button class="view-btn active" onclick="switchPackageView('global')">Global Env</button>
                <button class="view-btn" onclick="switchPackageView('local')">Local Env</button>
                <button class="view-btn" onclick="switchPackageView('registry')">Registry</button>
            </div>

            <!-- Global Environment View -->
            <div id="package-global" class="package-view active">
                <div class="card">
                    <h2>Global Environment</h2>
                    <div id="global-packages-list">
                        <p style="color: var(--text-secondary);">Loading global packages...</p>
                    </div>
                </div>
            </div>

            <!-- Local Environment View -->
            <div id="package-local" class="package-view" style="display: none;">
                <div class="card">
                    <h2>Local Environments</h2>
                    <div id="local-environments-list">
                        <p style="color: var(--text-secondary);">Loading local environments...</p>
                    </div>
                </div>
            </div>

            <!-- Registry View -->
            <div id="package-registry" class="package-view" style="display: none;">
                <div class="card">
                    <h2> Package Registry</h2>
                    <div style="display: flex; gap: 10px; margin-bottom: 20px;">
                        <input
                            type="text"
                            id="registry-search-input"
                            placeholder="Search registry packages..."
                            style="flex: 1; padding: 10px; background: var(--surface); border: 1px solid var(--border); border-radius: 6px; color: var(--text-primary); font-family: 'JetBrains Mono', monospace;"
                        />
                        <button
                            onclick="searchRegistry()"
                            style="padding: 10px 20px; background: var(--accent); color: var(--primary); border: none; border-radius: 6px; cursor: pointer; font-weight: 600; font-family: 'JetBrains Mono', monospace;"
                        >
                            Search
                        </button>
                    </div>
                    <div id="registry-results">
                        <p style="color: var(--text-secondary);">Search for packages above</p>
                    </div>
                </div>
            </div>
        </div>

        <!-- Remote Tab -->
        <div id="tab-remote" class="tab-content">
            <!-- Robot Connection (always visible) -->
            <div class="card" style="margin-bottom: 20px;">
                <h2>Robot Connection</h2>
                <div style="margin-top: 20px;">
                    <div style="display: flex; gap: 10px; align-items: flex-end;">
                        <div style="flex: 1;">
                            <label style="display: block; color: var(--text-secondary); margin-bottom: 5px; font-size: 0.9em;">
                                Robot Address (IP:Port)
                            </label>
                            <input
                                type="text"
                                id="robot-addr"
                                placeholder="192.168.1.100:8080"
                                style="width: 100%; padding: 10px; background: var(--surface); border: 1px solid var(--border); border-radius: 6px; color: var(--text-primary); font-family: 'JetBrains Mono', monospace;"
                            />
                        </div>
                        <button
                            onclick="connectToRobot()"
                            style="padding: 10px 20px; background: var(--accent); color: var(--primary); border: none; border-radius: 6px; cursor: pointer; font-weight: 600;"
                        >
                            Connect
                        </button>
                    </div>
                    <div id="robot-status" style="margin-top: 10px; color: var(--text-secondary); font-size: 0.9em;"></div>
                </div>
            </div>

            <!-- Remote View Switcher -->
            <div class="card">
                <div style="display: flex; gap: 10px; margin-bottom: 20px;">
                    <button
                        id="btn-remote-deploy"
                        class="remote-view-btn active"
                        onclick="switchRemoteView('deploy')"
                    >
                        Deploy
                    </button>
                    <button
                        id="btn-remote-deployments"
                        class="remote-view-btn"
                        onclick="switchRemoteView('deployments')"
                    >
                        Deployments
                    </button>
                    <button
                        id="btn-remote-hardware"
                        class="remote-view-btn"
                        onclick="switchRemoteView('hardware')"
                    >
                        Hardware
                    </button>
                </div>

                <!-- Deploy View -->
                <div id="remote-view-deploy" class="remote-view active">
                    <h3>Deploy Code</h3>
                    <div style="margin-top: 15px;">
                        <div style="margin-bottom: 15px;">
                            <label style="display: block; color: var(--text-secondary); margin-bottom: 5px; font-size: 0.9em;">
                                Entry File (optional, auto-detected if empty)
                            </label>
                            <input
                                type="text"
                                id="robot-file"
                                placeholder="main.py"
                                style="width: 100%; padding: 10px; background: var(--surface); border: 1px solid var(--border); border-radius: 6px; color: var(--text-primary); font-family: 'JetBrains Mono', monospace;"
                            />
                        </div>
                        <button
                            onclick="deployToRobot()"
                            style="padding: 12px; background: var(--accent); color: var(--primary); border: none; border-radius: 6px; cursor: pointer; font-weight: 600; width: 100%;"
                        >
                            Deploy
                        </button>
                        <div id="deploy-status" style="margin-top: 15px; padding: 15px; border-radius: 6px; display: none;"></div>
                    </div>
                </div>

                <!-- Deployments View -->
                <div id="remote-view-deployments" class="remote-view">
                    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 15px;">
                        <h3>Active Deployments</h3>
                        <button
                            onclick="refreshDeployments()"
                            style="padding: 6px 12px; background: var(--surface); color: var(--text-primary); border: 1px solid var(--border); border-radius: 6px; cursor: pointer; font-size: 0.9em;"
                        >
                            Refresh
                        </button>
                    </div>
                    <div id="deployments-list">
                        <p style="color: var(--text-tertiary); text-align: center; padding: 20px;">Connect to a robot to view deployments</p>
                    </div>
                </div>

                <!-- Hardware View -->
                <div id="remote-view-hardware" class="remote-view">
                    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 15px;">
                        <h3>Hardware</h3>
                        <button
                            onclick="refreshHardware()"
                            style="padding: 6px 12px; background: var(--surface); color: var(--text-primary); border: 1px solid var(--border); border-radius: 6px; cursor: pointer; font-size: 0.9em;"
                        >
                            Refresh
                        </button>
                    </div>
                    <div id="hardware-info">
                        <p style="color: var(--text-tertiary); text-align: center; padding: 20px;">Connect to a robot to view hardware</p>
                    </div>
                </div>
            </div>
        </div>

        </div> <!-- end main-content -->
    </div> <!-- end container -->

    <script>
        // Tab switching
        function switchTab(tabName) {{
            // Hide all tab contents
            document.querySelectorAll('.tab-content').forEach(content => {{
                content.classList.remove('active');
            }});

            // Remove active class from all nav items
            document.querySelectorAll('.nav-item').forEach(item => {{
                item.classList.remove('active');
            }});

            // Show selected tab content
            document.getElementById('tab-' + tabName).classList.add('active');

            // Add active class to clicked nav item
            event.target.classList.add('active');

            // Initialize packages tab if switching to it
            if (tabName === 'packages') {{
                onPackagesTabActivate();
            }}
        }}

        // Switch monitor view (list/graph)
        function switchMonitorView(viewName) {{
            // Hide all monitor views
            document.querySelectorAll('.monitor-view').forEach(view => {{
                view.classList.remove('active');
            }});

            // Remove active class from all view buttons
            document.querySelectorAll('.view-btn').forEach(btn => {{
                btn.classList.remove('active');
            }});

            // Show selected view
            document.getElementById('monitor-view-' + viewName).classList.add('active');

            // Add active class to clicked button
            event.target.classList.add('active');

            // Ensure canvas is properly sized when switching to graph view
            if (viewName === 'graph') {{
                console.log('Switching to graph view - preserving node positions');

                // Ensure canvas is properly sized after becoming visible
                setTimeout(() => {{
                    const canvas = document.getElementById('graph-canvas');
                    if (canvas) {{
                        const container = canvas.parentElement;
                        const rect = canvas.getBoundingClientRect();
                        canvas.width = rect.width || container.clientWidth || 1200;
                        canvas.height = rect.height || container.clientHeight || 600;
                        console.log('Canvas resized to:', canvas.width, 'x', canvas.height);
                        // Re-render graph with new dimensions
                        if (window.graphState && window.graphState.nodes) {{
                            renderGraph(window.graphState.nodes, window.graphState.edges);
                        }}
                    }}
                }}, 100);
            }}
        }}

        // Reset graph layout to default positions
        function resetGraphLayout() {{
            console.log('Resetting graph layout to default positions');
            graphState.nodePositions = {{}};
            graphState.nodeVelocities = {{}};
            graphState.hoveredNode = null;
            graphState.draggedNode = null;
        }}

        // Switch remote view (deploy/deployments/hardware)
        function switchRemoteView(viewName) {{
            // Hide all remote views
            document.querySelectorAll('.remote-view').forEach(view => {{
                view.classList.remove('active');
            }});

            // Remove active class from all remote view buttons
            document.querySelectorAll('.remote-view-btn').forEach(btn => {{
                btn.classList.remove('active');
            }});

            // Show selected view
            document.getElementById('remote-view-' + viewName).classList.add('active');

            // Add active class to clicked button
            event.target.classList.add('active');
        }}

        // Auto-refresh status
        async function updateStatus() {{
            try {{
                const response = await fetch('/api/status');
                const data = await response.json();

                document.getElementById('node-count').textContent = data.nodes;
                document.getElementById('topic-count').textContent = data.topics;
            }} catch (error) {{
                console.error('Failed to fetch status:', error);
            }}
        }}

        // Update nodes tooltip
        async function updateNodesToolTip() {{
            try {{
                const response = await fetch('/api/nodes');
                const data = await response.json();
                const tooltipContent = document.getElementById('nodes-tooltip-content');

                if (data.nodes.length === 0) {{
                    tooltipContent.innerHTML = '<div class="tooltip-loading">No nodes running</div>';
                }} else {{
                    tooltipContent.innerHTML = data.nodes.map(node => `
                        <div class="tooltip-node-item">
                            <div class="tooltip-node-health health-${{node.health_color || 'gray'}}"></div>
                            <div class="tooltip-node-name">${{node.name}}</div>
                            <div class="tooltip-node-status">${{node.health}}</div>
                        </div>
                    `).join('');
                }}
            }} catch (error) {{
                console.error('Failed to update nodes tooltip:', error);
            }}
        }}

        // Update topics tooltip
        async function updateTopicsToolTip() {{
            try {{
                const response = await fetch('/api/topics');
                const data = await response.json();
                const tooltipContent = document.getElementById('topics-tooltip-content');

                if (data.topics.length === 0) {{
                    tooltipContent.innerHTML = '<div class="tooltip-loading">No topics available</div>';
                }} else {{
                    tooltipContent.innerHTML = data.topics.map(topic => `
                        <div class="tooltip-topic-item">
                            <span class="tooltip-topic-bullet">•</span>
                            <span class="tooltip-topic-name">${{topic.name}}</span>
                        </div>
                    `).join('');
                }}
            }} catch (error) {{
                console.error('Failed to update topics tooltip:', error);
            }}
        }}

        // Fetch and display nodes
        async function updateNodes() {{
            try {{
                const response = await fetch('/api/nodes');
                const data = await response.json();
                const nodesList = document.getElementById('nodes-list');

                if (data.nodes.length === 0) {{
                    nodesList.innerHTML = '<div class=\"placeholder\">No active nodes detected.<br><div class=\"command\" style=\"margin-top: 1rem;\"><span class=\"command-prompt\">$</span> horus run your_node.rs</div></div>';
                }} else {{
                    nodesList.innerHTML = data.nodes.map(node => `
                        <div class=\"node-item\" data-node-name=\"${{node.name}}\" title=\"Click to view logs\">
                            <div class=\"node-header\">
                                <span class=\"node-name\">${{node.name}}</span>
                                <span class=\"node-status status-${{node.health_color || 'gray'}}\">${{node.health}}</span>
                            </div>
                            <div class=\"node-details\">
                                <span>PID: ${{node.pid}}</span>
                                <span>CPU: ${{node.cpu}}</span>
                                <span>Memory: ${{node.memory}}</span>
                            </div>
                        </div>
                    `).join('');
                }}
            }} catch (error) {{
                console.error('Failed to fetch nodes:', error);
            }}
        }}

        // Fetch and display topics
        async function updateTopics() {{
            try {{
                const response = await fetch('/api/topics');
                const data = await response.json();
                const topicsList = document.getElementById('topics-list');

                if (data.topics.length === 0) {{
                    topicsList.innerHTML = '<div class=\"placeholder\">No topics available.</div>';
                }} else {{
                    const topicsHtml = data.topics.map(topic => `
                        <div class=\"topic-item\" data-topic-name=\"${{topic.name}}\" title=\"Click to view logs\">
                            <div class=\"topic-header\">
                                <span class=\"topic-name\">${{topic.name}}</span>
                                <span class=\"node-status status-running\">${{topic.active ? 'Active' : 'Inactive'}}</span>
                            </div>
                            <div class=\"topic-details\">
                                <span>Size: ${{topic.size}}</span>
                                <span>Processes: ${{topic.processes}}</span>
                            </div>
                        </div>
                    `).join('');
                    console.log(' Generated topics HTML:', topicsHtml.substring(0, 200));
                    topicsList.innerHTML = topicsHtml;
                    console.log('Updated topics list. First topic element:', topicsList.querySelector('.topic-item'));
                }}
            }} catch (error) {{
                console.error('Failed to fetch topics:', error);
            }}
        }}

        // Interactive graph state (global for access from renderGraph and resize)
        window.graphState = {{
            nodePositions: {{}},
            nodes: [],
            edges: [],
            hoveredNode: null,
            draggedNode: null,
            mouseX: 0,
            mouseY: 0,
            offsetX: 0,
            offsetY: 0,
            isDragging: false,
            dragStartX: 0,
            dragStartY: 0
        }};
        const graphState = window.graphState; // Local alias for backward compatibility

        // Initialize canvas event listeners
        function initGraphInteraction() {{
            const canvas = document.getElementById('graph-canvas');
            if (!canvas) return;

            canvas.addEventListener('mousemove', (e) => {{
                const rect = canvas.getBoundingClientRect();
                const scaleX = canvas.width / rect.width;
                const scaleY = canvas.height / rect.height;
                graphState.mouseX = (e.clientX - rect.left) * scaleX;
                graphState.mouseY = (e.clientY - rect.top) * scaleY;

                // Handle dragging
                if (graphState.draggedNode) {{
                    graphState.nodePositions[graphState.draggedNode].x = graphState.mouseX - graphState.offsetX;
                    graphState.nodePositions[graphState.draggedNode].y = graphState.mouseY - graphState.offsetY;

                    // Mark as dragging if mouse moved more than 5 pixels
                    const dx = graphState.mouseX - graphState.dragStartX;
                    const dy = graphState.mouseY - graphState.dragStartY;
                    if (Math.sqrt(dx*dx + dy*dy) > 5) {{
                        graphState.isDragging = true;
                    }}
                }} else {{
                    // Check for hovered node (works for both circles and triangles)
                    graphState.hoveredNode = null;
                    Object.keys(graphState.nodePositions).forEach(nodeId => {{
                        const pos = graphState.nodePositions[nodeId];
                        const dx = graphState.mouseX - pos.x;
                        const dy = graphState.mouseY - pos.y;
                        const dist = Math.sqrt(dx * dx + dy * dy);
                        // Use larger hit area (30px) to account for triangle shapes
                        if (dist < 30) {{
                            graphState.hoveredNode = nodeId;
                        }}
                    }});
                    canvas.style.cursor = graphState.hoveredNode ? 'pointer' : 'default';
                }}
            }});

            canvas.addEventListener('mousedown', (e) => {{
                if (graphState.hoveredNode) {{
                    graphState.draggedNode = graphState.hoveredNode;
                    const pos = graphState.nodePositions[graphState.draggedNode];
                    graphState.offsetX = graphState.mouseX - pos.x;
                    graphState.offsetY = graphState.mouseY - pos.y;
                    graphState.dragStartX = graphState.mouseX;
                    graphState.dragStartY = graphState.mouseY;
                    graphState.isDragging = false; // Reset drag flag
                    canvas.style.cursor = 'grabbing';
                    e.preventDefault(); // Prevent text selection while dragging
                }}
            }});

            canvas.addEventListener('mouseup', () => {{
                graphState.draggedNode = null;
                canvas.style.cursor = graphState.hoveredNode ? 'pointer' : 'default';
            }});

            canvas.addEventListener('mouseleave', () => {{
                graphState.draggedNode = null;
                graphState.hoveredNode = null;
                canvas.style.cursor = 'default';
            }});

            canvas.addEventListener('click', (e) => {{
                // Don't open log panel if user was dragging
                if (graphState.isDragging) {{
                    graphState.isDragging = false; // Reset for next interaction
                    return;
                }}

                if (graphState.hoveredNode) {{
                    // Find the node data
                    const clickedNode = graphData.nodes.find(n => n.id === graphState.hoveredNode);
                    if (clickedNode) {{
                        if (clickedNode.type === 'process') {{
                            showNodeLogs(clickedNode.label);
                        }} else if (clickedNode.type === 'topic') {{
                            showTopicLogs(clickedNode.label);
                        }}
                    }}
                }}
            }});
        }}

        // Enhanced graph renderer with glowing nodes
        function renderGraph(nodes, edges) {{
            const canvas = document.getElementById('graph-canvas');
            if (!canvas) return;

            const ctx = canvas.getContext('2d');
            const width = canvas.width;
            const height = canvas.height;

            // Debug: Log canvas dimensions on first render
            if (!window.graphDebugLogged) {{
                console.log('Graph canvas dimensions:', width, 'x', height);
                window.graphDebugLogged = true;
            }}

            // Clear canvas
            ctx.fillStyle = 'rgb(10, 11, 13)';
            ctx.fillRect(0, 0, width, height);

            // Barycenter Heuristic Layout: Minimizes edge crossings in bipartite graph
            const processNodes = nodes.filter(n => n.type === 'process');
            const topicNodes = nodes.filter(n => n.type === 'topic');

            console.log(`[#] Graph data: ${{processNodes.length}} processes, ${{topicNodes.length}} topics, ${{edges.length}} edges`);
            if (topicNodes.length === 0) {{
                console.warn(' No topic nodes found! Node types:', nodes.map(n => `${{n.id}}:${{n.type}}`));
            }}

            // Initialize positions only if not already set (preserve drag positions)
            // OR if we have new nodes that don't have positions yet
            const needsLayout = !graphState.nodePositions ||
                                Object.keys(graphState.nodePositions).length === 0 ||
                                nodes.some(n => !graphState.nodePositions[n.id]);

            if (needsLayout) {{
                console.log(' Computing Barycenter layout...');

                // Step 1: Build adjacency maps
                const processToTopics = {{}};  // process_id -> [topic_ids]
                const topicToProcesses = {{}}; // topic_id -> [process_ids]

                edges.forEach(edge => {{
                    const isProcessToTopic = processNodes.some(p => p.id === edge.from);
                    if (isProcessToTopic) {{
                        if (!processToTopics[edge.from]) processToTopics[edge.from] = [];
                        processToTopics[edge.from].push(edge.to);
                        if (!topicToProcesses[edge.to]) topicToProcesses[edge.to] = [];
                        topicToProcesses[edge.to].push(edge.from);
                    }} else {{
                        if (!topicToProcesses[edge.from]) topicToProcesses[edge.from] = [];
                        topicToProcesses[edge.from].push(edge.to);
                        if (!processToTopics[edge.to]) processToTopics[edge.to] = [];
                        processToTopics[edge.to].push(edge.from);
                    }}
                }});

                // Step 2: Initial ordering (by ID for deterministic results)
                let processOrder = [...processNodes].sort((a, b) => a.id.localeCompare(b.id));
                let topicOrder = [...topicNodes].sort((a, b) => a.id.localeCompare(b.id));

                // Step 3: Barycenter iterations (5 iterations for convergence)
                for (let iter = 0; iter < 5; iter++) {{
                    // 3a. Reorder topics based on average Y of connected processes
                    topicOrder = topicOrder.map(topic => {{
                        const connectedProcesses = topicToProcesses[topic.id] || [];
                        if (connectedProcesses.length === 0) return {{ node: topic, barycenter: 0 }};

                        const avgIndex = connectedProcesses.reduce((sum, procId) => {{
                            const index = processOrder.findIndex(p => p.id === procId);
                            return sum + (index >= 0 ? index : 0);
                        }}, 0) / connectedProcesses.length;

                        return {{ node: topic, barycenter: avgIndex }};
                    }}).sort((a, b) => a.barycenter - b.barycenter).map(item => item.node);

                    // 3b. Reorder processes based on average Y of connected topics
                    processOrder = processOrder.map(process => {{
                        const connectedTopics = processToTopics[process.id] || [];
                        if (connectedTopics.length === 0) return {{ node: process, barycenter: 0 }};

                        const avgIndex = connectedTopics.reduce((sum, topicId) => {{
                            const index = topicOrder.findIndex(t => t.id === topicId);
                            return sum + (index >= 0 ? index : 0);
                        }}, 0) / connectedTopics.length;

                        return {{ node: process, barycenter: avgIndex }};
                    }}).sort((a, b) => a.barycenter - b.barycenter).map(item => item.node);
                }}

                // Step 4: Calculate final positions with even spacing
                const margin = 80;
                const processX = 180;
                const topicX = width - 250;

                // Calculate optimal spacing
                const processSpacing = processOrder.length > 1
                    ? Math.min(120, (height - 2 * margin) / (processOrder.length - 1))
                    : 0;
                const topicSpacing = topicOrder.length > 1
                    ? Math.min(100, (height - 2 * margin) / (topicOrder.length - 1))
                    : 0;

                // Position processes (only if they don't already have positions)
                const processTotalHeight = (processOrder.length - 1) * processSpacing;
                const processStartY = (height - processTotalHeight) / 2;

                processOrder.forEach((node, i) => {{
                    // Skip if already positioned (preserve manual drag positions)
                    if (!graphState.nodePositions[node.id]) {{
                        const y = processOrder.length === 1
                            ? height / 2
                            : processStartY + i * processSpacing;
                        graphState.nodePositions[node.id] = {{
                            x: processX,
                            y: Math.max(margin, Math.min(height - margin, y))
                        }};
                    }}
                }});

                // Position topics (only if they don't already have positions)
                const topicTotalHeight = (topicOrder.length - 1) * topicSpacing;
                const topicStartY = (height - topicTotalHeight) / 2;

                topicOrder.forEach((node, i) => {{
                    // Skip if already positioned (preserve manual drag positions)
                    if (!graphState.nodePositions[node.id]) {{
                        const y = topicOrder.length === 1
                            ? height / 2
                            : topicStartY + i * topicSpacing;
                        graphState.nodePositions[node.id] = {{
                            x: topicX,
                            y: Math.max(margin, Math.min(height - margin, y))
                        }};
                    }}
                }});

                console.log(` Layout complete: ${{processOrder.length}} processes, ${{topicOrder.length}} topics`);
            }}

            // Draw edges
            edges.forEach(edge => {{
                const from = graphState.nodePositions[edge.from];
                const to = graphState.nodePositions[edge.to];
                if (!from || !to) return;

                ctx.beginPath();
                ctx.moveTo(from.x, from.y);
                ctx.lineTo(to.x, to.y);
                ctx.strokeStyle = edge.type === 'publish' ? 'rgba(0, 212, 255, 0.7)' : 'rgba(0, 255, 136, 0.7)';
                ctx.lineWidth = 2.5;
                ctx.stroke();

                // Arrow head
                const angle = Math.atan2(to.y - from.y, to.x - from.x);
                const headlen = 15;
                ctx.beginPath();
                ctx.moveTo(to.x, to.y);
                ctx.lineTo(to.x - headlen * Math.cos(angle - Math.PI / 6), to.y - headlen * Math.sin(angle - Math.PI / 6));
                ctx.lineTo(to.x - headlen * Math.cos(angle + Math.PI / 6), to.y - headlen * Math.sin(angle + Math.PI / 6));
                ctx.closePath();
                ctx.fillStyle = edge.type === 'publish' ? 'rgba(0, 212, 255, 0.9)' : 'rgba(0, 255, 136, 0.9)';
                ctx.fill();
            }});

            // Draw animated data packets (tiny dots)
            dataPackets.forEach(packet => {{
                const edge = edges[packet.edgeIndex];
                if (!edge) return;

                const from = graphState.nodePositions[edge.from];
                const to = graphState.nodePositions[edge.to];
                if (!from || !to) return;

                // Calculate packet position by interpolating along the edge
                const t = packet.progress / 100;
                const x = from.x + (to.x - from.x) * t;
                const y = from.y + (to.y - from.y) * t;

                // Color based on edge type (cyan for publish, green for subscribe)
                const color = edge.type === 'publish'
                    ? 'rgba(0, 212, 255, 0.9)'
                    : 'rgba(0, 255, 136, 0.9)';

                // Draw tiny dot (3px radius)
                ctx.beginPath();
                ctx.arc(x, y, 3, 0, 2 * Math.PI);
                ctx.fillStyle = color;
                ctx.fill();
            }});

            // Draw nodes - RQT style: Circles for processes, Triangles for topics
            nodes.forEach(node => {{
                const pos = graphState.nodePositions[node.id];
                if (!pos) return;

                const isHovered = graphState.hoveredNode === node.id;
                const isDragged = graphState.draggedNode === node.id;
                const nodeSize = 15;

                // Different colors for processes vs topics
                const color = node.type === 'process'
                    ? {{ r: 0, g: 255, b: 136 }}     // Green for processes
                    : {{ r: 255, g: 20, b: 147 }};   // Pink for topics

                if (node.type === 'process') {{
                    // PROCESSES: Draw as circles

                    // Outer glow
                    for (let i = 4; i > 0; i--) {{
                        ctx.beginPath();
                        ctx.arc(pos.x, pos.y, nodeSize + i * 5, 0, 2 * Math.PI);
                        ctx.fillStyle = `rgba(${{color.r}}, ${{color.g}}, ${{color.b}}, ${{0.15 / i * (isHovered || isDragged ? 2.5 : 1)}})`;
                        ctx.fill();
                    }}

                    // Main circle
                    ctx.beginPath();
                    ctx.arc(pos.x, pos.y, nodeSize, 0, 2 * Math.PI);
                    ctx.fillStyle = `rgb(${{color.r}}, ${{color.g}}, ${{color.b}})`;
                    ctx.shadowBlur = isHovered || isDragged ? 25 : 15;
                    ctx.shadowColor = `rgba(${{color.r}}, ${{color.g}}, ${{color.b}}, 0.9)`;
                    ctx.fill();
                    ctx.shadowBlur = 0;
                }} else {{
                    // TOPICS: Draw as rectangles (RQT style)
                    // Format topic name with "/" (replace underscores)
                    const topicName = node.label.replace(/_/g, '/');

                    // Dynamically size rectangle based on text length
                    ctx.font = '10px JetBrains Mono, monospace';
                    const textWidth = ctx.measureText(topicName).width;
                    const rectWidth = Math.max(nodeSize * 3, textWidth + 20);
                    const rectHeight = nodeSize * 1.5;

                    // Outer glow (rectangle-shaped)
                    for (let i = 4; i > 0; i--) {{
                        const glowPadding = i * 5;
                        ctx.fillStyle = `rgba(${{color.r}}, ${{color.g}}, ${{color.b}}, ${{0.15 / i * (isHovered || isDragged ? 2.5 : 1)}})`;
                        ctx.fillRect(
                            pos.x - rectWidth/2 - glowPadding,
                            pos.y - rectHeight/2 - glowPadding,
                            rectWidth + glowPadding * 2,
                            rectHeight + glowPadding * 2
                        );
                    }}

                    // Main rectangle
                    ctx.fillStyle = `rgb(${{color.r}}, ${{color.g}}, ${{color.b}})`;
                    ctx.shadowBlur = isHovered || isDragged ? 25 : 15;
                    ctx.shadowColor = `rgba(${{color.r}}, ${{color.g}}, ${{color.b}}, 0.9)`;
                    ctx.fillRect(
                        pos.x - rectWidth/2,
                        pos.y - rectHeight/2,
                        rectWidth,
                        rectHeight
                    );
                    ctx.shadowBlur = 0;

                    // Draw topic name INSIDE the rectangle
                    ctx.fillStyle = 'rgb(10, 11, 13)'; // Dark text on pink background
                    ctx.font = `${{isHovered || isDragged ? '600' : '400'}} 10px JetBrains Mono, monospace`;
                    ctx.textAlign = 'center';
                    ctx.textBaseline = 'middle';
                    ctx.fillText(topicName, pos.x, pos.y);
                }}

                // Draw label for processes only (below the circle)
                if (node.type === 'process') {{
                    ctx.fillStyle = isHovered || isDragged ? 'rgb(255, 255, 255)' : 'rgb(226, 232, 240)';
                    ctx.font = `${{isHovered || isDragged ? '600' : '400'}} 11px JetBrains Mono, monospace`;
                    ctx.textAlign = 'center';
                    ctx.textBaseline = 'top';
                    ctx.shadowBlur = isHovered || isDragged ? 8 : 0;
                    ctx.shadowColor = `rgba(${{color.r}}, ${{color.g}}, ${{color.b}}, 0.8)`;
                    const label = node.label.length > 15 ? node.label.substring(0, 12) + '...' : node.label;
                    ctx.fillText(label, pos.x, pos.y + 25);
                    ctx.shadowBlur = 0;
                }}
            }});
        }}

        // Initialize interaction once
        initGraphInteraction();

        // Handle window resize for canvas
        window.addEventListener('resize', () => {{
            const canvas = document.getElementById('graph-canvas');
            if (canvas && canvas.offsetParent !== null) {{ // Only resize if visible
                const rect = canvas.getBoundingClientRect();
                const oldWidth = canvas.width;
                const oldHeight = canvas.height;
                canvas.width = rect.width;
                canvas.height = rect.height;

                // Re-render if dimensions changed
                if (oldWidth !== canvas.width || oldHeight !== canvas.height) {{
                    console.log('Canvas resized:', canvas.width, 'x', canvas.height);
                    if (window.graphState && window.graphState.nodes) {{
                        renderGraph(window.graphState.nodes, window.graphState.edges);
                    }}
                }}
            }}
        }});

        // Continuous graph rendering for smooth interaction
        let graphData = {{ nodes: [], edges: [] }};

        // Data packets state for animation
        let dataPackets = [];

        // Packet spawner and updater (1 packet per edge max)
        setInterval(() => {{
            // Update all packet progress
            dataPackets = dataPackets
                .map(p => ({{ ...p, progress: p.progress + 2 }}))
                .filter(p => p.progress < 100);

            // Spawn new packet only on edges without packets (30% chance per tick)
            if (Math.random() > 0.7 && graphData.edges && graphData.edges.length > 0) {{
                // Find edges that don't have packets
                const occupiedEdges = new Set(dataPackets.map(p => p.edgeIndex));
                const availableEdges = graphData.edges
                    .map((edge, idx) => idx)
                    .filter(idx => !occupiedEdges.has(idx));

                if (availableEdges.length > 0) {{
                    const edgeIndex = availableEdges[Math.floor(Math.random() * availableEdges.length)];
                    const edge = graphData.edges[edgeIndex];

                    dataPackets.push({{
                        id: Date.now() + Math.random(),
                        progress: 0,
                        edgeIndex: edgeIndex,
                        direction: edge.type // 'publish' or 'subscribe'
                    }});
                }}
            }}
        }}, 50);

        function animateGraph() {{
            if (graphData.nodes.length > 0) {{
                renderGraph(graphData.nodes, graphData.edges);
            }}
            requestAnimationFrame(animateGraph);
        }}
        animateGraph();

        // Update graph data
        async function updateGraphData() {{
            try {{
                const response = await fetch('/api/graph');
                const data = await response.json();
                console.log('[#] Graph API Response:', data);
                console.log(`   Nodes: ${{data.nodes?.length || 0}}, Edges: ${{data.edges?.length || 0}}`);
                if (data.edges && data.edges.length > 0) {{
                    console.log('   Edges:', data.edges);
                }} else {{
                    console.warn(' No edges found! Cannot draw connection lines.');
                }}
                graphData = data;
                // Store in global state for resize handler
                window.graphState.nodes = data.nodes || [];
                window.graphState.edges = data.edges || [];
            }} catch (error) {{
                console.error('Failed to fetch graph:', error);
            }}
        }}

        // Refresh all monitor data (triggers backend re-scan + updates frontend)
        // Backend performs fresh system scan on each API call - no caching
        async function refreshMonitorData() {{
            console.log(' Refreshing monitor data (backend + frontend)...');

            // Reset graph layout to default positions
            resetGraphLayout();

            await Promise.all([
                updateNodes(),      // Re-scans processes
                updateTopics(),     // Re-scans shared memory
                updateGraphData()   // Re-builds graph from fresh data
            ]);
            console.log(' Monitor data refreshed');
        }}

        // Track current log view for auto-updates
        let currentLogView = {{ type: null, name: null, interval: null }};

        // Log panel functions (defined early so onclick handlers can use them)
        async function showNodeLogs(nodeName) {{
            const panel = document.getElementById('log-panel');
            const title = document.getElementById('log-panel-title');
            const content = document.getElementById('log-panel-content');

            if (!panel || !title || !content) {{
                return;
            }}

            // Stop previous auto-update
            if (currentLogView.interval) {{
                clearInterval(currentLogView.interval);
            }}

            currentLogView = {{ type: 'node', name: nodeName, interval: null }};

            title.textContent = `Logs: ${{nodeName}} (live)`;
            content.innerHTML = '<p style="color: var(--text-secondary);">Loading logs...</p>';
            panel.classList.add('open');

            async function updateLogs() {{
                try {{
                    const response = await fetch(`/api/logs/node/${{encodeURIComponent(nodeName)}}`);
                    const data = await response.json();

                    if (data.logs && data.logs.length > 0) {{
                        const wasScrolledToBottom = content.scrollHeight - content.scrollTop <= content.clientHeight + 50;

                        content.innerHTML = data.logs.slice(-100).map(log => `
                            <div class="log-entry">
                                <div class="log-entry-header">
                                    <span class="log-timestamp">${{log.timestamp}}</span>
                                    <span class="log-type log-type-${{log.log_type.toLowerCase()}}">${{log.log_type}}</span>
                                </div>
                                ${{log.topic ? `<div style="color: var(--text-tertiary); font-size: 0.75rem;">Topic: ${{log.topic}}</div>` : ''}}
                                <div class="log-message">${{log.message}}</div>
                                <div style="color: var(--text-tertiary); font-size: 0.7rem; margin-top: 0.5rem;">
                                    Tick: ${{log.tick_us}}μs | IPC: ${{log.ipc_ns}}ns
                                </div>
                            </div>
                        `).join('');

                        if (wasScrolledToBottom) {{
                            content.scrollTop = content.scrollHeight;
                        }}
                    }} else {{
                        content.innerHTML = '<p style="color: var(--text-secondary);">No logs found for this node</p>';
                    }}
                }} catch (error) {{
                    content.innerHTML = `<p style="color: #ff4444;">Error loading logs: ${{error.message}}</p>`;
                }}
            }}

            await updateLogs();
            // Use requestAnimationFrame for maximum speed (display refresh rate)
            function animationLoop() {{
                updateLogs();
                if (currentLogView.interval) {{
                    currentLogView.interval = requestAnimationFrame(animationLoop);
                }}
            }}
            currentLogView.interval = requestAnimationFrame(animationLoop);
        }}

        async function showTopicLogs(topicName) {{
            const panel = document.getElementById('log-panel');
            const title = document.getElementById('log-panel-title');
            const content = document.getElementById('log-panel-content');

            if (!panel || !title || !content) {{
                return;
            }}

            // Stop previous auto-update
            if (currentLogView.interval) {{
                clearInterval(currentLogView.interval);
            }}

            currentLogView = {{ type: 'topic', name: topicName, interval: null }};

            title.textContent = `Logs: ${{topicName}} (live)`;
            content.innerHTML = '<p style="color: var(--text-secondary);">Loading logs...</p>';
            panel.classList.add('open');

            async function updateLogs() {{
                try {{
                    const response = await fetch(`/api/logs/topic/${{encodeURIComponent(topicName)}}`);
                    const data = await response.json();

                    if (data.logs && data.logs.length > 0) {{
                        const wasScrolledToBottom = content.scrollHeight - content.scrollTop <= content.clientHeight + 50;

                        content.innerHTML = data.logs.slice(-100).map(log => {{
                            // Convert log type to topic-centric description
                            let operation = log.log_type;
                            if (log.log_type === 'Publish') {{
                                operation = 'Write';
                            }} else if (log.log_type === 'Subscribe') {{
                                operation = 'Read';
                            }} else if (log.log_type === 'TopicMap') {{
                                operation = 'Map';
                            }} else if (log.log_type === 'TopicUnmap') {{
                                operation = 'Unmap';
                            }}

                            return `
                            <div class="log-entry">
                                <div class="log-entry-header">
                                    <span class="log-timestamp">${{log.timestamp}}</span>
                                    <span class="log-type log-type-${{log.log_type.toLowerCase()}}">${{operation}}</span>
                                </div>
                                <div style="color: var(--accent); font-size: 0.85rem; font-weight: 500;">by ${{log.node_name}}</div>
                                <div class="log-message">${{log.message}}</div>
                                ${{log.ipc_ns > 0 ? `<div style="color: var(--text-tertiary); font-size: 0.7rem; margin-top: 0.5rem;">
                                    Tick: ${{log.tick_us}}μs | IPC: ${{log.ipc_ns}}ns
                                </div>` : ''}}
                            </div>
                        `;
                        }}).join('');

                        if (wasScrolledToBottom) {{
                            content.scrollTop = content.scrollHeight;
                        }}
                    }} else {{
                        content.innerHTML = '<p style="color: var(--text-secondary);">No logs found for this topic</p>';
                    }}
                }} catch (error) {{
                    content.innerHTML = `<p style="color: #ff4444;">Error loading logs: ${{error.message}}</p>`;
                }}
            }}

            await updateLogs();
            // Use requestAnimationFrame for maximum speed (display refresh rate)
            function animationLoop() {{
                updateLogs();
                if (currentLogView.interval) {{
                    currentLogView.interval = requestAnimationFrame(animationLoop);
                }}
            }}
            currentLogView.interval = requestAnimationFrame(animationLoop);
        }}

        function closeLogPanel() {{
            // Stop auto-updates when panel closes
            if (currentLogView.interval) {{
                cancelAnimationFrame(currentLogView.interval);
                currentLogView = {{ type: null, name: null, interval: null }};
            }}
            document.getElementById('log-panel').classList.remove('open');
        }}

        // WebSocket connection for real-time updates
        let ws = null;
        let wsConnected = false;
        let pollingInterval = null;

        function connectWebSocket() {{
            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            const wsUrl = `${{protocol}}//${{window.location.host}}/api/ws`;

            ws = new WebSocket(wsUrl);

            ws.onopen = () => {{
                console.log('WebSocket connected - real-time updates enabled (20 FPS)');
                wsConnected = true;

                // Clear polling fallback if it was running
                if (pollingInterval) {{
                    clearInterval(pollingInterval);
                    pollingInterval = null;
                }}
            }};

            ws.onmessage = (event) => {{
                try {{
                    const update = JSON.parse(event.data);
                    if (update.type === 'update') {{
                        // Update nodes - create if missing, update if exists
                        if (update.data.nodes) {{
                            const nodesList = document.getElementById('nodes-list');
                            const existingNodes = nodesList.querySelectorAll('.node-item');

                            // If node count changed, do full refresh
                            if (existingNodes.length !== update.data.nodes.length) {{
                                if (update.data.nodes.length === 0) {{
                                    nodesList.innerHTML = '<div class=\"placeholder\">No active nodes detected.<br><div class=\"command\" style=\"margin-top: 1rem;\"><span class=\"command-prompt\">$</span> horus run your_node.rs</div></div>';
                                }} else {{
                                    nodesList.innerHTML = update.data.nodes.map(node => `
                                        <div class=\"node-item\" data-node-name=\"${{node.name}}\" title=\"Click to view logs\">
                                            <div class=\"node-header\">
                                                <span class=\"node-name\">${{node.name}}</span>
                                                <span class=\"node-status status-${{node.health_color || 'gray'}}\">${{node.health}}</span>
                                            </div>
                                            <div class=\"node-details\">
                                                <span>PID: ${{node.pid}}</span>
                                                <span>CPU: ${{node.cpu}}</span>
                                                <span>Memory: ${{node.memory}}</span>
                                            </div>
                                        </div>
                                    `).join('');
                                }}
                            }} else {{
                                // Just update stats for existing nodes
                                update.data.nodes.forEach(node => {{
                                    const nodeItem = document.querySelector(`[data-node-name="${{node.name}}"]`);
                                    if (nodeItem) {{
                                        // Update node details
                                        const details = nodeItem.querySelector('.node-details');
                                        if (details) {{
                                            details.innerHTML = `
                                                <span>PID: ${{node.pid}}</span>
                                                <span>CPU: ${{node.cpu}}</span>
                                                <span>Memory: ${{node.memory}}</span>
                                            `;
                                        }}

                                        // Update status badge color
                                        const statusBadge = nodeItem.querySelector('.node-status');
                                        if (statusBadge) {{
                                            statusBadge.className = `node-status status-${{node.health_color || 'gray'}}`;
                                            statusBadge.textContent = node.health;
                                        }}
                                    }}
                                }});
                            }}
                        }}

                        // Update topics - create if missing, update if exists
                        if (update.data.topics) {{
                            const topicsList = document.getElementById('topics-list');
                            const existingTopics = topicsList.querySelectorAll('.topic-item');

                            // If topic count changed, do full refresh
                            if (existingTopics.length !== update.data.topics.length) {{
                                if (update.data.topics.length === 0) {{
                                    topicsList.innerHTML = '<div class=\"placeholder\">No topics available.</div>';
                                }} else {{
                                    topicsList.innerHTML = update.data.topics.map(topic => `
                                        <div class=\"topic-item\" data-topic-name=\"${{topic.name}}\" title=\"Click to view logs\">
                                            <div class=\"topic-header\">
                                                <span class=\"topic-name\">${{topic.name}}</span>
                                                <span class=\"node-status status-running\">${{topic.active ? 'Active' : 'Inactive'}}</span>
                                            </div>
                                            <div class=\"topic-details\">
                                                <span>Size: ${{topic.size}}</span>
                                                <span>Processes: ${{topic.processes}}</span>
                                            </div>
                                        </div>
                                    `).join('');
                                }}
                            }} else {{
                                // Just update stats for existing topics
                                update.data.topics.forEach(topic => {{
                                    const topicItem = document.querySelector(`[data-topic-name="${{topic.name}}"]`);
                                    if (topicItem) {{
                                        const details = topicItem.querySelector('.topic-details');
                                        if (details) {{
                                            details.innerHTML = `
                                                <span>Size: ${{topic.size}}</span>
                                                <span>Processes: ${{topic.processes}}</span>
                                            `;
                                        }}
                                    }}
                                }});
                            }}
                        }}

                        // Update graph data
                        if (update.data.graph) {{
                            graphData = update.data.graph;
                        }}

                        // Update status bar and tooltips
                        updateStatus();
                        updateNodesToolTip();
                        updateTopicsToolTip();
                    }}
                }} catch (error) {{
                    console.error('WebSocket message parse error:', error);
                }}
            }};

            ws.onerror = (error) => {{
                console.warn(' WebSocket error, falling back to polling');
                wsConnected = false;
            }};

            ws.onclose = () => {{
                console.log('🔌 WebSocket disconnected, falling back to polling');
                wsConnected = false;

                // Fallback to polling
                if (!pollingInterval) {{
                    pollingInterval = setInterval(updateAll, 100);
                }}

                // Try to reconnect after 5 seconds
                setTimeout(connectWebSocket, 5000);
            }};
        }}

        // Update all data (polling fallback)
        function updateAll() {{
            updateStatus();
            updateNodes();
            updateTopics();
            updateGraphData();
            updateNodesToolTip();
            updateTopicsToolTip();
        }}

        // Event delegation for node and topic clicks - SET UP EARLY!
        console.log(' Setting up event delegation for nodes and topics');

        try {{
            document.addEventListener('click', (e) => {{
                // Check if click is on a node item
                const nodeItem = e.target.closest('.node-item');
                if (nodeItem) {{
                    e.preventDefault();
                    e.stopPropagation();
                    const nodeName = nodeItem.getAttribute('data-node-name');
                    if (nodeName) {{
                        showNodeLogs(nodeName);
                    }}
                    return;
                }}

                // Check if click is on a topic item
                const topicItem = e.target.closest('.topic-item');
                if (topicItem) {{
                    e.preventDefault();
                    e.stopPropagation();
                    const topicName = topicItem.getAttribute('data-topic-name');
                    if (topicName) {{
                        showTopicLogs(topicName);
                    }}
                    return;
                }}
            }}, true);

            console.log('Event delegation successfully attached!');
        }} catch (err) {{
            console.error('Failed to attach event delegation:', err);
        }}

        // Try WebSocket first, fallback to polling
        connectWebSocket();

        // Initial load via polling (in case WebSocket takes time to connect)
        updateAll();

        // Theme toggle functionality
        function toggleTheme() {{
            const html = document.documentElement;
            const currentTheme = html.getAttribute('data-theme');
            const newTheme = currentTheme === 'light' ? 'dark' : 'light';
            const themeButton = document.getElementById('theme-toggle');

            html.setAttribute('data-theme', newTheme);
            themeButton.textContent = newTheme === 'light' ? '☀️' : '🌙';

            // Save preference to localStorage
            localStorage.setItem('horus-theme', newTheme);
        }}

        // Load saved theme preference
        function loadTheme() {{
            const savedTheme = localStorage.getItem('horus-theme') || 'dark';
            const html = document.documentElement;
            const themeButton = document.getElementById('theme-toggle');

            html.setAttribute('data-theme', savedTheme);
            themeButton.textContent = savedTheme === 'light' ? '☀️' : '🌙';
        }}

        // Load theme on page load
        loadTheme();

        // Help modal functions
        function toggleHelp() {{
            const modal = document.getElementById('help-modal');
            modal.classList.toggle('active');
        }}

        // Keyboard shortcuts
        document.addEventListener('keydown', function(e) {{
            // Press '?' to open help
            if (e.key === '?' && !e.ctrlKey && !e.altKey && !e.metaKey) {{
                const activeElement = document.activeElement;
                // Don't trigger if typing in an input
                if (activeElement.tagName !== 'INPUT' && activeElement.tagName !== 'TEXTAREA') {{
                    e.preventDefault();
                    toggleHelp();
                }}
            }}

            // Press 'Esc' to close help modal
            if (e.key === 'Escape') {{
                const modal = document.getElementById('help-modal');
                if (modal.classList.contains('active')) {{
                    toggleHelp();
                }}
            }}
        }});

        // Close modal when clicking outside of it
        document.getElementById('help-modal').addEventListener('click', function(e) {{
            if (e.target === this) {{
                toggleHelp();
            }}
        }});

        // Package view switching
        function switchPackageView(view) {{
            // Hide all package views
            document.querySelectorAll('.package-view').forEach(v => v.style.display = 'none');

            // Remove active from all view buttons
            const packageTab = document.getElementById('tab-packages');
            packageTab.querySelectorAll('.view-btn').forEach(btn => btn.classList.remove('active'));

            // Show selected view and activate button
            if (view === 'global') {{
                document.getElementById('package-global').style.display = 'block';
                event.target.classList.add('active');
                loadGlobalEnvironment();
            }} else if (view === 'local') {{
                document.getElementById('package-local').style.display = 'block';
                event.target.classList.add('active');
                loadLocalEnvironments();
            }} else if (view === 'registry') {{
                document.getElementById('package-registry').style.display = 'block';
                event.target.classList.add('active');
            }}
        }}

        async function loadEnvironments() {{
            const container = document.getElementById('environments-list');
            try {{
                const response = await fetch('/api/packages/environments');
                const data = await response.json();

                let html = '';

                // Global Environment Section
                html += '<div style="margin-bottom: 30px;">';
                html += '<h3 style="color: var(--accent); margin-bottom: 15px; display: flex; align-items: center; gap: 8px;">';
                html += 'Global Environment';
                html += `<span style="font-size: 0.8em; color: var(--text-secondary); font-weight: normal;">(${{data.global?.length || 0}} packages)</span>`;
                html += '</h3>';

                if (data.global && data.global.length > 0) {{
                    html += data.global.map(pkg => `
                        <div style="padding: 12px 15px; background: var(--surface); border: 1px solid var(--border); border-radius: 6px; margin-bottom: 8px;">
                            <div style="display: flex; justify-content: space-between; align-items: center;">
                                <div>
                                    <span style="font-weight: 600; color: var(--text-primary);">${{pkg.name}}</span>
                                    <span style="color: var(--text-secondary); margin-left: 10px; font-size: 0.9em;">v${{pkg.version}}</span>
                                </div>
                            </div>
                        </div>
                    `).join('');
                }} else {{
                    html += '<p style="color: var(--text-secondary); font-size: 0.9em;">No global packages installed</p>';
                }}
                html += '</div>';

                // Local Environments Section
                html += '<div>';
                html += '<h3 style="color: var(--success); margin-bottom: 15px; display: flex; align-items: center; gap: 8px;">';
                html += 'Local Environments';
                html += `<span style="font-size: 0.8em; color: var(--text-secondary); font-weight: normal;">(${{data.local?.length || 0}} environments)</span>`;
                html += '</h3>';

                if (data.local && data.local.length > 0) {{
                    html += data.local.map((env, index) => {{
                        let expandableContent = '';
                        if (env.packages && env.packages.length > 0) {{
                            expandableContent = `
                                <div id="env-details-${{index}}" style="display: none; margin-top: 10px; padding-top: 10px; border-top: 1px solid var(--border);">
                                    <div style="color: var(--text-secondary); margin-bottom: 8px; font-size: 0.9em;">
                                        Packages in this environment:
                                    </div>
                                    ${{env.packages.map((p, pidx) => `
                                        <div style="margin-bottom: 6px;">
                                            <div onclick="togglePackageDetails(${{index}}, ${{pidx}})" style="padding: 8px 12px; background: var(--primary); border: 1px solid var(--border); border-radius: 4px; display: flex; justify-content: space-between; align-items: center; cursor: pointer; transition: background 0.2s;">
                                                <div style="display: flex; align-items: center; gap: 8px;">
                                                    <span id="pkg-arrow-${{index}}-${{pidx}}" style="color: var(--accent); font-size: 0.8em;">▶</span>
                                                    <span style="font-weight: 500;">${{p.name}}</span>
                                                </div>
                                                <span style="color: var(--text-secondary); font-size: 0.85em;">v${{p.version}}</span>
                                            </div>
                                            <div id="pkg-details-${{index}}-${{pidx}}" style="display: none; padding: 10px 12px; margin-left: 20px; background: var(--dark-bg); border-left: 2px solid var(--accent); border-radius: 4px; margin-top: 4px;">
                                                <div style="color: var(--text-secondary); font-size: 0.85em; margin-bottom: 8px;"><strong>Installed Packages (${{p.installed_packages?.length || 0}}):</strong></div>
                                                ${{p.installed_packages && p.installed_packages.length > 0 ? `
                                                    <div style="display: flex; flex-direction: column; gap: 4px;">
                                                        ${{p.installed_packages.map(pkg => `
                                                            <div style="padding: 6px 10px; background: var(--surface); border: 1px solid var(--border); border-radius: 4px; display: flex; justify-content: space-between; align-items: center; gap: 10px;">
                                                                <div style="display: flex; align-items: center; gap: 10px; flex: 1;">
                                                                    <span style="color: var(--text-primary); font-size: 0.85em;">${{pkg.name}}</span>
                                                                    <span style="color: var(--text-tertiary); font-size: 0.75em;">v${{pkg.version}}</span>
                                                                </div>
                                                                <button
                                                                    onclick="uninstallPackage('${{p.name}}', '${{pkg.name}}', event)"
                                                                    style="padding: 4px 10px; background: var(--error, #ff4444); color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 0.75em; font-weight: 600; transition: opacity 0.2s;"
                                                                    onmouseover="this.style.opacity='0.8'"
                                                                    onmouseout="this.style.opacity='1'"
                                                                >
                                                                    Uninstall
                                                                </button>
                                                            </div>
                                                        `).join('')}}
                                                    </div>
                                                ` : '<div style="color: var(--text-tertiary); font-size: 0.85em;">No packages installed</div>'}}
                                            </div>
                                        </div>
                                    `).join('')}}
                                </div>
                            `;
                        }}

                        return `
                            <div class="package-item" style="padding: 15px; background: var(--surface); border: 1px solid var(--border); border-radius: 6px; margin-bottom: 10px; ${{env.package_count > 0 ? 'cursor: pointer;' : ''}}">
                                <div style="display: flex; justify-content: space-between; align-items: center;" onclick="${{env.package_count > 0 ? `toggleEnvDetails(${{index}})` : ''}}">
                                    <div style="flex: 1;">
                                        <div style="display: flex; align-items: center; gap: 8px;">
                                            <span style="font-weight: 600; color: var(--text-primary); font-size: 1.05em;">
                                                ${{env.name}}
                                            </span>
                                            ${{env.package_count > 0 ? '<span id="arrow-' + index + '" style="color: var(--accent); font-size: 0.9em;">▶</span>' : ''}}
                                        </div>
                                        <div style="color: var(--text-secondary); margin-top: 5px; font-size: 0.85em;">
                                            ${{env.path}} • ${{env.package_count}} package(s)
                                        </div>
                                    </div>
                                </div>
                                ${{expandableContent}}
                            </div>
                        `;
                    }}).join('');
                }} else {{
                    html += '<p style="color: var(--text-secondary); font-size: 0.9em;">No local environments found</p>';
                }}
                html += '</div>';

                container.innerHTML = html;
            }} catch (error) {{
                container.innerHTML = `<p style="color: var(--error);">Failed to load environments: ${{error.message}}</p>`;
            }}
        }}

        function toggleEnvDetails(index) {{
            const detailsDiv = document.getElementById(`env-details-${{index}}`);
            const arrow = document.getElementById(`arrow-${{index}}`);

            if (detailsDiv) {{
                const isVisible = detailsDiv.style.display !== 'none';
                detailsDiv.style.display = isVisible ? 'none' : 'block';
                if (arrow) {{
                    arrow.textContent = isVisible ? '▶' : '▼';
                }}
            }}
        }}

        function togglePackageDetails(envIndex, pkgIndex) {{
            const detailsDiv = document.getElementById(`pkg-details-${{envIndex}}-${{pkgIndex}}`);
            const arrow = document.getElementById(`pkg-arrow-${{envIndex}}-${{pkgIndex}}`);

            if (detailsDiv) {{
                const isVisible = detailsDiv.style.display !== 'none';
                detailsDiv.style.display = isVisible ? 'none' : 'block';
                if (arrow) {{
                    arrow.textContent = isVisible ? '▶' : '▼';
                }}
            }}
        }}

        async function searchRegistry() {{
            const query = document.getElementById('registry-search-input').value;
            const container = document.getElementById('registry-results');

            container.innerHTML = '<p style="color: var(--text-secondary);">Searching...</p>';

            try {{
                const response = await fetch(`/api/packages/registry?q=${{encodeURIComponent(query)}}`);
                const data = await response.json();

                if (data.error) {{
                    container.innerHTML = `<p style="color: var(--error);">${{data.error}}</p>`;
                    return;
                }}

                if (!data.packages || data.packages.length === 0) {{
                    container.innerHTML = '<p style="color: var(--text-secondary);">No packages found</p>';
                    return;
                }}

                const html = data.packages.map(pkg => `
                    <div class="package-item" style="padding: 15px; background: var(--surface); border: 1px solid var(--border); border-radius: 6px; margin-bottom: 10px;">
                        <div style="display: flex; justify-content: space-between; align-items: center;">
                            <div style="flex: 1;">
                                <div style="font-weight: 600; color: var(--text-primary); font-size: 1.1em;">
                                    ${{pkg.name}}
                                </div>
                                <div style="color: var(--text-secondary); margin-top: 5px; font-size: 0.9em;">
                                    Version: ${{pkg.version}}
                                </div>
                                <div style="color: var(--text-secondary); margin-top: 5px; font-size: 0.9em;">
                                    ${{pkg.description}}
                                </div>
                            </div>
                            <button
                                onclick="showInstallDialog('${{pkg.name}}')"
                                style="padding: 8px 16px; background: var(--accent); color: var(--primary); border: none; border-radius: 4px; cursor: pointer; font-weight: 600; font-family: 'JetBrains Mono', monospace;"
                            >
                                Install
                            </button>
                        </div>
                    </div>
                `).join('');

                container.innerHTML = html;
            }} catch (error) {{
                container.innerHTML = `<p style="color: var(--error);">Search failed: ${{error.message}}</p>`;
            }}
        }}

        // Install dialog state
        let currentInstallPackage = null;
        let currentInstallLocation = 'global';

        async function showInstallDialog(packageName) {{
            currentInstallPackage = packageName;
            document.getElementById('install-pkg-name').textContent = packageName;
            document.getElementById('install-dialog').classList.add('active');

            // Reset to global selection
            selectInstallLocation('global');

            // Load local packages
            await loadLocalPackagesForInstall();
        }}

        function closeInstallDialog() {{
            document.getElementById('install-dialog').classList.remove('active');
            currentInstallPackage = null;
            currentInstallLocation = 'global';
        }}

        function selectInstallLocation(location) {{
            currentInstallLocation = location;

            // Update radio buttons
            document.getElementById('radio-global').checked = (location === 'global');
            document.getElementById('radio-local').checked = (location === 'local');

            // Update visual selection
            document.getElementById('install-option-global').classList.toggle('selected', location === 'global');
            document.getElementById('install-option-local').classList.toggle('selected', location === 'local');

            // Show/hide local package dropdown
            const dropdown = document.getElementById('local-package-select');
            dropdown.style.display = (location === 'local') ? 'block' : 'none';
        }}

        async function loadLocalPackagesForInstall() {{
            try {{
                const response = await fetch('/api/packages/environments');
                const data = await response.json();

                const dropdown = document.getElementById('local-package-select');
                dropdown.innerHTML = '<option value="">Select a package...</option>';

                if (data.local && data.local.length > 0) {{
                    data.local.forEach(env => {{
                        if (env.packages && env.packages.length > 0) {{
                            env.packages.forEach(pkg => {{
                                const option = document.createElement('option');
                                option.value = `${{env.path}}/.horus/packages/${{pkg.name}}`;
                                option.textContent = `${{env.name}} → ${{pkg.name}}`;
                                dropdown.appendChild(option);
                            }});
                        }}
                    }});
                }}
            }} catch (error) {{
                console.error('Failed to load local packages:', error);
            }}
        }}

        async function confirmInstall() {{
            if (!currentInstallPackage) return;

            let target = 'global';

            if (currentInstallLocation === 'local') {{
                const dropdown = document.getElementById('local-package-select');
                target = dropdown.value;

                if (!target) {{
                    alert('Please select a package to install into');
                    return;
                }}
            }}

            closeInstallDialog();

            try {{
                const response = await fetch('/api/packages/install', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{
                        package: currentInstallPackage,
                        target: target
                    }})
                }});

                const data = await response.json();

                if (data.success) {{
                    alert(`Successfully installed ${{currentInstallPackage}}`);
                    loadEnvironments();
                }} else {{
                    alert(`Failed to install: ${{data.error}}`);
                }}
            }} catch (error) {{
                alert(`Installation failed: ${{error.message}}`);
            }}
        }}

        async function installPackage(packageName) {{
            try {{
                const response = await fetch('/api/packages/install', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ package: packageName }})
                }});

                const data = await response.json();

                if (data.success) {{
                    alert(`Successfully installed ${{packageName}}`);
                    loadEnvironments();
                }} else {{
                    alert(`Failed to install: ${{data.error}}`);
                }}
            }} catch (error) {{
                alert(`Installation failed: ${{error.message}}`);
            }}
        }}

        async function uninstallPackage(parentPackage, packageName, event) {{
            // Stop event propagation to prevent toggling the package details
            event.stopPropagation();

            if (!confirm(`Uninstall ${{packageName}} from ${{parentPackage}}?`)) return;

            try {{
                const response = await fetch('/api/packages/uninstall', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{
                        parent_package: parentPackage,
                        package: packageName
                    }})
                }});

                const data = await response.json();

                if (data.success) {{
                    alert(`Successfully uninstalled ${{packageName}}`);
                    loadEnvironments();
                }} else {{
                    alert(`Failed to uninstall: ${{data.error}}`);
                }}
            }} catch (error) {{
                alert(`Uninstallation failed: ${{error.message}}`);
            }}
        }}

        async function publishPackage() {{
            if (!confirm('Publish current directory as a package?')) return;

            try {{
                const response = await fetch('/api/packages/publish', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{}})
                }});

                const data = await response.json();

                if (data.success) {{
                    alert('Package published successfully!');
                    loadEnvironments();
                }} else {{
                    alert(`Failed to publish: ${{data.error}}`);
                }}
            }} catch (error) {{
                alert(`Error: ${{error.message}}`);
            }}
        }}

        // Initialize on tab switch
        function onPackagesTabActivate() {{
            loadGlobalEnvironment();
        }}

        async function loadGlobalEnvironment() {{
            const container = document.getElementById('global-packages-list');
            try {{
                const response = await fetch('/api/packages/environments');
                const data = await response.json();

                if (data.global && data.global.length > 0) {{
                    const html = data.global.map(pkg => `
                        <div style="padding: 12px 15px; background: var(--surface); border: 1px solid var(--border); border-radius: 6px; margin-bottom: 8px;">
                            <div style="display: flex; justify-content: space-between; align-items: center;">
                                <div>
                                    <span style="font-weight: 600; color: var(--text-primary);">${{pkg.name}}</span>
                                    <span style="color: var(--text-secondary); margin-left: 10px; font-size: 0.9em;">v${{pkg.version}}</span>
                                </div>
                            </div>
                        </div>
                    `).join('');
                    container.innerHTML = html;
                }} else {{
                    container.innerHTML = '<p style="color: var(--text-secondary); font-size: 0.9em;">No global packages installed in ~/.horus/cache</p>';
                }}
            }} catch (error) {{
                container.innerHTML = `<p style="color: var(--error);">Failed to load global packages: ${{error.message}}</p>`;
            }}
        }}

        async function loadLocalEnvironments() {{
            const container = document.getElementById('local-environments-list');
            try {{
                const response = await fetch('/api/packages/environments');
                const data = await response.json();

                if (data.local && data.local.length > 0) {{
                    const html = data.local.map((env, index) => {{
                        let expandableContent = '';
                        if (env.packages && env.packages.length > 0) {{
                            expandableContent = `
                                <div id="env-details-${{index}}" style="display: none; margin-top: 10px; padding-top: 10px; border-top: 1px solid var(--border);">
                                    <div style="color: var(--text-secondary); margin-bottom: 8px; font-size: 0.9em;">
                                         Packages in this environment:
                                    </div>
                                    ${{env.packages.map((p, pidx) => `
                                        <div style="margin-bottom: 6px;">
                                            <div onclick="togglePackageDetails(${{index}}, ${{pidx}})" style="padding: 8px 12px; background: var(--primary); border: 1px solid var(--border); border-radius: 4px; display: flex; justify-content: space-between; align-items: center; cursor: pointer; transition: background 0.2s;">
                                                <div style="display: flex; align-items: center; gap: 8px;">
                                                    <span id="pkg-arrow-${{index}}-${{pidx}}" style="color: var(--accent); font-size: 0.8em;">▶</span>
                                                    <span style="font-weight: 500;">${{p.name}}</span>
                                                </div>
                                                <span style="color: var(--text-secondary); font-size: 0.85em;">v${{p.version}}</span>
                                            </div>
                                            <div id="pkg-details-${{index}}-${{pidx}}" style="display: none; padding: 10px 12px; margin-left: 20px; background: var(--dark-bg); border-left: 2px solid var(--accent); border-radius: 4px; margin-top: 4px;">
                                                <div style="color: var(--text-secondary); font-size: 0.85em; margin-bottom: 8px;"><strong>Installed Packages (${{p.installed_packages?.length || 0}}):</strong></div>
                                                ${{p.installed_packages && p.installed_packages.length > 0 ? `
                                                    <div style="display: flex; flex-direction: column; gap: 4px;">
                                                        ${{p.installed_packages.map(pkg => `
                                                            <div style="padding: 6px 10px; background: var(--surface); border: 1px solid var(--border); border-radius: 4px; display: flex; justify-content: space-between; align-items: center; gap: 10px;">
                                                                <div style="display: flex; align-items: center; gap: 10px; flex: 1;">
                                                                    <span style="color: var(--text-primary); font-size: 0.85em;">${{pkg.name}}</span>
                                                                    <span style="color: var(--text-tertiary); font-size: 0.75em;">v${{pkg.version}}</span>
                                                                </div>
                                                                <button
                                                                    onclick="uninstallPackage('${{p.name}}', '${{pkg.name}}', event)"
                                                                    style="padding: 4px 10px; background: var(--error, #ff4444); color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 0.75em; font-weight: 600; transition: opacity 0.2s;"
                                                                    onmouseover="this.style.opacity='0.8'"
                                                                    onmouseout="this.style.opacity='1'"
                                                                >
                                                                    Uninstall
                                                                </button>
                                                            </div>
                                                        `).join('')}}
                                                    </div>
                                                ` : '<div style="color: var(--text-tertiary); font-size: 0.85em;">No packages installed</div>'}}
                                            </div>
                                        </div>
                                    `).join('')}}
                                </div>
                            `;
                        }}

                        return `
                            <div class="package-item" style="padding: 15px; background: var(--surface); border: 1px solid var(--border); border-radius: 6px; margin-bottom: 10px; ${{env.package_count > 0 ? 'cursor: pointer;' : ''}}">
                                <div style="display: flex; justify-content: space-between; align-items: center;" onclick="${{env.package_count > 0 ? `toggleEnvDetails(${{index}})` : ''}}">
                                    <div style="flex: 1;">
                                        <div style="display: flex; align-items: center; gap: 8px;">
                                            <span style="font-weight: 600; color: var(--text-primary); font-size: 1.05em;">
                                                ${{env.name}}
                                            </span>
                                            ${{env.package_count > 0 ? '<span id="arrow-' + index + '" style="color: var(--accent); font-size: 0.9em;">▶</span>' : ''}}
                                        </div>
                                        <div style="color: var(--text-secondary); margin-top: 5px; font-size: 0.85em;">
                                            ${{env.path}} • ${{env.package_count}} package(s)
                                        </div>
                                    </div>
                                </div>
                                ${{expandableContent}}
                            </div>
                        `;
                    }}).join('');
                    container.innerHTML = html;
                }} else {{
                    container.innerHTML = '<p style="color: var(--text-secondary); font-size: 0.9em;">No local HORUS environments found</p>';
                }}
            }} catch (error) {{
                container.innerHTML = `<p style="color: var(--error);">Failed to load local environments: ${{error.message}}</p>`;
            }}
        }}

        // Remote deployment functions
        async function deployToRobot() {{
            const robotAddr = document.getElementById('robot-addr').value.trim();
            const robotFile = document.getElementById('robot-file').value.trim();
            const statusDiv = document.getElementById('deploy-status');

            if (!robotAddr) {{
                alert('Please enter a robot address');
                return;
            }}

            statusDiv.style.display = 'block';
            statusDiv.style.background = 'var(--surface)';
            statusDiv.style.border = '1px solid var(--border)';
            statusDiv.innerHTML = '<p style="color: var(--text-secondary); margin: 0;">Deploying...</p>';

            try {{
                const response = await fetch('/api/remote/deploy', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{
                        robot_addr: robotAddr,
                        file: robotFile || null
                    }})
                }});

                const data = await response.json();

                if (data.success) {{
                    statusDiv.style.background = 'rgba(0, 255, 136, 0.1)';
                    statusDiv.style.border = '1px solid var(--success)';
                    statusDiv.innerHTML = `
                        <p style="color: var(--success); margin: 0; font-weight: 600;">Deployment Successful</p>
                        <p style="color: var(--text-secondary); margin: 5px 0 0 0; font-size: 0.9em;">${{data.message}}</p>
                    `;
                }} else {{
                    statusDiv.style.background = 'rgba(255, 68, 68, 0.1)';
                    statusDiv.style.border = '1px solid #ff4444';
                    statusDiv.innerHTML = `
                        <p style="color: #ff4444; margin: 0; font-weight: 600;">Deployment Failed</p>
                        <p style="color: var(--text-secondary); margin: 5px 0 0 0; font-size: 0.9em;">${{data.error}}</p>
                    `;
                }}
            }} catch (error) {{
                statusDiv.style.background = 'rgba(255, 68, 68, 0.1)';
                statusDiv.style.border = '1px solid #ff4444';
                statusDiv.innerHTML = `
                    <p style="color: #ff4444; margin: 0; font-weight: 600;">Error</p>
                    <p style="color: var(--text-secondary); margin: 5px 0 0 0; font-size: 0.9em;">${{error.message}}</p>
                `;
            }}
        }}

        // Connect to robot
        async function connectToRobot() {{
            const robotAddr = document.getElementById('robot-addr').value.trim();
            const statusDiv = document.getElementById('robot-status');

            if (!robotAddr) {{
                alert('Please enter a robot address');
                return;
            }}

            statusDiv.innerHTML = 'Connecting...';

            try {{
                const response = await fetch('/api/remote/deployments', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ robot_addr: robotAddr }})
                }});

                if (response.ok) {{
                    statusDiv.innerHTML = 'Connected';
                    statusDiv.style.color = 'var(--success)';
                    refreshDeployments();
                    refreshHardware();
                }} else {{
                    statusDiv.innerHTML = 'Connection failed';
                    statusDiv.style.color = '#ff4444';
                }}
            }} catch (error) {{
                statusDiv.innerHTML = `Error: ${{error.message}}`;
                statusDiv.style.color = '#ff4444';
            }}
        }}

        // Refresh deployments list
        async function refreshDeployments() {{
            const robotAddr = document.getElementById('robot-addr').value.trim();
            const listDiv = document.getElementById('deployments-list');

            if (!robotAddr) {{
                alert('Please enter a robot address first');
                return;
            }}

            listDiv.innerHTML = '<p style="color: var(--text-secondary); text-align: center;">Loading...</p>';

            try {{
                const response = await fetch('/api/remote/deployments', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ robot_addr: robotAddr }})
                }});

                const data = await response.json();

                if (data && data.length > 0) {{
                    listDiv.innerHTML = data.map(d => `
                        <div style="background: var(--surface); border: 1px solid var(--border); border-radius: 8px; padding: 15px; margin-bottom: 10px;">
                            <div style="display: flex; justify-content: space-between; align-items: flex-start;">
                                <div style="flex: 1;">
                                    <div style="font-weight: 600; color: var(--accent); margin-bottom: 5px;">${{d.deployment_id || d.id}}</div>
                                    <div style="color: var(--text-secondary); font-size: 0.9em; margin-bottom: 5px;">
                                        Status: <span style="color: ${{d.status === 'Running' ? 'var(--success)' : '#ffa500'}}">${{d.status}}</span>
                                    </div>
                                    ${{d.pid ? `<div style="color: var(--text-tertiary); font-size: 0.85em;">PID: ${{d.pid}}</div>` : ''}}
                                    ${{d.cpu ? `<div style="color: var(--text-tertiary); font-size: 0.85em;">CPU: ${{d.cpu}}% | Memory: ${{d.memory}}</div>` : ''}}
                                </div>
                                ${{d.status === 'Running' ? `
                                <button
                                    onclick="stopDeployment('${{d.deployment_id || d.id}}')"
                                    style="padding: 6px 12px; background: #ff4444; color: white; border: none; border-radius: 6px; cursor: pointer; font-size: 0.85em;"
                                >
                                    Stop
                                </button>
                                ` : ''}}
                            </div>
                        </div>
                    `).join('');
                }} else {{
                    listDiv.innerHTML = '<p style="color: var(--text-tertiary); text-align: center; padding: 20px;">No active deployments</p>';
                }}
            }} catch (error) {{
                listDiv.innerHTML = `<p style="color: #ff4444; text-align: center;">Error: ${{error.message}}</p>`;
            }}
        }}

        // Refresh hardware info
        async function refreshHardware() {{
            const robotAddr = document.getElementById('robot-addr').value.trim();
            const hardwareDiv = document.getElementById('hardware-info');

            if (!robotAddr) {{
                alert('Please enter a robot address first');
                return;
            }}

            hardwareDiv.innerHTML = '<p style="color: var(--text-secondary); text-align: center;">Loading...</p>';

            try {{
                const response = await fetch('/api/remote/hardware', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ robot_addr: robotAddr }})
                }});

                const data = await response.json();

                if (data) {{
                    hardwareDiv.innerHTML = `
                        <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 15px;">
                            <div style="background: var(--surface); border: 1px solid var(--border); border-radius: 8px; padding: 15px;">
                                <div style="color: var(--text-tertiary); font-size: 0.85em; margin-bottom: 5px;">System</div>
                                <div style="color: var(--text-primary); font-weight: 600;">${{data.hostname || 'Unknown'}}</div>
                                <div style="color: var(--text-secondary); font-size: 0.85em;">${{data.os || ''}} ${{data.arch || ''}}</div>
                            </div>
                            <div style="background: var(--surface); border: 1px solid var(--border); border-radius: 8px; padding: 15px;">
                                <div style="color: var(--text-tertiary); font-size: 0.85em; margin-bottom: 5px;">CPU</div>
                                <div style="color: var(--text-primary); font-weight: 600;">${{data.cpu_count || 0}} cores</div>
                            </div>
                            <div style="background: var(--surface); border: 1px solid var(--border); border-radius: 8px; padding: 15px;">
                                <div style="color: var(--text-tertiary); font-size: 0.85em; margin-bottom: 5px;">Memory</div>
                                <div style="color: var(--text-primary); font-weight: 600;">${{data.total_memory || 'Unknown'}}</div>
                            </div>
                            ${{data.cameras && data.cameras.length > 0 ? `
                            <div style="background: var(--surface); border: 1px solid var(--border); border-radius: 8px; padding: 15px;">
                                <div style="color: var(--text-tertiary); font-size: 0.85em; margin-bottom: 5px;">Cameras</div>
                                <div style="color: var(--text-primary); font-weight: 600;">${{data.cameras.length}}</div>
                            </div>
                            ` : ''}}
                            ${{data.gpio_available ? `
                            <div style="background: var(--surface); border: 1px solid var(--border); border-radius: 8px; padding: 15px;">
                                <div style="color: var(--text-tertiary); font-size: 0.85em; margin-bottom: 5px;">GPIO</div>
                                <div style="color: var(--success); font-weight: 600;">Available</div>
                            </div>
                            ` : ''}}
                        </div>
                    `;
                }} else {{
                    hardwareDiv.innerHTML = '<p style="color: #ff4444; text-align: center;">Failed to load hardware info</p>';
                }}
            }} catch (error) {{
                hardwareDiv.innerHTML = `<p style="color: #ff4444; text-align: center;">Error: ${{error.message}}</p>`;
            }}
        }}

        // Stop deployment
        async function stopDeployment(deploymentId) {{
            const robotAddr = document.getElementById('robot-addr').value.trim();

            if (!confirm(`Stop deployment ${{deploymentId}}?`)) return;

            try {{
                const response = await fetch('/api/remote/stop', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{
                        robot_addr: robotAddr,
                        deployment_id: deploymentId
                    }})
                }});

                const data = await response.json();

                if (data.success || data.status === 'stopped') {{
                    refreshDeployments();
                }} else {{
                    alert(`Failed to stop: ${{data.error || data.message}}`);
                }}
            }} catch (error) {{
                alert(`Error: ${{error.message}}`);
            }}
        }}

        // Enter key support for search
        document.getElementById('search-input')?.addEventListener('keypress', (e) => {{
            if (e.key === 'Enter') searchPackages();
        }});

        // === Parameter Management Functions ===
        let allParams = [];
        let editingParam = null;

        async function refreshParams() {{
            try {{
                const response = await fetch('/api/params');
                const data = await response.json();

                if (data.success) {{
                    allParams = data.params;
                    renderParams(allParams);
                }}
            }} catch (error) {{
                console.error('Failed to fetch params:', error);
            }}
        }}

        function renderParams(params) {{
            const tbody = document.getElementById('params-table-body');

            if (params.length === 0) {{
                tbody.innerHTML = `
                    <tr>
                        <td colspan="4" style="padding: 2rem; text-align: center; color: var(--text-tertiary);">
                            No parameters found. Click "Add Parameter" to create one.
                        </td>
                    </tr>
                `;
                return;
            }}

            tbody.innerHTML = params.map(param => {{
                const valueDisplay = typeof param.value === 'object'
                    ? JSON.stringify(param.value)
                    : String(param.value);

                return `
                    <tr style="border-bottom: 1px solid var(--border); transition: background 0.2s;" onmouseover="this.style.background='var(--surface)'" onmouseout="this.style.background='transparent'">
                        <td style="padding: 0.75rem; font-weight: 600; color: var(--accent);">${{param.key}}</td>
                        <td style="padding: 0.75rem; color: var(--text-primary); font-family: 'JetBrains Mono', monospace;">${{valueDisplay}}</td>
                        <td style="padding: 0.75rem; color: var(--text-secondary);">
                            <span style="background: var(--surface); padding: 0.25rem 0.5rem; border-radius: 4px; font-size: 0.85rem;">${{param.type}}</span>
                        </td>
                        <td style="padding: 0.75rem; text-align: right;">
                            <button onclick="editParam('${{param.key}}')" style="padding: 0.25rem 0.75rem; background: var(--accent); color: white; border: none; border-radius: 6px; cursor: pointer; margin-right: 0.5rem; font-size: 0.85rem;">Edit</button>
                            <button onclick="deleteParam('${{param.key}}')" style="padding: 0.25rem 0.75rem; background: var(--error); color: white; border: none; border-radius: 6px; cursor: pointer; font-size: 0.85rem;">Delete</button>
                        </td>
                    </tr>
                `;
            }}).join('');
        }}

        function showAddParamDialog() {{
            editingParam = null;
            document.getElementById('dialog-title').textContent = 'Add Parameter';
            document.getElementById('param-key').value = '';
            document.getElementById('param-key').disabled = false;
            document.getElementById('param-type').value = 'number';
            document.getElementById('param-value').value = '';
            document.getElementById('param-dialog').style.display = 'flex';
        }}

        async function editParam(key) {{
            editingParam = key;
            const param = allParams.find(p => p.key === key);

            if (param) {{
                document.getElementById('dialog-title').textContent = 'Edit Parameter';
                document.getElementById('param-key').value = key;
                document.getElementById('param-key').disabled = true;
                document.getElementById('param-type').value = param.type;
                document.getElementById('param-value').value = typeof param.value === 'object'
                    ? JSON.stringify(param.value)
                    : String(param.value);
                document.getElementById('param-dialog').style.display = 'flex';
            }}
        }}

        function closeParamDialog() {{
            document.getElementById('param-dialog').style.display = 'none';
            editingParam = null;
        }}

        function updateValueInput() {{
            const type = document.getElementById('param-type').value;
            const valueInput = document.getElementById('param-value');

            if (type === 'boolean') {{
                valueInput.value = 'false';
                valueInput.placeholder = 'true or false';
            }} else if (type === 'number') {{
                valueInput.value = '0';
                valueInput.placeholder = 'Enter number';
            }} else {{
                valueInput.value = '';
                valueInput.placeholder = 'Enter text';
            }}
        }}

        async function saveParam() {{
            const key = document.getElementById('param-key').value.trim();
            const type = document.getElementById('param-type').value;
            const valueStr = document.getElementById('param-value').value.trim();

            if (!key) {{
                alert('Parameter key is required');
                return;
            }}

            let value;
            try {{
                if (type === 'number') {{
                    value = parseFloat(valueStr);
                    if (isNaN(value)) throw new Error('Invalid number');
                }} else if (type === 'boolean') {{
                    value = valueStr.toLowerCase() === 'true';
                }} else {{
                    value = valueStr;
                }}
            }} catch (e) {{
                alert('Invalid value for type ' + type);
                return;
            }}

            try {{
                const response = await fetch(`/api/params/${{encodeURIComponent(key)}}`, {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ value }})
                }});

                const data = await response.json();

                if (data.success) {{
                    closeParamDialog();
                    refreshParams();
                }} else {{
                    alert('Error: ' + data.error);
                }}
            }} catch (error) {{
                alert('Failed to save parameter: ' + error.message);
            }}
        }}

        async function deleteParam(key) {{
            if (!confirm(`Are you sure you want to delete parameter "${{key}}"?`)) {{
                return;
            }}

            try {{
                const response = await fetch(`/api/params/${{encodeURIComponent(key)}}`, {{
                    method: 'DELETE'
                }});

                const data = await response.json();

                if (data.success) {{
                    refreshParams();
                }} else {{
                    alert('Error: ' + data.error);
                }}
            }} catch (error) {{
                alert('Failed to delete parameter: ' + error.message);
            }}
        }}

        async function exportParams() {{
            try {{
                const response = await fetch('/api/params/export', {{ method: 'POST' }});
                const data = await response.json();

                if (data.success) {{
                    const blob = new Blob([data.data], {{ type: 'text/yaml' }});
                    const url = URL.createObjectURL(blob);
                    const a = document.createElement('a');
                    a.href = url;
                    a.download = 'horus_params_' + new Date().toISOString().split('T')[0] + '.yaml';
                    a.click();
                    URL.revokeObjectURL(url);
                }} else {{
                    alert('Export failed: ' + data.error);
                }}
            }} catch (error) {{
                alert('Failed to export parameters: ' + error.message);
            }}
        }}

        function showImportDialog() {{
            document.getElementById('import-format').value = 'yaml';
            document.getElementById('import-data').value = '';
            document.getElementById('import-dialog').style.display = 'flex';
        }}

        function closeImportDialog() {{
            document.getElementById('import-dialog').style.display = 'none';
        }}

        async function importParams() {{
            const format = document.getElementById('import-format').value;
            const data = document.getElementById('import-data').value.trim();

            if (!data) {{
                alert('Please paste data to import');
                return;
            }}

            try {{
                const response = await fetch('/api/params/import', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ format, data }})
                }});

                const result = await response.json();

                if (result.success) {{
                    alert(result.message);
                    closeImportDialog();
                    refreshParams();
                }} else {{
                    alert('Import failed: ' + result.error);
                }}
            }} catch (error) {{
                alert('Failed to import parameters: ' + error.message);
            }}
        }}

        // Search parameters
        document.getElementById('param-search')?.addEventListener('input', (e) => {{
            const query = e.target.value.toLowerCase();

            if (!query) {{
                renderParams(allParams);
                return;
            }}

            const filtered = allParams.filter(param =>
                param.key.toLowerCase().includes(query) ||
                String(param.value).toLowerCase().includes(query)
            );

            renderParams(filtered);
        }});

        // Load parameters when params tab is shown
        const paramsTabObserver = new MutationObserver((mutations) => {{
            mutations.forEach((mutation) => {{
                if (mutation.target.classList.contains('active') &&
                    mutation.target.id === 'tab-params') {{
                    refreshParams();
                }}
            }});
        }});

        const paramsTab = document.getElementById('tab-params');
        if (paramsTab) {{
            paramsTabObserver.observe(paramsTab, {{
                attributes: true,
                attributeFilter: ['class']
            }});
        }}
    </script>

    <!-- Install Dialog -->
    <div class="install-dialog" id="install-dialog">
        <div class="install-dialog-content">
            <div class="install-dialog-header">
                <h3>Install Package: <span id="install-pkg-name"></span></h3>
                <button onclick="closeInstallDialog()" style="background: transparent; border: none; color: var(--text-secondary); font-size: 1.5rem; cursor: pointer; padding: 0; width: 30px; height: 30px;">&times;</button>
            </div>
            <div class="install-dialog-body">
                <p style="color: var(--text-secondary); margin-bottom: 1rem;">Where would you like to install this package?</p>

                <div class="install-option" onclick="selectInstallLocation('global')" id="install-option-global">
                    <input type="radio" name="install-location" id="radio-global" value="global">
                    <label for="radio-global" style="cursor: pointer; color: var(--text-primary); font-weight: 600;">
                        Global Installation
                    </label>
                    <div style="color: var(--text-secondary); font-size: 0.85em; margin-top: 0.5rem; margin-left: 24px;">
                        Available to all HORUS projects
                    </div>
                </div>

                <div class="install-option" onclick="selectInstallLocation('local')" id="install-option-local">
                    <input type="radio" name="install-location" id="radio-local" value="local">
                    <label for="radio-local" style="cursor: pointer; color: var(--text-primary); font-weight: 600;">
                        Local Installation
                    </label>
                    <div style="color: var(--text-secondary); font-size: 0.85em; margin-top: 0.5rem; margin-left: 24px;">
                        Install into a specific package
                    </div>
                    <select id="local-package-select" class="local-packages-select" style="display: none;">
                        <option value="">Select a package...</option>
                    </select>
                </div>
            </div>
            <div class="install-dialog-footer">
                <button onclick="closeInstallDialog()" style="padding: 10px 20px; background: var(--surface); color: var(--text-secondary); border: 1px solid var(--border); border-radius: 6px; cursor: pointer; font-weight: 600;">
                    Cancel
                </button>
                <button onclick="confirmInstall()" style="padding: 10px 20px; background: var(--accent); color: var(--primary); border: none; border-radius: 6px; cursor: pointer; font-weight: 600;">
                    Install
                </button>
            </div>
        </div>
    </div>

    <!-- Log Panel -->
    <div id="log-panel" class="log-panel">
        <div class="log-panel-header">
            <div id="log-panel-title" class="log-panel-title">Logs</div>
            <button class="log-panel-close" onclick="closeLogPanel()">✕ Close</button>
        </div>
        <div id="log-panel-content" class="log-panel-content">
            <!-- Logs will be loaded here -->
        </div>
    </div>
</body>
</html>"#,
        port = port
    )
}
