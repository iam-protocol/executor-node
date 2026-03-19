# executor-node

IAM Protocol executor node. HTTP relayer service that accepts ZK proofs from the Pulse SDK and submits on-chain verification transactions to Solana. Enables walletless verification — users prove their humanity without a wallet, SOL, or crypto knowledge.

## API

### POST /verify

Accepts a Groth16 proof, submits `create_challenge` + `verify_proof` on-chain, returns the result.

```json
Request:
{
  "proof_bytes": [0, 1, 2, ...],        // 256 bytes
  "public_inputs": [[0, 1, ...], ...],  // 3 × 32 bytes
  "commitment": [0, 1, ...],            // 32 bytes
  "is_first_verification": true
}

Response:
{
  "success": true,
  "tx_signature": "5abc...",
  "verified": true
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

## License

Proprietary. Not open source.
