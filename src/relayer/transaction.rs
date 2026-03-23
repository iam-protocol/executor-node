use std::sync::Arc;

use crate::error::AppError;
use crate::solana::client::SolanaClient;
use crate::solana::instructions;
use crate::solana::pda;

/// VerificationResult account layout offset for `is_valid` field:
/// 8 (discriminator) + 32 (verifier) + 32 (proof_hash) + 8 (verified_at) = 80
const VERIFICATION_IS_VALID_OFFSET: usize = 8 + 32 + 32 + 8;

pub struct VerificationOutcome {
    pub signature: String,
    pub is_valid: bool,
}

pub struct RelayerTransaction {
    client: Arc<SolanaClient>,
}

impl RelayerTransaction {
    pub fn new(client: Arc<SolanaClient>) -> Self {
        Self { client }
    }

    /// Submit a verification: create_challenge + verify_proof in one transaction.
    /// Returns the transaction signature and whether the proof was valid.
    pub async fn submit_verification(
        &self,
        proof_bytes: &[u8],
        public_inputs: &[[u8; 32]],
    ) -> Result<VerificationOutcome, AppError> {
        let nonce: [u8; 32] = rand::random();

        let relayer = self.client.relayer_pubkey();
        let (challenge_pda, _) = pda::find_challenge_pda(&relayer, &nonce);
        let (verification_pda, _) = pda::find_verification_result_pda(&relayer, &nonce);

        tracing::info!(
            nonce = %bs58::encode(&nonce).into_string(),
            "Generated verification nonce"
        );

        let ix1 = instructions::build_create_challenge(&relayer, &challenge_pda, &nonce);
        let ix2 = instructions::build_verify_proof(
            &relayer,
            &challenge_pda,
            &verification_pda,
            proof_bytes,
            public_inputs,
            &nonce,
        );

        let signature = self.client.send_verification_tx(vec![ix1, ix2]).await?;

        tracing::info!(
            signature = %signature,
            "Verification transaction confirmed"
        );

        let is_valid = match self.client.get_account_data(&verification_pda).await? {
            Some(data) => {
                if data.len() <= VERIFICATION_IS_VALID_OFFSET {
                    tracing::warn!(
                        expected = VERIFICATION_IS_VALID_OFFSET + 1,
                        actual = data.len(),
                        "VerificationResult account data too short"
                    );
                    false
                } else {
                    data[VERIFICATION_IS_VALID_OFFSET] == 1
                }
            }
            None => false,
        };

        Ok(VerificationOutcome {
            signature: signature.to_string(),
            is_valid,
        })
    }
}
