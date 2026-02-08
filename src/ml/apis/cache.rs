//! ML API Response Cache
//!
//! Caches LLM responses to reduce API costs and latency
//! Sprint 10: Response caching (Redis)

use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use chrono::{DateTime, Utc, Duration};

/// Cache errors
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("Redis error: {0}")]
    RedisError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("No connection")]
    NoConnection,
}

impl From<redis::RedisError> for CacheError {
    fn from(e: redis::RedisError) -> Self {
        CacheError::RedisError(e.to_string())
    }
}

impl From<serde_json::Error> for CacheError {
    fn from(e: serde_json::Error) -> Self {
        CacheError::SerializationError(e.to_string())
    }
}

/// Cache for LLM responses
pub struct ResponseCache {
    redis: Option<redis::aio::MultiplexedConnection>,
    default_ttl: u64, // seconds
    hit_count: std::sync::atomic::AtomicU64,
    miss_count: std::sync::atomic::AtomicU64,
}

/// Cached response entry
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    response: String,
    provider: String,
    cached_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    prompt_hash: String,
}

/// Cache key generation
fn generate_cache_key(provider: &str, prompt: &str) -> String {
    let mut hasher = DefaultHasher::new();
    provider.hash(&mut hasher);
    prompt.hash(&mut hasher);
    let hash = hasher.finish();
    format!("llm:cache:{:x}", hash)
}

/// Sanitize prompt for cache key (remove timestamps, etc.)
fn sanitize_prompt(prompt: &str) -> String {
    // Remove common non-deterministic elements
    prompt
        .lines()
        .filter(|line| !line.contains("timestamp"))
        .filter(|line| !line.contains("current_time"))
        .collect::<Vec<_>>()
        .join("\n")
}

impl ResponseCache {
    /// Create new cache with Redis backend
    pub async fn new(redis_url: &str, default_ttl_seconds: u64) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(redis_url)?;
        let redis = match client.get_multiplexed_async_connection().await {
            Ok(conn) => Some(conn),
            Err(e) => {
                tracing::warn!("Failed to connect to Redis cache: {}", e);
                None
            }
        };
        
