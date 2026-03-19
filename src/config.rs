use solana_sdk::signature::{read_keypair_file, Keypair};
use std::net::SocketAddr;

#[allow(dead_code)]
pub struct Config {
    pub rpc_url: String,
    pub relayer_keypair: Keypair,
    pub listen_addr: SocketAddr,
    pub api_keys: Vec<String>,
    pub rate_limit_per_minute: u32,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let rpc_url =
            std::env::var("RPC_URL").unwrap_or_else(|_| "https://api.devnet.solana.com".into());

        let keypair_path = std::env::var("RELAYER_KEYPAIR_PATH")
            .unwrap_or_else(|_| "./relayer-keypair.json".into());

        let relayer_keypair = read_keypair_file(&keypair_path)
            .map_err(|e| format!("Failed to read keypair from {}: {}", keypair_path, e))?;

        let listen_addr: SocketAddr = std::env::var("LISTEN_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:3001".into())
            .parse()?;

        let api_keys: Vec<String> = std::env::var("API_KEYS")
            .map(|s| serde_json::from_str(&s).unwrap_or_default())
            .unwrap_or_default();

        let rate_limit_per_minute: u32 = std::env::var("RATE_LIMIT_PER_MINUTE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(60);

        Ok(Config {
            rpc_url,
            relayer_keypair,
            listen_addr,
            api_keys,
            rate_limit_per_minute,
        })
    }
}
