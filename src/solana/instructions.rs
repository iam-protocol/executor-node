use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

const SYSTEM_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("11111111111111111111111111111111");

fn verifier_program_id() -> Pubkey {
    Pubkey::from_str("4F97jNoxQzT2qRbkWpW3ztC3Nz2TtKj3rnKG8ExgnrfV").expect("valid pubkey")
}

// Anchor discriminators (from IDL)
const CREATE_CHALLENGE_DISC: [u8; 8] = [170, 244, 47, 1, 1, 15, 173, 239];
const VERIFY_PROOF_DISC: [u8; 8] = [217, 211, 191, 110, 144, 13, 186, 98];

/// Build create_challenge instruction.
/// Data: 8-byte discriminator + 32-byte nonce
pub fn build_create_challenge(
    challenger: &Pubkey,
    challenge_pda: &Pubkey,
    nonce: &[u8; 32],
) -> Instruction {
    let mut data = Vec::with_capacity(40);
    data.extend_from_slice(&CREATE_CHALLENGE_DISC);
    data.extend_from_slice(nonce);

    Instruction {
        program_id: verifier_program_id(),
        accounts: vec![
            AccountMeta::new(*challenger, true),
            AccountMeta::new(*challenge_pda, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

/// Build verify_proof instruction.
/// Data: 8-byte discriminator + borsh Vec<u8> proof + borsh Vec<[u8;32]> inputs + 32-byte nonce
pub fn build_verify_proof(
    verifier: &Pubkey,
    challenge_pda: &Pubkey,
    verification_pda: &Pubkey,
    proof_bytes: &[u8],
    public_inputs: &[[u8; 32]],
    nonce: &[u8; 32],
) -> Instruction {
    let mut data = Vec::new();
    data.extend_from_slice(&VERIFY_PROOF_DISC);

    // Borsh-serialize Vec<u8>: 4-byte little-endian length + raw bytes
    let proof_len = proof_bytes.len() as u32;
    data.extend_from_slice(&proof_len.to_le_bytes());
    data.extend_from_slice(proof_bytes);

    // Borsh-serialize Vec<[u8; 32]>: 4-byte little-endian count + count * 32 bytes
    let inputs_count = public_inputs.len() as u32;
    data.extend_from_slice(&inputs_count.to_le_bytes());
    for input in public_inputs {
        data.extend_from_slice(input);
    }

    // Raw 32-byte nonce (fixed-size array, no length prefix)
    data.extend_from_slice(nonce);

    Instruction {
        program_id: verifier_program_id(),
        accounts: vec![
            AccountMeta::new(*verifier, true),
            AccountMeta::new(*challenge_pda, false),
            AccountMeta::new(*verification_pda, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_challenge_data_is_40_bytes() {
        let pubkey = Pubkey::new_unique();
        let nonce = [0u8; 32];
        let ix = build_create_challenge(&pubkey, &pubkey, &nonce);
        assert_eq!(ix.data.len(), 40); // 8 disc + 32 nonce
    }

    #[test]
    fn verify_proof_data_has_correct_structure() {
        let pubkey = Pubkey::new_unique();
        let proof = vec![0u8; 256];
        let inputs = [[0u8; 32]; 3];
        let nonce = [0u8; 32];
        let ix = build_verify_proof(&pubkey, &pubkey, &pubkey, &proof, &inputs, &nonce);
        // 8 disc + (4 + 256) proof + (4 + 96) inputs + 32 nonce = 400
        assert_eq!(ix.data.len(), 400);
    }

    #[test]
    fn discriminator_starts_correctly() {
        let pubkey = Pubkey::new_unique();
        let nonce = [0u8; 32];
        let ix = build_create_challenge(&pubkey, &pubkey, &nonce);
        assert_eq!(&ix.data[0..8], &CREATE_CHALLENGE_DISC);
    }
}
