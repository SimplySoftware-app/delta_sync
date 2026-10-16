# syntax=docker/dockerfile:1

FROM rust:1.96.0-bookworm AS build
WORKDIR /src
# Build context is the repository root (Cargo workspace): see
# server/docker-compose.yml and server/Dockerfile.dockerignore.
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY server/Cargo.toml ./server/Cargo.toml
COPY server/src ./server/src
COPY server/certs ./server/certs
COPY crates ./crates
RUN cargo build --release --locked -p delta-sync-server

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates util-linux \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --home /nonexistent --shell /usr/sbin/nologin deltasync
COPY --from=build /src/target/release/delta-sync-server /usr/local/bin/delta-sync-server
COPY server/entrypoint.sh /entrypoint.sh
RUN chmod 755 /entrypoint.sh
ENV LISTEN_ADDR=0.0.0.0:8080 \
    DATA_DIR=/data \
    ATTESTATION_MODE=live \
    RUST_LOG=info
EXPOSE 8080
ENTRYPOINT ["/entrypoint.sh"]
