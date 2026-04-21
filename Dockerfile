# Builder stage: compile the Rust binary.
#
# Explicit Dockerfile instead of relying on Railway's Railpack autodetect.
# Railpack's Rust plan caches aggressively on content hashes that were
# silently reusing the pre-feature-branch binary even after source changes
# landed in HEAD (`railway up` 4.36.0 + Railpack 0.23.0 behavior, April 2026).
# With a first-party Dockerfile Railway uses BuildKit directly and the COPY
# layer's content hash invalidates on real source changes.
FROM rust:1.88 AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src/ src/

RUN cargo build --release

# Runtime stage: debian-slim with CA certs + binary.
#
# `ca-certificates` is needed for outbound HTTPS (Solana devnet RPC, the
# internal validation service, the SAS attestation service). `libssl3`
# covers OpenSSL deps pulled in transitively by solana-client and related
# crates. Everything else (glibc, libgcc, libstdc++) comes from the base
# image.
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*
RUN useradd --system --no-create-home iam

COPY --from=builder /app/target/release/executor-node /usr/local/bin/executor-node

USER iam

# Railway injects PORT (typically 8080); executor-node's config.rs reads it
# and falls back to LISTEN_ADDR for local dev. EXPOSE here is documentation
# only — Railway ignores it.
EXPOSE 8080

CMD ["executor-node"]
