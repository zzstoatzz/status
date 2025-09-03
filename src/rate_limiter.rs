use actix_web::HttpRequest;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct RateLimiter {
    buckets: Arc<Mutex<HashMap<String, TokenBucket>>>,
    max_tokens: u32,
    refill_rate: Duration,
}

struct TokenBucket {
    tokens: u32,
    last_refill: Instant,
}

impl RateLimiter {
    pub fn new(max_tokens: u32, refill_rate: Duration) -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
            max_tokens,
            refill_rate,
        }
    }

    pub fn check_rate_limit(&self, key: &str) -> bool {
        let mut buckets = self.buckets.lock().unwrap();
        let now = Instant::now();

        let bucket = buckets.entry(key.to_string()).or_insert(TokenBucket {
            tokens: self.max_tokens,
            last_refill: now,
        });

        // Refill tokens based on elapsed time
        let elapsed = now.duration_since(bucket.last_refill);
        let tokens_to_add = (elapsed.as_secs_f64() / self.refill_rate.as_secs_f64()
            * self.max_tokens as f64) as u32;

        if tokens_to_add > 0 {
            bucket.tokens = (bucket.tokens + tokens_to_add).min(self.max_tokens);
            bucket.last_refill = now;
        }

        // Check if we have tokens available
        if bucket.tokens > 0 {
            bucket.tokens -= 1;
            true
        } else {
            false
        }
    }

    pub fn get_client_key(req: &HttpRequest) -> String {
        // Use IP address as the key for rate limiting
        req.connection_info()
            .realip_remote_addr()
            .unwrap_or("unknown")
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_rate_limiter_basic() {
        let limiter = RateLimiter::new(5, Duration::from_secs(1));

        // Should allow first 5 requests
        for _ in 0..5 {
            assert!(limiter.check_rate_limit("test_client"));
        }

        // 6th request should be blocked
        assert!(!limiter.check_rate_limit("test_client"));
    }

    #[test]
    fn test_rate_limiter_refill() {
        let limiter = RateLimiter::new(2, Duration::from_millis(100));

        // Use up tokens
        assert!(limiter.check_rate_limit("test_client"));
        assert!(limiter.check_rate_limit("test_client"));
        assert!(!limiter.check_rate_limit("test_client"));

        // Wait for refill
        thread::sleep(Duration::from_millis(150));

        // Should have tokens again
        assert!(limiter.check_rate_limit("test_client"));
    }

    #[test]
    fn test_rate_limiter_different_clients() {
        let limiter = RateLimiter::new(1, Duration::from_secs(1));

        // Different clients should have separate buckets
        assert!(limiter.check_rate_limit("client1"));
        assert!(limiter.check_rate_limit("client2"));

        // But same client should be limited
        assert!(!limiter.check_rate_limit("client1"));
        assert!(!limiter.check_rate_limit("client2"));
    }
}
