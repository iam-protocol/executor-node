use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use crate::error::AppError;
use crate::server::AppState;

#[derive(Deserialize)]
pub struct ChallengeRequest {
    pub wallet: String,
}

#[derive(Serialize)]
pub struct ChallengeResponse {
    pub nonce: Vec<u8>,
    pub expires_in: u64,
}

pub async fn challenge_handler(
    State(state): State<AppState>,
    Query(req): Query<ChallengeRequest>,
) -> Result<Json<ChallengeResponse>, AppError> {
    let wallet = Pubkey::from_str(&req.wallet).map_err(|_| {
        AppError::InvalidRequest(format!("Invalid wallet address: {}", req.wallet))
    })?;

    let nonce = state.challenge_registry.issue(wallet);

    tracing::debug!(wallet = %wallet, "Challenge nonce issued");

    Ok(Json(ChallengeResponse {
        nonce: nonce.to_vec(),
        expires_in: state.challenge_ttl_secs,
    }))
}
