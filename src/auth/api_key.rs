use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use subtle::ConstantTimeEq;

use crate::error::AppError;

/// Validate the X-API-Key header against the configured API keys.
/// Uses constant-time comparison to prevent timing side-channel attacks.
pub async fn api_key_auth(
    request: Request,
    next: Next,
    api_keys: &[String],
) -> Result<Response, AppError> {
    let key = request
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    let key_bytes = key.as_bytes();
    let is_valid = api_keys
        .iter()
        .any(|k| k.len() == key_bytes.len() && k.as_bytes().ct_eq(key_bytes).into());

    if !is_valid {
        return Err(AppError::Unauthorized);
    }

    Ok(next.run(request).await)
}
