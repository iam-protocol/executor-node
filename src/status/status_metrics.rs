use std::sync::atomic::{AtomicU64, Ordering};

pub struct StatusMetrics {
    total_verifications_relayed: AtomicU64,
    total_attestations_issued: AtomicU64,
    start_time: u64,
    cached_balance: AtomicU64,
    balance_fetched_at: AtomicU64,
}

impl StatusMetrics {
    pub fn new() -> Self {
        Self {
            total_verifications_relayed: AtomicU64::new(0),
            total_attestations_issued: AtomicU64::new(0),
            start_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            cached_balance: AtomicU64::new(0),
            balance_fetched_at: AtomicU64::new(0),
        }
    }

    // increase total_verifications_relayed by 1
    pub fn increment_verifications(&self) {
        self.total_verifications_relayed
            .fetch_add(1, Ordering::Relaxed);
    }

    // increase total_attestations_issued by 1
    pub fn increment_attestations(&self) {
        self.total_attestations_issued
            .fetch_add(1, Ordering::Relaxed);
    }

    // getters for the metrics
    pub fn verifications_relayed(&self) -> u64 {
        self.total_verifications_relayed.load(Ordering::Relaxed)
    }

    pub fn attestations_issued(&self) -> u64 {
        self.total_attestations_issued.load(Ordering::Relaxed)
    }

    pub fn start_time(&self) -> u64 {
        self.start_time
    }

    pub fn cached_balance(&self) -> u64 {
        self.cached_balance.load(Ordering::Relaxed)
    }

    pub fn balance_fetched_at(&self) -> u64 {
        self.balance_fetched_at.load(Ordering::Relaxed)
    }

    pub fn update_cached_balance(&self, balance: u64, fetched_at: u64) {
        self.cached_balance.store(balance, Ordering::Relaxed);
        self.balance_fetched_at.store(fetched_at, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_start_at_zero() {
        let m = StatusMetrics::new();
        assert_eq!(m.verifications_relayed(), 0);
        assert_eq!(m.attestations_issued(), 0);
    }

    #[test]
    fn increment_verifications_counts_correctly() {
        let m = StatusMetrics::new();
        m.increment_verifications();
        m.increment_verifications();
        assert_eq!(m.verifications_relayed(), 2);
        assert_eq!(m.attestations_issued(), 0);
    }

    #[test]
    fn increment_attestations_counts_correctly() {
        let m = StatusMetrics::new();
        m.increment_attestations();
        assert_eq!(m.attestations_issued(), 1);
        assert_eq!(m.verifications_relayed(), 0);
    }

    #[test]
    fn counters_are_independent() {
        let m = StatusMetrics::new();
        m.increment_verifications();
        m.increment_verifications();
        m.increment_attestations();
        assert_eq!(m.verifications_relayed(), 2);
        assert_eq!(m.attestations_issued(), 1);
    }

    #[test]
    fn start_time_is_recent() {
        let before = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let m = StatusMetrics::new();
        let after = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        assert!(m.start_time() >= before);
        assert!(m.start_time() <= after);
    }

    #[test]
    fn balance_cache_starts_empty() {
        let m = StatusMetrics::new();
        assert_eq!(m.cached_balance(), 0);
        assert_eq!(m.balance_fetched_at(), 0);
    }

    #[test]
    fn balance_cache_updates_together() {
        let m = StatusMetrics::new();
        m.update_cached_balance(123, 456);
        assert_eq!(m.cached_balance(), 123);
        assert_eq!(m.balance_fetched_at(), 456);
    }
}
