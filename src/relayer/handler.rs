use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::server::AppState;

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub proof_bytes: Vec<u8>,
    pub public_inputs: Vec<Vec<u8>>,
    pub commitment: Vec<u8>,
}

#[derive(Serialize)]
pub struct VerifyResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_quota: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub async fn verify_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, AppError> {
    let api_key = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("authenticated")
        .to_string();

    if req.proof_bytes.len() != 256 {
        return Err(AppError::InvalidRequest(format!(
            "proof_bytes must be 256 bytes, got {}",
            req.proof_bytes.len()
        )));
    }

    if req.public_inputs.len() != 4 {
        return Err(AppError::InvalidRequest(format!(
            "public_inputs must have 4 elements, got {}",
            req.public_inputs.len()
        )));
    }

    let mut inputs: [[u8; 32]; 4] = [[0u8; 32]; 4];
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

    let remaining = state.tracker.check_and_deduct(&api_key)?;

    tracing::info!(api_key = %api_key, "Submitting verification");

    let outcome = match state.relayer_tx.submit_verification(&req.proof_bytes, &inputs).await {
        Ok(outcome) => outcome,
        Err(e) => {
            state.tracker.refund(&api_key);
            tracing::error!(api_key = %api_key, error = %e, "Verification submission failed");
            return Err(e);
        }
    };

    tracing::info!(
        api_key = %api_key,
        signature = %outcome.signature,
        verified = outcome.is_valid,
        remaining_quota = remaining,
        "Verification completed"
    );

    Ok(Json(VerifyResponse {
        success: true,
        tx_signature: Some(outcome.signature),
        verified: Some(outcome.is_valid),
        remaining_quota: Some(remaining),
        error: None,
    }))
}

pub async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}