        Ok(Self {
            redis,
            default_ttl: default_ttl_seconds,
            hit_count: std::sync::atomic::AtomicU64::new(0),
            miss_count: std::sync::atomic::AtomicU64::new(0),
        })
    }
    
    /// Create in-memory only cache (no Redis)
    pub fn new_memory_only(default_ttl_seconds: u64) -> Self {
        Self {
            redis: None,
            default_ttl: default_ttl_seconds,
            hit_count: std::sync::atomic::AtomicU64::new(0),
            miss_count: std::sync::atomic::AtomicU64::new(0),
        }
    }
    
    /// Get cached response
    pub async fn get(&self, provider: &str, prompt: &str) -> Option<String> {
        if self.redis.is_none() {
            return None;
        }
        
        let key = generate_cache_key(provider, &sanitize_prompt(prompt));
        
        let mut conn = self.redis.as_ref()?.clone();
        let result: Option<String> = conn.get(&key).await.ok()?;
        
        if let Some(json_str) = result {
            if let Ok(entry) = serde_json::from_str::<CacheEntry>(&json_str) {
                // Check if expired
                if Utc::now() < entry.expires_at {
                    self.hit_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    tracing::debug!("Cache hit for provider: {}", provider);
                    return Some(entry.response);
                }
            }
        }
        
        self.miss_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        None
    }
    
    /// Store response in cache
    pub async fn set(&self, provider: &str, prompt: &str, response: &str) -> Result<(), CacheError> {
        if self.redis.is_none() {
            return Ok(());
        }
        
        let key = generate_cache_key(provider, &sanitize_prompt(prompt));
        let now = Utc::now();
        
        let entry = CacheEntry {
            response: response.to_string(),
            provider: provider.to_string(),
            cached_at: now,
            expires_at: now + Duration::seconds(self.default_ttl as i64),
            prompt_hash: key.clone(),
        };
        
        let json = serde_json::to_string(&entry)?;
        
        let mut conn = self.redis.as_ref().ok_or(CacheError::NoConnection)?.clone();
        conn.set_ex::<_, _, ()>(&key, json, self.default_ttl).await.map_err(|e| CacheError::RedisError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Invalidate cache entry
    pub async fn invalidate(&self, provider: &str, prompt: &str) -> Result<(), CacheError> {
        if self.redis.is_none() {
            return Ok(());
        }
        
        let key = generate_cache_key(provider, &sanitize_prompt(prompt));
        
        let mut conn = self.redis.as_ref().ok_or(CacheError::NoConnection)?.clone();
        conn.del::<_, ()>(&key).await.map_err(|e| CacheError::RedisError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Clear all cached responses
    pub async fn clear_all(&self) -> Result<(), CacheError> {
        if self.redis.is_none() {
            return Ok(());
        }
        
        let mut conn = self.redis.as_ref().ok_or(CacheError::NoConnection)?.clone();
        let keys: Vec<String> = conn.keys::<_, Vec<String>>("llm:cache:*").await.map_err(|e| CacheError::RedisError(e.to_string()))?;
        
        if !keys.is_empty() {
            conn.del::<_, ()>(&keys).await.map_err(|e| CacheError::RedisError(e.to_string()))?;
        }
        
        Ok(())
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let hits = self.hit_count.load(std::sync::atomic::Ordering::Relaxed);
        let misses = self.miss_count.load(std::sync::atomic::Ordering::Relaxed);
        let total = hits + misses;
        
        CacheStats {
            hits,
            misses,
            hit_rate: if total > 0 { (hits as f64 / total as f64) * 100.0 } else { 0.0 },
            total_requests: total,
        }
    }
    
    /// Check if caching is available
    pub fn is_available(&self) -> bool {
        self.redis.is_some()
    }
    
    /// Get TTL
    pub fn ttl(&self) -> u64 {
        self.default_ttl
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub total_requests: u64,
}

/// Cache wrapper for API clients
pub struct CachedClient<T> {
    inner: T,
    cache: ResponseCache,
    provider_name: String,
    /// Whether to cache this provider's responses
    should_cache: bool,
}

impl<T> CachedClient<T> {
    pub fn new(inner: T, cache: ResponseCache, provider_name: impl Into<String>) -> Self {
        Self {
            inner,
            cache,
            provider_name: provider_name.into(),
            should_cache: true,
        }
    }
    
    /// Disable caching for this client
    pub fn disable_cache(&mut self) {
        self.should_cache = false;
    }
    
    /// Check cache before making request
    pub async fn get_cached(&self, prompt: &str) -> Option<String> {
        if !self.should_cache {
            return None;
        }
        
        self.cache.get(&self.provider_name, prompt).await
    }
    
    /// Store response in cache
    pub async fn store_cached(&self, prompt: &str, response: &str) -> Result<(), CacheError> {
        if !self.should_cache {
            return Ok(());
        }
        
        self.cache.set(&self.provider_name, prompt, response).await
    }
    
    pub fn inner(&self) -> &T {
        &self.inner
    }
    
    pub fn cache(&self) -> &ResponseCache {
        &self.cache
    }
}

/// Determine if a prompt should be cached
pub fn should_cache_prompt(prompt: &str) -> bool {
    // Don't cache prompts with time-sensitive data
    if prompt.contains("current price") || prompt.contains("latest") {
        return false;
    }
    
    // Don't cache very short prompts (likely test/health checks)
    if prompt.len() < 50 {
        return false;
    }
    
    // Cache SEC filings analysis (stable content)
    if prompt.contains("SEC filing") || prompt.contains("10-K") || prompt.contains("10-Q") {
        return true;
    }
    
    // Cache earnings analysis
    if prompt.contains("earnings") || prompt.contains("transcript") {
        return true;
    }
    
    // Default: cache if it looks like analysis
    prompt.len() > 200
}

/// Different TTLs for different types of content
pub fn get_ttl_for_content(prompt: &str) -> u64 {
    if prompt.contains("SEC filing") {
        7 * 24 * 3600 // 7 days - SEC filings don't change
    } else if prompt.contains("earnings") {
        24 * 3600 // 1 day - earnings are quarterly
    } else if prompt.contains("sentiment") {
        3600 // 1 hour - sentiment can change
    } else {
        6 * 3600 // 6 hours default
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cache_key_generation() {
        let key1 = generate_cache_key("gemini", "Analyze AAPL");
        let key2 = generate_cache_key("gemini", "Analyze AAPL");
        let key3 = generate_cache_key("openai", "Analyze AAPL");
        
        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }
    
    #[test]
    fn test_should_cache() {
        assert!(!should_cache_prompt("test"));
        assert!(!should_cache_prompt("current price of AAPL?"));
        assert!(should_cache_prompt("Analyze this SEC filing for AAPL..."));
    }
    
    #[test]
    fn test_ttl_for_content() {
        assert_eq!(get_ttl_for_content("SEC filing 10-K"), 7 * 24 * 3600);
        assert_eq!(get_ttl_for_content("earnings call"), 24 * 3600);
        assert_eq!(get_ttl_for_content("generic prompt"), 6 * 3600);
    }
}
