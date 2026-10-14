use std::sync::Arc;

use crate::ca::CertificateAuthority;
use crate::challenge::ChallengeStore;
use crate::config::Config;

pub mod attest;
pub mod ca;
pub mod challenge;
pub mod config;
pub mod error;
pub mod routes;
pub mod verify_asset;

pub struct AppState {
    pub config: Config,
    pub ca: CertificateAuthority,
    pub challenges: ChallengeStore,
    pub http: reqwest::Client,
}

impl AppState {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let ca = CertificateAuthority::load_or_create(
            &config.data_dir,
            config.ca_cert_path.as_deref(),
            config.ca_key_path.as_deref(),
        )?;
        Ok(Self {
            config,
            ca,
            challenges: ChallengeStore::new(),
            http: attest::play_integrity::http_client(),
        })
    }
}

pub fn app(state: Arc<AppState>) -> axum::Router {
    routes::router(state)
}
