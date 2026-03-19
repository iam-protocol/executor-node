use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

use crate::error::AppError;

/// Validate the X-API-Key header against the configured API keys.
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

    if !api_keys.iter().any(|k| k == key) {
        return Err(AppError::Unauthorized);
    }

    Ok(next.run(request).await)
}
