# syntax=docker/dockerfile:1

FROM rust:1.96.0-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY src ./src
COPY certs ./certs
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates util-linux \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --home /nonexistent --shell /usr/sbin/nologin deltasync
COPY --from=build /src/target/release/delta-sync-server /usr/local/bin/delta-sync-server
COPY entrypoint.sh /entrypoint.sh
RUN chmod 755 /entrypoint.sh
ENV LISTEN_ADDR=0.0.0.0:8080 \
    DATA_DIR=/data \
    ATTESTATION_MODE=live \
    RUST_LOG=info
EXPOSE 8080
ENTRYPOINT ["/entrypoint.sh"]
