//! HTTP Middleware
//!
//! S8-D4: Rate limiting middleware
//! S8-D5: Health checks

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use redis::AsyncCommands;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimiter {
    redis: redis::aio::MultiplexedConnection,
    max_requests: u32,
    window_secs: u64,
}

/// Rate limit check result
#[derive(Debug, Clone)]
pub enum RateLimitResult {
    Allowed { remaining: u32 },
    Exceeded { retry_after: u64 },
}

impl RateLimiter {
    /// Create a new rate limiter
    pub async fn new(
        redis: redis::aio::MultiplexedConnection,
        max_requests: u32,
        window_secs: u64,
    ) -> Self {
        Self {
            redis,
            max_requests,
            window_secs,
        }
    }

    /// Check rate limit for a key
    pub async fn check(&mut self, key: &str) -> RateLimitResult {
        let window_key = format!("rate_limit:{}", key);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Get current count
        let count: u32 = self.redis.get(&window_key).await.unwrap_or(0);
        
        if count >= self.max_requests {
            // Get TTL
            let ttl: i64 = self.redis.ttl(&window_key).await.unwrap_or(0);
            RateLimitResult::Exceeded { retry_after: ttl.max(0) as u64 }
        } else {
            // Increment counter
            let _: () = self.redis.incr(&window_key, 1).await.unwrap_or(());
            
            // Set expiry if first request
            if count == 0 {
                let _: () = self.redis.expire(&window_key, self.window_secs as i64).await.unwrap_or(());
            }
            
            RateLimitResult::Allowed { remaining: self.max_requests - count - 1 }
        }
    }
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(limiter): State<Arc<tokio::sync::Mutex<RateLimiter>>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract client IP or API key
    let client_key = extract_client_key(&request);
    
    let mut limiter = limiter.lock().await;
    
    match limiter.check(&client_key).await {
        RateLimitResult::Allowed { remaining } => {
            let mut response = next.run(request).await;
            
            // Add rate limit headers
            response.headers_mut().insert(
                "X-RateLimit-Limit",
                limiter.max_requests.to_string().parse().unwrap(),
            );
            response.headers_mut().insert(
                "X-RateLimit-Remaining",
                remaining.to_string().parse().unwrap(),
            );
            
            Ok(response)
        }
        RateLimitResult::Exceeded { retry_after } => {
            warn!("Rate limit exceeded for client: {}", client_key);
            Err(StatusCode::TOO_MANY_REQUESTS)
        }
    }
}

/// Extract client identifier from request
fn extract_client_key(request: &Request) -> String {
    // Try X-API-Key header first
    if let Some(api_key) = request.headers().get("X-API-Key") {
        if let Ok(key) = api_key.to_str() {
            return format!("api:{}", key);
        }
    }
    
    // Fall back to IP address
    if let Some(forwarded) = request.headers().get("X-Forwarded-For") {
        if let Ok(ip) = forwarded.to_str() {
            return format!("ip:{}", ip.split(',').next().unwrap_or("unknown").trim());
        }
    }
    
    // Last resort
    "ip:unknown".to_string()
}

/// Request logging middleware
pub async fn logging_middleware(
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = std::time::Instant::now();
    
    let response = next.run(request).await;
    
    let duration = start.elapsed();
    let status = response.status();
    
    debug!(
        "{} {} - {} - {:?}",
        method,
        uri,
        status.as_u16(),
        duration
    );
    
    response
}
