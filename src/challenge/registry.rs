use dashmap::DashMap;
use solana_sdk::pubkey::Pubkey;
use std::fmt;
use std::time::Instant;

struct NonceEntry {
    nonce: [u8; 32],
    issued_at: Instant,
}

/// Server-side challenge nonce registry. Issues nonces for wallet-connected
/// verifications and validates them at attestation time. Prevents
/// pre-computation attacks by ensuring clients use server-issued nonces
/// with a tight time window.
///
/// Keyed by wallet pubkey — one outstanding challenge per wallet at a time.
/// In-memory for devnet. Resets on restart (acceptable — worst case, the
/// client falls back to a client-generated nonce and skips attestation).
pub struct ChallengeNonceRegistry {
    entries: DashMap<Pubkey, NonceEntry>,
}

#[derive(Debug)]
pub enum ChallengeError {
    NotFound,
    NonceMismatch,
    Expired,
}

impl fmt::Display for ChallengeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "No challenge issued for this wallet"),
            Self::NonceMismatch => write!(f, "Nonce does not match issued challenge"),
            Self::Expired => write!(f, "Challenge has expired"),
        }
    }
}

impl ChallengeNonceRegistry {
    pub fn new() -> Self {
        Self {
            entries: DashMap::new(),
        }
    }

    /// Issue a new challenge nonce for the given wallet.
    /// Overwrites any existing outstanding challenge for this wallet.
    pub fn issue(&self, wallet: Pubkey) -> [u8; 32] {
        let nonce: [u8; 32] = rand::random();
        self.entries.insert(
            wallet,
            NonceEntry {
                nonce,
                issued_at: Instant::now(),
            },
        );
        nonce
    }

    /// Validate that the nonce was server-issued for this wallet and is
    /// within the time window. Consumes the entry on success (single-use).
    /// Uses atomic remove_if to avoid TOCTOU race between check and removal.
    pub fn validate_and_consume(
        &self,
        wallet: &Pubkey,
        nonce: &[u8; 32],
        max_age_secs: u64,
    ) -> Result<(), ChallengeError> {
        // Atomically remove only if nonce matches
        let removed = self
            .entries
            .remove_if(wallet, |_, entry| entry.nonce == *nonce);

        match removed {
            Some((_, entry)) => {
                if entry.issued_at.elapsed().as_secs() > max_age_secs {
                    Err(ChallengeError::Expired)
                } else {
                    Ok(())
                }
            }
            None => {
                // Either wallet not found or nonce didn't match
                if self.entries.contains_key(wallet) {
                    Err(ChallengeError::NonceMismatch)
                } else {
                    Err(ChallengeError::NotFound)
                }
            }
        }
    }

    /// Evict entries older than max_age_secs. Called by background task.
    pub fn evict_stale(&self, max_age_secs: u64) {
        self.entries
            .retain(|_, entry| entry.issued_at.elapsed().as_secs() <= max_age_secs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_wallet() -> Pubkey {
        Pubkey::new_unique()
    }

    #[test]
    fn issue_returns_nonce() {
        let registry = ChallengeNonceRegistry::new();
        let wallet = test_wallet();
        let nonce = registry.issue(wallet);
        assert_ne!(nonce, [0u8; 32]);
    }

    #[test]
    fn validate_and_consume_succeeds() {
        let registry = ChallengeNonceRegistry::new();
        let wallet = test_wallet();
        let nonce = registry.issue(wallet);
        assert!(registry.validate_and_consume(&wallet, &nonce, 60).is_ok());
    }

    #[test]
    fn validate_consumes_entry() {
        let registry = ChallengeNonceRegistry::new();
        let wallet = test_wallet();
        let nonce = registry.issue(wallet);
        registry.validate_and_consume(&wallet, &nonce, 60).unwrap();
        // Second use fails
        assert!(matches!(
            registry.validate_and_consume(&wallet, &nonce, 60),
            Err(ChallengeError::NotFound)
        ));
    }

    #[test]
    fn validate_wrong_nonce_fails() {
        let registry = ChallengeNonceRegistry::new();
        let wallet = test_wallet();
        registry.issue(wallet);
        let wrong_nonce = [42u8; 32];
        assert!(matches!(
            registry.validate_and_consume(&wallet, &wrong_nonce, 60),
            Err(ChallengeError::NonceMismatch)
        ));
    }

    #[test]
    fn validate_unknown_wallet_fails() {
        let registry = ChallengeNonceRegistry::new();
        let wallet = test_wallet();
        let nonce = [1u8; 32];
        assert!(matches!(
            registry.validate_and_consume(&wallet, &nonce, 60),
            Err(ChallengeError::NotFound)
        ));
    }

    #[test]
    fn validate_expired_fails() {
        let registry = ChallengeNonceRegistry::new();
        let wallet = test_wallet();
        let nonce = registry.issue(wallet);

        // Manually age the entry
        if let Some(mut entry) = registry.entries.get_mut(&wallet) {
            entry.issued_at = Instant::now() - std::time::Duration::from_secs(120);
        }

        assert!(matches!(
            registry.validate_and_consume(&wallet, &nonce, 60),
            Err(ChallengeError::Expired)
        ));
    }

    #[test]
    fn new_issue_overwrites_previous() {
        let registry = ChallengeNonceRegistry::new();
        let wallet = test_wallet();
        let nonce1 = registry.issue(wallet);
        let nonce2 = registry.issue(wallet);
        // Old nonce is gone
        assert!(registry.validate_and_consume(&wallet, &nonce1, 60).is_err());
        // New nonce works
        assert!(registry.validate_and_consume(&wallet, &nonce2, 60).is_ok());
    }

    #[test]
    fn evict_stale_removes_old_entries() {
        let registry = ChallengeNonceRegistry::new();
        let wallet = test_wallet();
        registry.issue(wallet);

        if let Some(mut entry) = registry.entries.get_mut(&wallet) {
            entry.issued_at = Instant::now() - std::time::Duration::from_secs(120);
        }

        registry.evict_stale(60);
        assert!(registry.entries.is_empty());
    }

    #[test]
    fn evict_stale_keeps_fresh_entries() {
        let registry = ChallengeNonceRegistry::new();
        let wallet = test_wallet();
        registry.issue(wallet);
        registry.evict_stale(60);
        assert_eq!(registry.entries.len(), 1);
    }

    #[test]
    fn different_wallets_are_independent() {
        let registry = ChallengeNonceRegistry::new();
        let wallet1 = test_wallet();
        let wallet2 = test_wallet();
        let nonce1 = registry.issue(wallet1);
        let nonce2 = registry.issue(wallet2);
        assert!(registry.validate_and_consume(&wallet1, &nonce1, 60).is_ok());
        assert!(registry.validate_and_consume(&wallet2, &nonce2, 60).is_ok());
    }
}
