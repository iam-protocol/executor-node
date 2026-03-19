mod auth;
mod config;
mod error;
mod relayer;
mod server;
mod solana;

use std::sync::Arc;
use tracing_subscriber::EnvFilter;

use config::Config;
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

    let state = AppState {
        relayer_tx,
        api_keys: Arc::new(config.api_keys),
    };

    let app = create_router(state);

    let listener = tokio::net::TcpListener::bind(&config.listen_addr).await?;
    tracing::info!(addr = %config.listen_addr, "Executor node started");

    axum::serve(listener, app).await?;

    Ok(())
}
