use serde::Deserialize;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{read_keypair_file, Keypair};
use std::net::SocketAddr;
use std::str::FromStr;

#[derive(Deserialize)]
pub struct IntegratorConfig {
    pub api_key: String,
    pub name: String,
    pub quota: u64,
}

pub struct Config {
    pub rpc_url: String,
    pub ws_url: String,
    pub relayer_keypair: Keypair,
    pub listen_addr: SocketAddr,
    pub api_keys: Vec<String>,
    pub rate_limit_per_minute: u32,
    pub integrators: Vec<IntegratorConfig>,
    pub cors_origins: Vec<String>,
    pub sas_credential_pda: Option<Pubkey>,
    pub sas_schema_pda: Option<Pubkey>,
    pub sas_attestation_ttl_days: u64,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let rpc_url =
            std::env::var("RPC_URL").unwrap_or_else(|_| "https://api.devnet.solana.com".into());

        let ws_url =
            std::env::var("WS_URL").unwrap_or_else(|_| "wss://api.devnet.solana.com".into());

        let relayer_keypair = if let Ok(json) = std::env::var("RELAYER_KEYPAIR") {
            let bytes: Vec<u8> = serde_json::from_str(&json)
                .map_err(|e| format!("RELAYER_KEYPAIR is not valid JSON: {e}"))?;
            Keypair::try_from(bytes.as_slice())
                .map_err(|e| format!("RELAYER_KEYPAIR contains invalid keypair: {e}"))?
        } else {
            let keypair_path = std::env::var("RELAYER_KEYPAIR_PATH")
                .unwrap_or_else(|_| "./relayer-keypair.json".into());
            read_keypair_file(&keypair_path)
                .map_err(|e| format!("Failed to read keypair from {keypair_path}: {e}"))?
        };

        let listen_addr: SocketAddr = if let Ok(port) = std::env::var("PORT") {
            format!("0.0.0.0:{port}").parse()?
        } else {
            std::env::var("LISTEN_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:3001".into())
                .parse()?
        };

        let api_keys: Vec<String> = match std::env::var("API_KEYS") {
            Ok(s) => serde_json::from_str(&s)
                .map_err(|e| format!("API_KEYS contains invalid JSON: {e}"))?,
            Err(_) => vec![],
        };

        let rate_limit_per_minute: u32 = match std::env::var("RATE_LIMIT_PER_MINUTE") {
            Ok(s) => s.parse()
                .map_err(|e| format!("RATE_LIMIT_PER_MINUTE is not a valid u32: {e}"))?,
            Err(_) => 60,
        };

        let integrators: Vec<IntegratorConfig> = match std::env::var("INTEGRATORS") {
            Ok(s) => serde_json::from_str(&s)
                .map_err(|e| format!("INTEGRATORS contains invalid JSON: {e}"))?,
            Err(_) => vec![],
        };

        let api_keys = if api_keys.is_empty() && !integrators.is_empty() {
            integrators.iter().map(|i| i.api_key.clone()).collect()
        } else {
            api_keys
        };

        let cors_origins: Vec<String> = match std::env::var("CORS_ORIGINS") {
            Ok(s) => serde_json::from_str(&s)
                .map_err(|e| format!("CORS_ORIGINS contains invalid JSON: {e}"))?,
            Err(_) => vec![],
        };

        let sas_credential_pda = std::env::var("SAS_CREDENTIAL_PDA")
            .ok()
            .and_then(|s| Pubkey::from_str(&s).ok());

        let sas_schema_pda = std::env::var("SAS_SCHEMA_PDA")
            .ok()
            .and_then(|s| Pubkey::from_str(&s).ok());

        let sas_attestation_ttl_days: u64 = std::env::var("SAS_ATTESTATION_TTL_DAYS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);

        Ok(Config {
            rpc_url,
            ws_url,
            relayer_keypair,
            listen_addr,
            api_keys,
            rate_limit_per_minute,
            integrators,
            cors_origins,
            sas_credential_pda,
            sas_schema_pda,
            sas_attestation_ttl_days,
        })
    }
}
