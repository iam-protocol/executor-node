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
    pub uptime_seconds: u64,
    pub verifications_relayed: u64,
    pub attestations_issued: u64,
    pub relayer_balance_lamports: Option<u64>,
    pub sas_configured: bool,
}

pub async fn status_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<StatusResponse>, AppError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let relayer_balance_lamports = match headers
        .get("X-API-Key")
        .and_then(|value| value.to_str().ok())
    {
        Some(key) => {
            let key_bytes = key.as_bytes();
            let is_valid = state.api_keys.iter().any(|candidate| {
                candidate.len() == key_bytes.len() && candidate.as_bytes().ct_eq(key_bytes).into()
            });

            if is_valid {
                let balance_fetched_at = state.metrics.balance_fetched_at();
                let cached_balance = state.metrics.cached_balance();

                if now.saturating_sub(balance_fetched_at) < BALANCE_CACHE_TTL_SECONDS {
                    Some(cached_balance)
                } else {
                    let balance = state.relayer_tx.get_balance().await?;
                    state.metrics.update_cached_balance(balance, now);
                    Some(balance)
                }
            } else {
                None
            }
        }
        _ => None,
    };

    Ok(Json(StatusResponse {
        uptime_seconds: now.saturating_sub(state.metrics.start_time()),
        verifications_relayed: state.metrics.verifications_relayed(),
        attestations_issued: state.metrics.attestations_issued(),
        relayer_balance_lamports,
        sas_configured: state.sas_attestor.is_some(),
    }))
}
