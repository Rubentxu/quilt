//! Rate Limiting Middleware
//!
//! In-memory rate limiter using token bucket algorithm.
//! Tracks requests per client IP within a time window.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};

/// Rate limiter state using token bucket algorithm.
#[derive(Clone)]
pub struct RateLimiter {
    requests: Arc<RwLock<HashMap<String, (usize, Instant)>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    /// Create a new rate limiter.
    ///
    /// - `max_requests`: Maximum requests allowed per `window`
    /// - `window`: Time window for request counting
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            requests: Arc::new(RwLock::new(HashMap::new())),
            max_requests,
            window,
        }
    }

    /// Check if a request from the given key should be allowed.
    /// Returns `true` if allowed, `false` if rate limited.
    pub async fn check(&self, key: &str) -> bool {
        let mut requests = self.requests.write().await;
        let now = Instant::now();

        // Clean up expired entries
        if let Some((count, start)) = requests.get(key) {
            if start.elapsed() > self.window {
                requests.remove(key);
            } else if *count >= self.max_requests {
                return false;
            }
        }

        // Record this request
        requests
            .entry(key.to_string())
            .and_modify(|(count, _)| *count += 1)
            .or_insert((1, now));

        true
    }

    /// Get the configured limits for response headers.
    pub fn limits(&self) -> (usize, u64) {
        (self.max_requests, self.window.as_secs())
    }
}

/// Extract client IP from request, handling proxies via X-Forwarded-For.
fn extract_client_ip(request: &Request) -> String {
    // Check X-Forwarded-For header first (for proxy setups)
    if let Some(forwarded) = request.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            // Take the first IP in the chain (original client)
            if let Some(ip) = forwarded_str.split(',').next() {
                return ip.trim().to_string();
            }
        }
    }

    // Fall back to X-Real-IP
    if let Some(real_ip) = request.headers().get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            return ip_str.trim().to_string();
        }
    }

    // Fall back to connection info (from axum's connect_info)
    request
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Rate limiting middleware that rejects requests over the limit.
pub async fn rate_limit_middleware(
    request: Request,
    next: Next,
    limiter: Arc<RateLimiter>,
) -> Result<Response, StatusCode> {
    let client_ip = extract_client_ip(&request);
    let (max_requests, window_secs) = limiter.limits();

    if !limiter.check(&client_ip).await {
        tracing::warn!(
            "Rate limit exceeded for client: {} (limit: {} requests / {}s)",
            client_ip,
            max_requests,
            window_secs
        );
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_allows_under_limit() {
        let limiter = RateLimiter::new(5, Duration::from_secs(60));

        for i in 0..5 {
            let result = limiter.check(&format!("client-{}", i)).await;
            assert!(result, "Request {} should be allowed", i);
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60));

        // First 3 requests should succeed
        assert!(limiter.check("client").await);
        assert!(limiter.check("client").await);
        assert!(limiter.check("client").await);

        // 4th request should be blocked
        assert!(!limiter.check("client").await);
    }

    #[tokio::test]
    async fn test_rate_limiter_separate_clients() {
        let limiter = RateLimiter::new(1, Duration::from_secs(60));

        // Each client should have their own limit
        assert!(limiter.check("client-a").await);
        assert!(limiter.check("client-b").await);

        // client-a is now blocked, client-b should still work
        assert!(!limiter.check("client-a").await);
        assert!(limiter.check("client-b").await);
    }

    #[tokio::test]
    async fn test_rate_limiter_window_reset() {
        let limiter = RateLimiter::new(2, Duration::from_millis(50));

        // Exhaust limit
        assert!(limiter.check("client").await);
        assert!(limiter.check("client").await);
        assert!(!limiter.check("client").await);

        // Wait for window to expire
        tokio::time::sleep(Duration::from_millis(60)).await;

        // Should be allowed again
        assert!(limiter.check("client").await);
    }
}
