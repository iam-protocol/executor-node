use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use crate::error::AppError;
use crate::server::AppState;

#[derive(Deserialize)]
pub struct AttestRequest {
    pub wallet_address: String,
}

#[derive(Serialize)]
pub struct AttestResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attestation_tx: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub async fn attest_handler(
    State(state): State<AppState>,
    Json(req): Json<AttestRequest>,
) -> Result<Json<AttestResponse>, AppError> {
    // 1. Check SAS is configured
    let attestor = state.sas_attestor.as_ref().ok_or_else(|| {
        AppError::InvalidRequest("SAS attestation is not configured".into())
    })?;

    // 2. Parse wallet address
    let user_wallet = Pubkey::from_str(&req.wallet_address).map_err(|_| {
        AppError::InvalidRequest(format!("Invalid wallet address: {}", req.wallet_address))
    })?;

    // 3. Issue attestation
    match attestor.issue_attestation(&user_wallet).await {
        Ok(sig) => {
            tracing::info!(
                wallet = %user_wallet,
                attestation_sig = %sig,
                "SAS attestation issued"
            );

            state.metrics.increment_attestations();

            Ok(Json(AttestResponse {
                success: true,
                attestation_tx: Some(sig),
                error: None,
            }))
        }
        Err(e) => {
            tracing::error!(
                wallet = %user_wallet,
                error = %e,
                "SAS attestation failed"
            );
            Ok(Json(AttestResponse {
                success: false,
                attestation_tx: None,
                error: Some(e.to_string()),
            }))
        }
    }
}
