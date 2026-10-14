use std::{fs, path::Path, time::Duration};

use anyhow::{anyhow, bail};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug)]
pub struct PlayIntegrityResult {
    pub package_name: String,
    pub app_recognition: String,
    pub device_recognition: Vec<String>,
}

pub async fn verify(
    http: &reqwest::Client,
    credentials_file: &Path,
    package_name: &str,
    integrity_token: &str,
    expected_nonce: &str,
) -> anyhow::Result<PlayIntegrityResult> {
    let creds = ServiceAccount::load(credentials_file)?;
    let access_token = creds.access_token(http).await?;
    let url =
        format!("https://playintegrity.googleapis.com/v1/{package_name}:decodeIntegrityToken");
    let response = http
        .post(url)
        .bearer_auth(access_token)
        .json(&serde_json::json!({ "integrity_token": integrity_token }))
        .send()
        .await
        .map_err(|err| anyhow!("Play Integrity request failed: {err}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|err| anyhow!("reading Play Integrity response: {err}"))?;
    if !status.is_success() {
        bail!("Play Integrity decode returned {status}: {body}");
    }
    let decoded: DecodeResponse =
        serde_json::from_str(&body).map_err(|err| anyhow!("Play Integrity JSON: {err}"))?;
    let payload = decoded
        .token_payload_external
        .ok_or_else(|| anyhow!("Play Integrity response has no token payload"))?;
    let details = payload
        .request_details
        .ok_or_else(|| anyhow!("Play Integrity response has no request details"))?;
    if details.request_package_name.as_deref() != Some(package_name) {
        bail!("Play Integrity package does not match {package_name}");
    }
    if details.nonce.as_deref() != Some(expected_nonce) {
        bail!("Play Integrity nonce does not commit to this challenge and device key");
    }
    let app = payload
        .app_integrity
        .and_then(|a| a.app_recognition_verdict)
        .unwrap_or_default();
    if app != "PLAY_RECOGNIZED" {
        bail!("Play Integrity app verdict is {app}, expected PLAY_RECOGNIZED");
    }
    let device = payload
        .device_integrity
        .map(|d| d.device_recognition_verdict)
        .unwrap_or_default();
    if !device.iter().any(|v| v == "MEETS_DEVICE_INTEGRITY") {
        bail!("Play Integrity device verdict does not meet MEETS_DEVICE_INTEGRITY");
    }
    Ok(PlayIntegrityResult {
        package_name: package_name.to_string(),
        app_recognition: app,
        device_recognition: device,
    })
}

#[derive(Deserialize)]
struct ServiceAccount {
    client_email: String,
    private_key: String,
    token_uri: String,
}

#[derive(Serialize)]
struct JwtClaims<'a> {
    iss: &'a str,
    scope: &'a str,
    aud: &'a str,
    iat: i64,
    exp: i64,
}

impl ServiceAccount {
    fn load(path: &Path) -> anyhow::Result<Self> {
        let text = fs::read_to_string(path)
            .map_err(|_| anyhow!("could not read Google credentials file"))?;
        serde_json::from_str(&text)
            .map_err(|_| anyhow!("Google credentials file is not a service account JSON"))
    }

    async fn access_token(&self, http: &reqwest::Client) -> anyhow::Result<String> {
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let claims = JwtClaims {
            iss: &self.client_email,
            scope: "https://www.googleapis.com/auth/playintegrity",
            aud: &self.token_uri,
            iat: now,
            exp: now + 3600,
        };
        let assertion = encode(
            &Header::new(Algorithm::RS256),
            &claims,
            &EncodingKey::from_rsa_pem(self.private_key.as_bytes())
                .map_err(|_| anyhow!("service account private key is not RSA PEM"))?,
        )
        .map_err(|err| anyhow!("signing Google JWT: {err}"))?;
        let response = http
            .post(&self.token_uri)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", assertion.as_str()),
            ])
            .send()
            .await
            .map_err(|err| anyhow!("Google token request failed: {err}"))?;
        let status = response.status();
        let token: TokenResponse = response
            .json()
            .await
            .map_err(|err| anyhow!("Google token response ({status}): {err}"))?;
        token
            .access_token
            .ok_or_else(|| anyhow!("Google token response did not include an access token"))
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
}

#[derive(Deserialize)]
struct DecodeResponse {
    #[serde(rename = "tokenPayloadExternal")]
    token_payload_external: Option<TokenPayload>,
}

#[derive(Deserialize)]
struct TokenPayload {
    #[serde(rename = "requestDetails")]
    request_details: Option<RequestDetails>,
    #[serde(rename = "appIntegrity")]
    app_integrity: Option<AppIntegrity>,
    #[serde(rename = "deviceIntegrity")]
    device_integrity: Option<DeviceIntegrity>,
}

#[derive(Deserialize)]
struct RequestDetails {
    #[serde(rename = "requestPackageName")]
    request_package_name: Option<String>,
    nonce: Option<String>,
}

#[derive(Deserialize)]
struct AppIntegrity {
    #[serde(rename = "appRecognitionVerdict")]
    app_recognition_verdict: Option<String>,
}

#[derive(Deserialize)]
struct DeviceIntegrity {
    #[serde(rename = "deviceRecognitionVerdict", default)]
    device_recognition_verdict: Vec<String>,
}

pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .expect("reqwest client")
}
