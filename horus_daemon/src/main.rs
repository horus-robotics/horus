mod auth;
mod deploy;
mod discovery;
mod executor;
mod hardware;
mod process;
mod stream;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    middleware,
    routing::{get, post},
    Json, Router,
};
use executor::ProcessExecutor;
use horus_core::core::log_buffer::{LogEntry, GLOBAL_LOG_BUFFER};
use process::{ProcessInfo, ProcessRegistry};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<ProcessRegistry>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "horus_daemon=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize process registry and executor
    let registry = Arc::new(ProcessRegistry::new());
    let executor = Arc::new(ProcessExecutor::new(registry.clone()));

    // Start background monitoring
    executor.clone().start_monitoring();
    ProcessExecutor::start_cleanup(registry.clone());

    // Start mDNS discovery broadcasting
    if let Ok(discovery) = discovery::DiscoveryService::new() {
        let hostname = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "horus-robot".to_string());

        if let Err(e) = discovery.start_broadcasting(hostname) {
            tracing::warn!("Failed to start mDNS broadcasting: {}", e);
        }
    } else {
        tracing::warn!("Failed to initialize mDNS service");
    }

    let state = AppState {
        registry: registry.clone(),
    };

    let mut app = Router::new()
        .route("/health", get(health))
        .route("/deploy", post(handle_deploy))
        .route("/logs", get(get_all_logs))
        .route("/logs/:deployment_id", get(get_deployment_logs))
        .route("/deployments", get(list_deployments))
        .route("/deployments/:deployment_id", get(get_deployment))
        .route("/deployments/:deployment_id/stop", post(stop_deployment))
        .route(
            "/stream/:deployment_id",
            get(stream::stream_deployment_logs),
        )
        .route("/hardware", get(get_hardware))
        .with_state(state);

    // Optionally add authentication middleware
    if auth::is_auth_enabled() {
        tracing::info!(" API authentication enabled");
        app = app.layer(middleware::from_fn(auth::auth_middleware));
    }

    // Add CORS layer
    let app = app.layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!(" HORUS daemon listening on {}", addr);
    tracing::info!(" Ready to receive deployments");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> &'static str {
    "OK"
}

async fn get_all_logs() -> (StatusCode, Json<Vec<LogEntry>>) {
    let logs = GLOBAL_LOG_BUFFER.get_all();
    (StatusCode::OK, Json(logs))
}

async fn get_deployment_logs(
    Path(deployment_id): Path<String>,
) -> (StatusCode, Json<Vec<LogEntry>>) {
    let node_name = format!("deploy-{}", deployment_id);
    let logs = GLOBAL_LOG_BUFFER.get_for_node(&node_name);
    (StatusCode::OK, Json(logs))
}

async fn handle_deploy(
    State(state): State<AppState>,
    body: axum::body::Bytes,
) -> Result<Json<deploy::DeployResponse>, StatusCode> {
    deploy::handle_deploy(body, state.registry).await
}

async fn list_deployments(State(state): State<AppState>) -> (StatusCode, Json<Vec<ProcessInfo>>) {
    let deployments = state.registry.list();
    (StatusCode::OK, Json(deployments))
}

async fn get_deployment(
    State(state): State<AppState>,
    Path(deployment_id): Path<String>,
) -> (StatusCode, Json<Option<ProcessInfo>>) {
    let deployment = state.registry.get(&deployment_id);
    (StatusCode::OK, Json(deployment))
}

async fn stop_deployment(
    State(state): State<AppState>,
    Path(deployment_id): Path<String>,
) -> (StatusCode, String) {
    match state.registry.stop(&deployment_id) {
        Ok(_) => (
            StatusCode::OK,
            format!("Deployment {} stopped", deployment_id),
        ),
        Err(e) => (StatusCode::BAD_REQUEST, e),
    }
}

async fn get_hardware() -> (StatusCode, Json<hardware::HardwareInfo>) {
    let info = hardware::detect_hardware();
    (StatusCode::OK, Json(info))
}
