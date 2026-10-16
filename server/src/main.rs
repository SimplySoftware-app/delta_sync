use std::sync::Arc;

use delta_sync_server::{app, config::Config, AppState};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("delta_sync_server=info".parse()?),
        )
        .init();

    let config = Config::from_env()?;
    if config.attestation_mode == delta_sync_server::config::AttestationMode::Stub {
        tracing::warn!(
            "ATTESTATION_MODE=stub. Device certificates will be issued without App Attest or Play Integrity. Do not use this on a public host."
        );
    }
    if config.seal_token.is_none() {
        tracing::warn!("MEDIA_SEAL_TOKEN is not set. /internal/seal answers 503.");
    }
    if config.tsa_url.is_none() {
        tracing::warn!("TSA_URL is off. Service seals carry no RFC 3161 time-stamp.");
    }
    let listen = config.listen_addr.clone();
    let state = Arc::new(AppState::new(config)?);
    tracing::info!(
        listen = %listen,
        ca = %state.ca.fingerprint_sha256(),
        "delta-sync signing authority listening"
    );
    let listener = tokio::net::TcpListener::bind(&listen).await?;
    axum::serve(listener, app(state)).await?;
    Ok(())
}
