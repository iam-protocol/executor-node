# executor-node

IAM Protocol executor node. HTTP relayer service that accepts ZK proofs from the Pulse SDK and submits on-chain verification transactions to Solana. Enables walletless verification — users prove their humanity without a wallet, SOL, or crypto knowledge.

## API

### POST /verify

Accepts a Groth16 proof, submits `create_challenge` + `verify_proof` on-chain, returns the result.

```json
Request:
{
  "proof_bytes": [0, 1, 2, ...],        // 256 bytes
  "public_inputs": [[0, 1, ...], ...],  // 4 × 32 bytes
  "commitment": [0, 1, ...]             // 32 bytes
}

Response:
{
  "success": true,
  "tx_signature": "5abc...",
  "verified": true,
  "remaining_quota": 999
}
```

Requires `X-API-Key` header.

### GET /health

Returns service status (no auth required).

## Setup

```bash
# Prerequisites: Rust, Solana CLI

# Configure environment
cp .env.example .env
# Edit .env: set RPC_URL, RELAYER_KEYPAIR_PATH, API_KEYS

# Build
cargo build --release

# Run
cargo run

# Test
cargo test
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RPC_URL` | `https://api.devnet.solana.com` | Solana RPC endpoint |
| `WS_URL` | `wss://api.devnet.solana.com` | Solana WebSocket endpoint |
| `RELAYER_KEYPAIR_PATH` | `./relayer-keypair.json` | Path to relayer keypair JSON |
| `LISTEN_ADDR` | `0.0.0.0:3001` | Server bind address |
| `API_KEYS` | `[]` | JSON array of valid API keys |
| `RATE_LIMIT_PER_MINUTE` | `60` | Max requests per minute per API key |
| `INTEGRATORS` | `[]` | JSON array of `{ api_key, name, quota }` objects |

## License

MIT
