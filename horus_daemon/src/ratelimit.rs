use axum::{
    extract::{ConnectInfo, Request},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct RateLimiter {
    requests: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window: Duration::from_secs(window_secs),
        }
    }

    pub fn check_rate_limit(&self, ip: String) -> bool {
        let mut requests = self.requests.lock().unwrap();
        let now = Instant::now();

        let entry = requests.entry(ip).or_default();

        // Remove old requests outside the window
        entry.retain(|&time| now.duration_since(time) < self.window);

        if entry.len() < self.max_requests {
            entry.push(now);
            true
        } else {
            false
        }
    }

    /// Middleware function for rate limiting
    pub async fn middleware(
        limiter: Arc<RateLimiter>,
        ConnectInfo(addr): ConnectInfo<SocketAddr>,
        request: Request,
        next: Next,
    ) -> Result<Response, StatusCode> {
        let ip = addr.ip().to_string();

        if limiter.check_rate_limit(ip) {
            Ok(next.run(request).await)
        } else {
            Err(StatusCode::TOO_MANY_REQUESTS)
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        // Default: 100 requests per minute
        Self::new(100, 60)
    }
}
