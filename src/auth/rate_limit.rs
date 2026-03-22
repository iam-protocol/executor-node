use std::num::NonZeroU32;
use std::sync::Arc;

use dashmap::DashMap;
use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter as GovernorLimiter};

type Limiter = GovernorLimiter<NotKeyed, InMemoryState, DefaultClock>;

const MAX_TRACKED_KEYS: usize = 10_000;

/// Per-API-key rate limiter using the GCRA algorithm.
/// Bounded to MAX_TRACKED_KEYS entries to prevent memory exhaustion.
pub struct RateLimiter {
    limiters: DashMap<String, Arc<Limiter>>,
    quota: Quota,
}

impl RateLimiter {
    pub fn new(requests_per_minute: u32) -> Self {
        let per_minute = NonZeroU32::new(requests_per_minute.max(1))
            .unwrap_or(NonZeroU32::new(1).expect("literal 1"));
        let quota = Quota::per_minute(per_minute);
        Self {
            limiters: DashMap::new(),
            quota,
        }
    }

    /// Check if a request from the given API key is allowed.
    /// Returns Ok(()) if allowed, Err(()) if rate limited.
    /// Rejects new keys if the map exceeds MAX_TRACKED_KEYS to prevent OOM.
    pub fn check(&self, api_key: &str) -> Result<(), ()> {
        if !self.limiters.contains_key(api_key) && self.limiters.len() >= MAX_TRACKED_KEYS {
            return Err(());
        }

        let limiter = self
            .limiters
            .entry(api_key.to_string())
            .or_insert_with(|| Arc::new(GovernorLimiter::direct(self.quota)))
            .clone();

        limiter.check().map_err(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_requests_within_limit() {
        let limiter = RateLimiter::new(60);
        assert!(limiter.check("test_key").is_ok());
    }

    #[test]
    fn separate_keys_have_separate_limits() {
        let limiter = RateLimiter::new(1);
        assert!(limiter.check("key_a").is_ok());
        assert!(limiter.check("key_b").is_ok());
    }
}
