//! Security module for HORUS dashboard
//!
//! Provides token-based authentication, TLS, and security middleware.

pub mod auth;
pub mod middleware;
pub mod tls;

pub use auth::AuthService;
pub use middleware::{security_headers_middleware, token_middleware};
pub use tls::TlsConfig;
