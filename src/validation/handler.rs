use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use crate::error::AppError;
use crate::server::AppState;

#[derive(Deserialize)]
pub struct ValidateFeaturesRequest {
    pub features: Vec<f64>,
    pub wallet_id: String,
}

#[derive(Serialize)]
pub struct ValidateFeaturesResponse {
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_quota: Option<u64>,
}

pub async fn validate_features_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ValidateFeaturesRequest>,
) -> Result<Json<ValidateFeaturesResponse>, AppError> {
    let api_key = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("authenticated")
        .to_string();

    // Validate wallet_id is a real Solana pubkey
    if Pubkey::from_str(&req.wallet_id).is_err() {
        return Err(AppError::InvalidRequest(format!(
            "invalid wallet_id: {}",
            req.wallet_id
        )));
    }

    // Deduct quota before doing work
    let remaining = state.tracker.check_and_deduct(&api_key)?;

    let result = state
        .validation_service
        .validate_features(&req.features, &req.wallet_id)
        .await;

    if !result.valid {
        tracing::info!(api_key = %api_key, wallet_id = %req.wallet_id, "Feature validation rejected");
        state.metrics.increment_validations();
        return Err(AppError::ValidationFailed);
    }

    tracing::info!(api_key = %api_key, wallet_id = %req.wallet_id, "Feature validation passed");
    state.metrics.increment_validations();

    Ok(Json(ValidateFeaturesResponse {
        valid: true,
        remaining_quota: Some(remaining),
    }))
}
