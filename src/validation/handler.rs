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

    if Pubkey::from_str(&req.wallet_id).is_err() {
        return Err(AppError::InvalidRequest(format!(
            "invalid wallet_id: {}",
            req.wallet_id
        )));
    }

    let remaining = state.tracker.check_and_deduct(&api_key)?;

    // If validation service is not configured, pass through
    let validation_url = match &state.validation_url {
        Some(url) => url,
        None => {
            tracing::debug!("Validation service not configured, skipping");
            state.metrics.increment_validations();
            return Ok(Json(ValidateFeaturesResponse {
                valid: true,
                remaining_quota: Some(remaining),
            }));
        }
    };

    // Build request to internal validation service
    let mut request = state
        .http_client
        .post(format!("{validation_url}/validate"))
        .json(&serde_json::json!({
            "features": req.features,
            "wallet_id": req.wallet_id,
        }))
        .timeout(std::time::Duration::from_secs(3));

    // Add bearer token if configured
    if let Some(key) = &state.validation_api_key {
        request = request.bearer_auth(key);
    }

    let response = match request.send().await {
        Ok(r) => r,
        Err(e) => {
            // Infrastructure failure — refund quota
            state.tracker.refund(&api_key);
            return Err(AppError::ValidationServiceError(e.to_string()));
        }
    };

    state.metrics.increment_validations();

    if !response.status().is_success() {
        tracing::info!(api_key = %api_key, wallet_id = %req.wallet_id, "Feature validation rejected");
        return Err(AppError::ValidationFailed);
    }

    tracing::info!(api_key = %api_key, wallet_id = %req.wallet_id, "Feature validation passed");

    Ok(Json(ValidateFeaturesResponse {
        valid: true,
        remaining_quota: Some(remaining),
    }))
}
