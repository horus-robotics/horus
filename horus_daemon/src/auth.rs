use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use std::env;

const DEFAULT_API_KEY: &str = "horus-default-key";

/// Middleware to check API key authentication
pub async fn auth_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Get expected API key from environment or use default
    let expected_key = env::var("HORUS_API_KEY").unwrap_or_else(|_| DEFAULT_API_KEY.to_string());

    // Skip auth for health check
    if request.uri().path() == "/health" {
        return Ok(next.run(request).await);
    }

    // Check Authorization header
    if let Some(auth_header) = headers.get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            // Support both "Bearer TOKEN" and just "TOKEN"
            let token = auth_str.strip_prefix("Bearer ").unwrap_or(auth_str);

            if token == expected_key {
                return Ok(next.run(request).await);
            }
        }
    }

    // Check X-API-Key header
    if let Some(api_key) = headers.get("X-API-Key") {
        if let Ok(key_str) = api_key.to_str() {
            if key_str == expected_key {
                return Ok(next.run(request).await);
            }
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}

/// Check if authentication is enabled
pub fn is_auth_enabled() -> bool {
    env::var("HORUS_AUTH_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .parse()
        .unwrap_or(false)
}
