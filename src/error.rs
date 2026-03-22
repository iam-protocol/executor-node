use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Rate limited")]
    RateLimited,

    #[error("Insufficient quota")]
    InsufficientQuota,

    #[error("Solana RPC error: {0}")]
    SolanaRpc(String),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".into()),
            AppError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "Rate limited".into()),
            AppError::InsufficientQuota => {
                (StatusCode::PAYMENT_REQUIRED, "Insufficient verification quota".into())
            }
            AppError::SolanaRpc(msg) => (StatusCode::BAD_GATEWAY, msg.clone()),
            AppError::TransactionFailed(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, msg.clone())
            }
        };

        let body = json!({ "error": message });
        (status, axum::Json(body)).into_response()
    }
}
