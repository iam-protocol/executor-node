use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signature, Signer};
use solana_sdk::transaction::Transaction;

use crate::error::AppError;

pub struct SolanaClient {
    rpc: RpcClient,
    relayer_keypair: Keypair,
}

impl SolanaClient {
    pub fn new(rpc_url: &str, keypair: Keypair) -> Self {
        let rpc =
            RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());
        Self {
            rpc,
            relayer_keypair: keypair,
        }
    }

    pub fn relayer_pubkey(&self) -> Pubkey {
        self.relayer_keypair.pubkey()
    }

    pub async fn get_balance(&self) -> Result<u64, AppError> {
        self.rpc
            .get_balance(&self.relayer_keypair.pubkey())
            .await
            .map_err(|e| AppError::SolanaRpc(e.to_string()))
    }

    pub async fn get_account_data(&self, pubkey: &Pubkey) -> Result<Option<Vec<u8>>, AppError> {
        match self.rpc.get_account(pubkey).await {
            Ok(account) => Ok(Some(account.data)),
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("AccountNotFound") || err_str.contains("could not find account")
                {
                    Ok(None)
                } else {
                    Err(AppError::SolanaRpc(err_str))
                }
            }
        }
    }

    /// Send a transaction with the given instructions, signed by the relayer keypair.
    /// Includes a compute budget instruction for ZK verification headroom.
    pub async fn send_verification_tx(
        &self,
        instructions: Vec<Instruction>,
    ) -> Result<Signature, AppError> {
        let mut all_instructions = vec![ComputeBudgetInstruction::set_compute_unit_limit(400_000)];
        all_instructions.extend(instructions);

        let recent_blockhash = self
            .rpc
            .get_latest_blockhash()
            .await
            .map_err(|e| AppError::SolanaRpc(e.to_string()))?;

        let tx = Transaction::new_signed_with_payer(
            &all_instructions,
            Some(&self.relayer_keypair.pubkey()),
            &[&self.relayer_keypair],
            recent_blockhash,
        );

        self.rpc
            .send_and_confirm_transaction(&tx)
            .await
            .map_err(|e| AppError::TransactionFailed(e.to_string()))
    }
}
