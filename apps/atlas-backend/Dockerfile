# Build context is the repo root (docker-compose.atlas.yml), not this
# directory - every COPY path below is relative to the workspace root.

# === Build

FROM rust:1-slim-bookworm AS builder
WORKDIR /app

# sqlx's postgres feature and argon2 both need a C toolchain + OpenSSL headers.
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Dependencies first, cached separately from source so an app-code change
# doesn't force every crate to recompile.
COPY Cargo.toml Cargo.lock ./
COPY apps/atlas-backend/Cargo.toml apps/atlas-backend/Cargo.toml
RUN mkdir -p apps/atlas-backend/src \
    && echo "fn main() {}" > apps/atlas-backend/src/main.rs \
    && cargo build --release --package atlas-backend \
    && rm -rf apps/atlas-backend/src

COPY apps/atlas-backend/src apps/atlas-backend/src
# Backdate the stub's mtime so Cargo actually recompiles main.rs - the
# rm/copy above can otherwise leave a newer stub artifact than the real
# source depending on filesystem timestamp resolution.
RUN touch apps/atlas-backend/src/main.rs \
    && cargo build --release --package atlas-backend

# === Runtime

FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/atlas-backend /usr/local/bin/atlas-backend

EXPOSE 5755
CMD ["atlas-backend"]
