use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::error::AppError;
use crate::relayer::transaction::RelayerTransaction;

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct VerifyRequest {
    pub proof_bytes: Vec<u8>,
    pub public_inputs: Vec<Vec<u8>>,
    pub commitment: Vec<u8>,
    pub is_first_verification: bool,
}

#[derive(Serialize)]
pub struct VerifyResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub async fn verify_handler(
    State(relayer_tx): State<Arc<RelayerTransaction>>,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, AppError> {
    // Validate input
    if req.proof_bytes.len() != 256 {
        return Err(AppError::InvalidRequest(format!(
            "proof_bytes must be 256 bytes, got {}",
            req.proof_bytes.len()
        )));
    }

    if req.public_inputs.len() != 3 {
        return Err(AppError::InvalidRequest(format!(
            "public_inputs must have 3 elements, got {}",
            req.public_inputs.len()
        )));
    }

    // Convert public inputs to fixed-size arrays
    let mut inputs: [[u8; 32]; 3] = [[0u8; 32]; 3];
    for (i, pi) in req.public_inputs.iter().enumerate() {
        if pi.len() != 32 {
            return Err(AppError::InvalidRequest(format!(
                "public_inputs[{}] must be 32 bytes, got {}",
                i,
                pi.len()
            )));
        }
        inputs[i].copy_from_slice(pi);
    }

    if req.commitment.len() != 32 {
        return Err(AppError::InvalidRequest(format!(
            "commitment must be 32 bytes, got {}",
            req.commitment.len()
        )));
    }

    // Submit verification
    let outcome = relayer_tx
        .submit_verification(&req.proof_bytes, &inputs)
        .await?;

    Ok(Json(VerifyResponse {
        success: true,
        tx_signature: Some(outcome.signature),
        verified: Some(outcome.is_valid),
        error: None,
    }))
}

pub async fn health_handler(
    State(_relayer_tx): State<Arc<RelayerTransaction>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
    }))
}
