//! Security module for HORUS dashboard
//!
//! Provides password-based authentication, TLS, and security middleware.

pub mod auth;
pub mod middleware;
pub mod tls;

pub use auth::AuthService;
pub use middleware::{security_headers_middleware, session_middleware};
pub use tls::TlsConfig;
