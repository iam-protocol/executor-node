#![allow(dead_code)]

use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

fn verifier_program_id() -> Pubkey {
    Pubkey::from_str("4F97jNoxQzT2qRbkWpW3ztC3Nz2TtKj3rnKG8ExgnrfV").expect("valid pubkey")
}

fn anchor_program_id() -> Pubkey {
    Pubkey::from_str("GZYwTp2ozeuRA5Gof9vs4ya961aANcJBdUzB7LN6q4b2").expect("valid pubkey")
}

fn registry_program_id() -> Pubkey {
    Pubkey::from_str("6VBs3zr9KrfFPGd6j7aGBPQWwZa5tajVfA7HN6MMV9VW").expect("valid pubkey")
}

pub fn find_challenge_pda(challenger: &Pubkey, nonce: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"challenge", challenger.as_ref(), nonce.as_ref()],
        &verifier_program_id(),
    )
}

pub fn find_verification_result_pda(verifier: &Pubkey, nonce: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"verification", verifier.as_ref(), nonce.as_ref()],
        &verifier_program_id(),
    )
}

pub fn find_identity_state_pda(user: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"identity", user.as_ref()], &anchor_program_id())
}

pub fn find_protocol_config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"protocol_config"], &registry_program_id())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pda_derivation_is_deterministic() {
        let pubkey = Pubkey::new_unique();
        let nonce = [42u8; 32];
        let (pda1, bump1) = find_challenge_pda(&pubkey, &nonce);
        let (pda2, bump2) = find_challenge_pda(&pubkey, &nonce);
        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);
    }

    #[test]
    fn different_nonces_produce_different_pdas() {
        let pubkey = Pubkey::new_unique();
        let nonce1 = [1u8; 32];
        let nonce2 = [2u8; 32];
        let (pda1, _) = find_challenge_pda(&pubkey, &nonce1);
        let (pda2, _) = find_challenge_pda(&pubkey, &nonce2);
        assert_ne!(pda1, pda2);
    }
}
