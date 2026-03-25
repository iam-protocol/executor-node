mod auth;
mod config;
mod error;
mod integrator;
mod listener;
mod relayer;
mod server;
mod solana;

use std::sync::Arc;
use tracing_subscriber::EnvFilter;

use config::Config;
use integrator::tracker::IntegratorTracker;
use listener::event_monitor::EventMonitor;
use relayer::commitment_registry::CommitmentRegistry;
use relayer::transaction::RelayerTransaction;
use server::{create_router, AppState};
use solana::client::SolanaClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = Config::from_env()?;

    let solana_client = Arc::new(SolanaClient::new(&config.rpc_url, config.relayer_keypair));

    let balance = solana_client.get_balance().await?;
    tracing::info!(
        balance_sol = balance as f64 / 1_000_000_000.0,
        relayer = %solana_client.relayer_pubkey(),
        "Relayer initialized"
    );

    let relayer_tx = Arc::new(RelayerTransaction::new(solana_client));

    // Initialize per-API-key rate limiter
    let rate_limiter = Arc::new(auth::rate_limit::RateLimiter::new(
        config.rate_limit_per_minute,
    ));
    tracing::info!(
        requests_per_minute = config.rate_limit_per_minute,
        "Rate limiter initialized"
    );

    // Initialize integrator quota tracker
    let integrator_count = config.integrators.len();
    let tracker = Arc::new(IntegratorTracker::new(config.integrators));
    tracing::info!(
        integrators = integrator_count,
        "Integrator tracker initialized (in-memory, resets on restart)"
    );

    let commitment_registry = Arc::new(CommitmentRegistry::new());
    tracing::info!("Commitment registry initialized (in-memory, resets on restart)");

    // Spawn background eviction task for stale commitment entries
    let registry_ref = Arc::clone(&commitment_registry);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            registry_ref.evict_stale();
        }
    });

    let state = AppState {
        relayer_tx,
        api_keys: Arc::new(config.api_keys),
        rate_limiter,
        tracker,
        commitment_registry,
    };

    let app = create_router(state, &config.cors_origins);

    // Spawn RPC event listener in background
    let verifier_program_id = solana::pda::verifier_program_id();
    let ws_url = config.ws_url;
    tokio::spawn(async move {
        let monitor = EventMonitor::new(&ws_url, verifier_program_id);
        monitor.start().await;
    });

    let listener = tokio::net::TcpListener::bind(&config.listen_addr).await?;
    tracing::info!(addr = %config.listen_addr, "Executor node started");

    axum::serve(listener, app).await?;

    Ok(())
}
