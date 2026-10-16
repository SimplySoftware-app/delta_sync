use std::sync::Arc;

use tokio::sync::Semaphore;

use crate::ca::CertificateAuthority;
use crate::challenge::ChallengeStore;
use crate::config::Config;

pub mod attest;
pub mod ca;
pub mod challenge;
pub mod config;
pub mod error;
pub mod routes;
pub mod seal;
pub mod verify_asset;

pub struct AppState {
    pub config: Config,
    pub ca: CertificateAuthority,
    pub challenges: ChallengeStore,
    pub http: reqwest::Client,
    /// Bounds concurrent seals. Each holds several copies of the asset in memory.
    pub seal_permits: Semaphore,
}

impl AppState {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let ca = CertificateAuthority::load_or_create(&config.data_dir, &config.pki)?;
        let seal_permits = Semaphore::new(config.seal_max_concurrency);
        Ok(Self {
            config,
            ca,
            challenges: ChallengeStore::new(),
            http: attest::play_integrity::http_client(),
            seal_permits,
        })
    }
}

pub fn app(state: Arc<AppState>) -> axum::Router {
    routes::router(state)
}
