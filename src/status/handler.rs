use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde::Serialize;
use subtle::ConstantTimeEq;

use crate::error::AppError;
use crate::server::AppState;

const BALANCE_CACHE_TTL_SECONDS: u64 = 30;

#[derive(Serialize)]
pub struct StatusResponse {
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verifications_relayed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attestations_issued: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validations_performed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relayer_balance_lamports: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sas_configured: Option<bool>,
}

pub async fn status_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<StatusResponse>, AppError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let is_authenticated = headers
        .get("X-API-Key")
        .and_then(|value| value.to_str().ok())
        .map(|key| {
            let key_bytes = key.as_bytes();
            state.api_keys.iter().any(|candidate| {
                candidate.len() == key_bytes.len() && candidate.as_bytes().ct_eq(key_bytes).into()
            })
        })
        .unwrap_or(false);

    if !is_authenticated {
        return Ok(Json(StatusResponse {
            status: "ok",
            uptime_seconds: None,
            verifications_relayed: None,
            attestations_issued: None,
            validations_performed: None,
            relayer_balance_lamports: None,
            sas_configured: None,
        }));
    }

    let (cached_balance, fetched_at) = state.metrics.cached_balance();
    let balance = if now.saturating_sub(fetched_at) < BALANCE_CACHE_TTL_SECONDS {
        cached_balance
    } else {
        let balance = state.relayer_tx.get_balance().await?;
        state.metrics.update_cached_balance(balance, now);
        balance
    };

    Ok(Json(StatusResponse {
        status: "ok",
        uptime_seconds: Some(now.saturating_sub(state.metrics.start_time())),
        verifications_relayed: Some(state.metrics.verifications_relayed()),
        attestations_issued: Some(state.metrics.attestations_issued()),
        validations_performed: Some(state.metrics.validations_performed()),
        relayer_balance_lamports: Some(balance),
        sas_configured: Some(state.sas_attestor.is_some()),
    }))
}
