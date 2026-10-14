use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};

const CHALLENGE_TTL: Duration = Duration::from_secs(5 * 60);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Ios,
    Android,
}

impl Platform {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "ios" => Some(Self::Ios),
            "android" => Some(Self::Android),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ios => "ios",
            Self::Android => "android",
        }
    }
}

struct Pending {
    platform: Platform,
    created: Instant,
}

/// Single-process, single-use challenges. A second replica will not see them.
pub struct ChallengeStore {
    inner: Mutex<HashMap<String, Pending>>,
}

impl Default for ChallengeStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ChallengeStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    pub fn issue(&self, platform: Platform) -> String {
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        let challenge = URL_SAFE_NO_PAD.encode(bytes);
        self.inner.lock().expect("challenge lock").insert(
            challenge.clone(),
            Pending {
                platform,
                created: Instant::now(),
            },
        );
        challenge
    }

    /// Removes the challenge so it cannot be reused.
    pub fn consume(&self, challenge: &str, platform: Platform) -> Result<(), String> {
        let mut guard = self.inner.lock().expect("challenge lock");
        let Some(pending) = guard.remove(challenge) else {
            return Err("unknown or already used challenge".into());
        };
        if pending.created.elapsed() > CHALLENGE_TTL {
            return Err("challenge expired".into());
        }
        if pending.platform != platform {
            return Err("challenge was issued for a different platform".into());
        }
        Ok(())
    }
}

/// Binds a hardware public key to a server challenge.
///
/// `SHA-256(challenge_utf8 || 0x00 || uncompressed P-256 point)`.
/// App Attest uses these 32 bytes as `clientDataHash`.
/// Play Integrity uses the base64url form (no padding) as the nonce.
pub fn client_data_hash(challenge: &str, uncompressed_point: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(challenge.as_bytes());
    hasher.update([0u8]);
    hasher.update(uncompressed_point);
    hasher.finalize().into()
}

pub fn integrity_nonce(challenge: &str, uncompressed_point: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(client_data_hash(challenge, uncompressed_point))
}

/// 32 random bytes, URL-safe base64 without padding. Used only by tests via the store.
pub fn random_id() -> String {
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
