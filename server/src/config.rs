use std::{env, fs, path::PathBuf, time::Duration};

use anyhow::{bail, Context};

/// `stub` issues device certificates without checking App Attest or Play Integrity.
/// `live` requires those checks and refuses to start half-configured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttestationMode {
    Stub,
    Live,
}

impl AttestationMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stub => "stub",
            Self::Live => "live",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub listen_addr: String,
    pub data_dir: PathBuf,
    pub attestation_mode: AttestationMode,
    pub device_cert_ttl: Duration,
    pub allowed_app_ids: Vec<String>,
    pub allowed_android_packages: Vec<String>,
    pub google_credentials_file: Option<PathBuf>,
    pub pki: PkiPaths,
    /// RFC 3161 time-stamp authority used when the service signs a derivative.
    /// `None` disables time-stamping.
    pub tsa_url: Option<String>,
    /// Shared secret for `/internal/seal` (Bearer). `None` leaves the route
    /// answering 503.
    pub seal_token: Option<String>,
    /// How many seals may run at once. Each one holds several copies of the
    /// asset in memory.
    pub seal_max_concurrency: usize,
}

/// Optional overrides for the PKI files that otherwise live in `DATA_DIR`.
/// Use them to provision an offline root and the two ICAs from outside the
/// service. Either all of the root and ICA paths are set or none are.
#[derive(Clone, Debug, Default)]
pub struct PkiPaths {
    pub root_cert: Option<PathBuf>,
    pub device_ica_cert: Option<PathBuf>,
    pub device_ica_key: Option<PathBuf>,
    pub service_ica_cert: Option<PathBuf>,
    pub service_ica_key: Option<PathBuf>,
    pub service_cert: Option<PathBuf>,
    pub service_key: Option<PathBuf>,
}

/// Longest life of a device certificate.
pub const MAX_DEVICE_CERT_TTL_SECONDS: u64 = 86_400;
pub const DEFAULT_TSA_URL: &str = "http://timestamp.digicert.com";
pub const MIN_SEAL_TOKEN_LEN: usize = 32;

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let listen_addr = env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
        if listen_addr.starts_with("127.0.0.1:") || listen_addr.starts_with("localhost:") {
            bail!(
                "LISTEN_ADDR is {listen_addr}. Bind 0.0.0.0 inside the container so Caddy can reach it by container name. Do not publish 127.0.0.1 on the host."
            );
        }

        let data_dir = PathBuf::from(env::var("DATA_DIR").unwrap_or_else(|_| "/data".to_string()));
        let attestation_mode = match env::var("ATTESTATION_MODE")
            .unwrap_or_else(|_| "stub".to_string())
            .to_ascii_lowercase()
            .as_str()
        {
            "stub" | "dev" => AttestationMode::Stub,
            "live" | "enforce" => AttestationMode::Live,
            other => bail!("ATTESTATION_MODE must be stub or live, got {other}"),
        };

        let ttl_secs: u64 = env::var("DEVICE_CERT_TTL_SECONDS")
            .unwrap_or_else(|_| "900".to_string())
            .parse()
            .context("DEVICE_CERT_TTL_SECONDS")?;
        if !(60..=MAX_DEVICE_CERT_TTL_SECONDS).contains(&ttl_secs) {
            bail!("DEVICE_CERT_TTL_SECONDS must be between 60 and {MAX_DEVICE_CERT_TTL_SECONDS} (24 h)");
        }

        let allowed_app_ids = split_list("ALLOWED_APP_IDS");
        let allowed_android_packages = split_list("ALLOWED_ANDROID_PACKAGES");
        let google_credentials_file = env::var("GOOGLE_PLAY_CREDENTIALS_FILE")
            .ok()
            .filter(|s| !s.is_empty())
            .map(PathBuf::from);
        let pki = PkiPaths {
            root_cert: path_var("ROOT_CERT_PATH"),
            device_ica_cert: path_var("DEVICE_ICA_CERT_PATH"),
            device_ica_key: path_var("DEVICE_ICA_KEY_PATH"),
            service_ica_cert: path_var("SERVICE_ICA_CERT_PATH"),
            service_ica_key: path_var("SERVICE_ICA_KEY_PATH"),
            service_cert: path_var("SERVICE_CERT_PATH"),
            service_key: path_var("SERVICE_KEY_PATH"),
        };
        // Unset: use the public default. Empty or "off": no time-stamping.
        let tsa_url = match env::var("TSA_URL") {
            Err(_) => Some(DEFAULT_TSA_URL.to_string()),
            Ok(value) => match value.trim() {
                "" | "off" | "none" => None,
                url if url.starts_with("http://") || url.starts_with("https://") => {
                    Some(url.to_string())
                }
                other => bail!("TSA_URL must be an http(s) URL, empty or off, got {other}"),
            },
        };
        let seal_token = env::var("MEDIA_SEAL_TOKEN").ok().filter(|s| !s.is_empty());
        if let Some(token) = &seal_token {
            if token.len() < MIN_SEAL_TOKEN_LEN {
                bail!("MEDIA_SEAL_TOKEN must be at least {MIN_SEAL_TOKEN_LEN} characters");
            }
        }
        let seal_max_concurrency: usize = env::var("SEAL_MAX_CONCURRENCY")
            .unwrap_or_else(|_| "2".to_string())
            .parse()
            .context("SEAL_MAX_CONCURRENCY")?;
        if seal_max_concurrency == 0 {
            bail!("SEAL_MAX_CONCURRENCY must be at least 1");
        }

        if attestation_mode == AttestationMode::Live {
            if allowed_app_ids.is_empty() {
                bail!("live mode requires ALLOWED_APP_IDS (Apple team id + bundle id, e.g. ABCDE12345.cloud.deltasync.capture)");
            }
            if allowed_android_packages.is_empty() {
                bail!("live mode requires ALLOWED_ANDROID_PACKAGES");
            }
            let Some(path) = &google_credentials_file else {
                bail!("live mode requires GOOGLE_PLAY_CREDENTIALS_FILE");
            };
            if !path.is_file() {
                bail!(
                    "GOOGLE_PLAY_CREDENTIALS_FILE does not exist at {} (do not commit this file)",
                    path.display()
                );
            }
        }

        Ok(Self {
            listen_addr,
            data_dir,
            attestation_mode,
            device_cert_ttl: Duration::from_secs(ttl_secs),
            allowed_app_ids,
            allowed_android_packages,
            google_credentials_file,
            pki,
            tsa_url,
            seal_token,
            seal_max_concurrency,
        })
    }

    /// Test helper. Live checks are off, the PKI lives in `data_dir`, there is no
    /// time-stamp authority (tests never use the network) and no seal token.
    pub fn for_test(data_dir: PathBuf) -> Self {
        Self {
            listen_addr: "0.0.0.0:0".into(),
            data_dir,
            attestation_mode: AttestationMode::Stub,
            device_cert_ttl: Duration::from_secs(900),
            allowed_app_ids: vec!["TESTTEAM.cloud.deltasync.capture".into()],
            allowed_android_packages: vec!["cloud.deltasync.capture".into()],
            google_credentials_file: None,
            pki: PkiPaths::default(),
            tsa_url: None,
            seal_token: None,
            seal_max_concurrency: 2,
        }
    }
}

pub fn read_to_string_private(path: &std::path::Path) -> anyhow::Result<String> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(text)
}

fn path_var(key: &str) -> Option<PathBuf> {
    env::var(key)
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

fn split_list(key: &str) -> Vec<String> {
    env::var(key)
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}
