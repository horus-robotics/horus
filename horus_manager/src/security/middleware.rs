//! Security middleware for Axum - token validation and security headers

use super::auth::AuthService;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

/// Token validation middleware - checks for ?token=xxx in URL
pub async fn token_middleware(
    State(auth_service): State<Arc<AuthService>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract token from query parameter
    let query = req.uri().query().unwrap_or("");
    let token = query
        .split('&')
        .find_map(|param| {
            let mut parts = param.split('=');
            if parts.next() == Some("token") {
                parts.next()
            } else {
                None
            }
        })
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Validate token
    if !auth_service.validate_token(token) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(req).await)
}

/// Security headers middleware
pub async fn security_headers_middleware(req: Request<Body>, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    // Content Security Policy
    headers.insert(
        "Content-Security-Policy",
        "default-src 'self' 'unsafe-inline' 'unsafe-eval'; connect-src 'self'"
            .parse()
            .unwrap(),
    );

    // Prevent clickjacking
    headers.insert("X-Frame-Options", "DENY".parse().unwrap());

    // Prevent MIME sniffing
    headers.insert("X-Content-Type-Options", "nosniff".parse().unwrap());

    // Enable XSS protection
    headers.insert("X-XSS-Protection", "1; mode=block".parse().unwrap());

    // Referrer policy
    headers.insert(
        "Referrer-Policy",
        "strict-origin-when-cross-origin".parse().unwrap(),
    );

    // HSTS (only for HTTPS)
    headers.insert(
        "Strict-Transport-Security",
        "max-age=31536000; includeSubDomains".parse().unwrap(),
    );

    response
}
