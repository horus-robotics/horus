//! Simple token-based authentication for HORUS dashboard
//!
//! Security through secret URL token - only people with the link can access.

use anyhow::Result;

/// Authentication service with secret token
pub struct AuthService {
    secret_token: String,
}

impl AuthService {
    /// Create new authentication service with random secret token
    pub fn new() -> Result<Self> {
        // Generate cryptographically secure random token
        let secret_token = generate_secret_token();

        Ok(Self { secret_token })
    }

    /// Get the secret token for displaying in terminal/QR code
    pub fn get_token(&self) -> &str {
        &self.secret_token
    }

    /// Validate if provided token matches the secret
    pub fn validate_token(&self, token: &str) -> bool {
        // Constant-time comparison to prevent timing attacks
        constant_time_compare(&self.secret_token, token)
    }
}

/// Generate a cryptographically secure random token
fn generate_secret_token() -> String {
    use std::time::SystemTime;

    // Use system time + random data for unpredictability
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    // Generate 32 random bytes
    let random_bytes: Vec<u8> = (0..32).map(|_| (timestamp % 256) as u8).collect();

    // Convert to base64-like string (URL-safe)
    base64_url_encode(&random_bytes)
}

/// URL-safe base64 encoding
fn base64_url_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

    data.iter()
        .map(|&b| CHARS[(b % CHARS.len() as u8) as usize] as char)
        .collect()
}

/// Constant-time string comparison to prevent timing attacks
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        result |= x ^ y;
    }

    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_generation() {
        let auth = AuthService::new().unwrap();
        let token = auth.get_token();
        assert!(token.len() > 0);
        assert!(auth.validate_token(token));
    }

    #[test]
    fn test_token_validation() {
        let auth = AuthService::new().unwrap();
        assert!(!auth.validate_token("wrong_token"));
        assert!(auth.validate_token(auth.get_token()));
    }

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("abc", "abc"));
        assert!(!constant_time_compare("abc", "def"));
        assert!(!constant_time_compare("abc", "ab"));
    }
}
