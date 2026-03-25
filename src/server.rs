use axum::extract::{DefaultBodyLimit, Request, State};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::auth::api_key::api_key_auth;
use crate::auth::rate_limit::RateLimiter;
use crate::error::AppError;
use crate::integrator::tracker::IntegratorTracker;
use crate::relayer::commitment_registry::CommitmentRegistry;
use crate::relayer::handler::{health_handler, verify_handler};
use crate::relayer::transaction::RelayerTransaction;

#[derive(Clone)]
pub struct AppState {
    pub relayer_tx: Arc<RelayerTransaction>,
    pub api_keys: Arc<Vec<String>>,
    pub rate_limiter: Arc<RateLimiter>,
    pub tracker: Arc<IntegratorTracker>,
    pub commitment_registry: Arc<CommitmentRegistry>,
}

async fn auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    api_key_auth(request, next, &state.api_keys).await
}

async fn rate_limit_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let key = request
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("authenticated");

    if state.rate_limiter.check(key).is_err() {
        tracing::warn!(api_key = key, "Rate limit exceeded");
        return Err(AppError::RateLimited);
    }

    Ok(next.run(request).await)
}

pub fn create_router(state: AppState, cors_origins: &[String]) -> Router {
    // Middleware order: auth runs first (outermost layer applied last),
    // then rate limiting runs against validated keys only.
    let verify_routes = Router::new()
        .route("/verify", post(verify_handler))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit_middleware,
        ))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    let cors = if cors_origins.is_empty() {
        // No origins configured — permissive for development
        CorsLayer::permissive()
    } else {
        CorsLayer::new()
            .allow_origin(
                cors_origins
                    .iter()
                    .filter_map(|o| o.parse().ok())
                    .collect::<Vec<axum::http::HeaderValue>>(),
            )
            .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::OPTIONS])
            .allow_headers([axum::http::header::CONTENT_TYPE, axum::http::header::HeaderName::from_static("x-api-key")])
    };

    Router::new()
        .route("/health", get(health_handler))
        .merge(verify_routes)
        .layer(DefaultBodyLimit::max(4096))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
