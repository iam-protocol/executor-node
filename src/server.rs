use axum::extract::{Request, State};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::auth::api_key::api_key_auth;
use crate::error::AppError;
use crate::relayer::handler::{health_handler, verify_handler};
use crate::relayer::transaction::RelayerTransaction;

#[derive(Clone)]
pub struct AppState {
    pub relayer_tx: Arc<RelayerTransaction>,
    pub api_keys: Arc<Vec<String>>,
}

async fn auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    api_key_auth(request, next, &state.api_keys).await
}

pub fn create_router(state: AppState) -> Router {
    let verify_routes = Router::new()
        .route("/verify", post(verify_handler))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    Router::new()
        .route("/health", get(health_handler))
        .merge(verify_routes)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state.relayer_tx.clone())
}
