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
    /// F0 contour per audio frame. Forwarded to the validation service for
    /// Tier 2 cross-modal temporal analysis. Absent for older SDK versions.
    #[serde(default)]
    pub f0_contour: Option<Vec<f64>>,
    /// Acceleration magnitude time-series, resampled to match `f0_contour` length.
    /// Paired with `f0_contour` for lagged cross-correlation.
    #[serde(default)]
    pub accel_magnitude: Option<Vec<f64>>,
    /// Base64-encoded 16-bit PCM audio samples (mono). Forwarded unchanged
    /// to the validation service for phrase content binding (master-list
    /// #89). Absent for older SDK versions.
    #[serde(default)]
    pub audio_samples_b64: Option<String>,
    /// Native sample rate of the transmitted audio. Forwarded unchanged to
    /// the validation service, which resamples to 16kHz if the browser
    /// delivered a rate other than the SDK target (common on iOS Safari
    /// with Bluetooth codec negotiation).
    #[serde(default)]
    pub audio_sample_rate_hz: Option<u32>,
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

    // Parse the wallet once. Valid wallets proceed; malformed inputs are
    // rejected before touching the rate limiter or validation service.
    let wallet = Pubkey::from_str(&req.wallet_id).map_err(|_| {
        AppError::InvalidRequest(format!("invalid wallet_id: {}", req.wallet_id))
    })?;

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

    // Look up the challenge phrase for this wallet so the validation service
    // can match transcription against it (master-list #89). If no challenge
    // was issued (old SDK path) or it has aged out, forward `None` — the
    // validation service treats missing phrase as skip, preserving backward
    // compatibility for pre-0.10.0 SDK clients.
    let expected_phrase = state
        .challenge_registry
        .peek_phrase(&wallet, state.challenge_ttl_secs);

    // Build request to internal validation service. Forward time-series and
    // audio fields unchanged — the validation service handles absence of any
    // field (old SDK versions).
    //
    // Whisper-tiny inference adds ~1s to the validation round trip. Bump the
    // client-side timeout accordingly (3s → 8s) so legitimate audio payloads
    // don't time out before transcription completes.
    let mut request = state
        .http_client
        .post(format!("{validation_url}/validate"))
        .json(&serde_json::json!({
            "features": req.features,
            "wallet_id": req.wallet_id,
            "f0_contour": req.f0_contour,
            "accel_magnitude": req.accel_magnitude,
            "audio_samples_b64": req.audio_samples_b64,
            "audio_sample_rate_hz": req.audio_sample_rate_hz,
            "expected_phrase": expected_phrase,
        }))
        .timeout(std::time::Duration::from_secs(8));

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
