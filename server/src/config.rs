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
    pub ca_cert_path: Option<PathBuf>,
    pub ca_key_path: Option<PathBuf>,
}

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
        if !(60..86_400).contains(&ttl_secs) {
            bail!("DEVICE_CERT_TTL_SECONDS must be between 60 and 86400");
        }

        let allowed_app_ids = split_list("ALLOWED_APP_IDS");
        let allowed_android_packages = split_list("ALLOWED_ANDROID_PACKAGES");
        let google_credentials_file = env::var("GOOGLE_PLAY_CREDENTIALS_FILE")
            .ok()
            .filter(|s| !s.is_empty())
            .map(PathBuf::from);
        let ca_cert_path = env::var("CA_CERT_PATH")
            .ok()
            .filter(|s| !s.is_empty())
            .map(PathBuf::from);
        let ca_key_path = env::var("CA_KEY_PATH")
            .ok()
            .filter(|s| !s.is_empty())
            .map(PathBuf::from);

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
            ca_cert_path,
            ca_key_path,
        })
    }

    /// Test helper. Live checks are off and the CA lives in `data_dir`.
    pub fn for_test(data_dir: PathBuf) -> Self {
        Self {
            listen_addr: "0.0.0.0:0".into(),
            data_dir,
            attestation_mode: AttestationMode::Stub,
            device_cert_ttl: Duration::from_secs(900),
            allowed_app_ids: vec!["TESTTEAM.cloud.deltasync.capture".into()],
            allowed_android_packages: vec!["cloud.deltasync.capture".into()],
            google_credentials_file: None,
            ca_cert_path: None,
            ca_key_path: None,
        }
    }
}

pub fn read_to_string_private(path: &std::path::Path) -> anyhow::Result<String> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(text)
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
