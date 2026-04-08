use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::error::AppError;
use crate::server::AppState;

#[derive(Serialize)]
pub struct StatusResponse {
    pub uptime_seconds: u64,
    pub verifications_relayed: u64,
    pub attestations_issued: u64,
    pub relayer_balance_lamports: u64,
    pub sas_configured: bool,
}

pub async fn status_handler(
    State(state): State<AppState>,
) -> Result<Json<StatusResponse>, AppError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    Ok(Json(StatusResponse {
        uptime_seconds: now.saturating_sub(state.metrics.start_time()),
        verifications_relayed: state.metrics.verifications_relayed(),
        attestations_issued: state.metrics.attestations_issued(),
        relayer_balance_lamports: state.relayer_tx.get_balance().await?,
        sas_configured: state.sas_attestor.is_some(),
    }))
}
