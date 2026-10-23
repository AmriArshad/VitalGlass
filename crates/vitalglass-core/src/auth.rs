//! Google OAuth 2.0 for an installed (Desktop app) client.
//!
//! Flow, per Google's native-app guidance:
//! 1. Bind a listener on `127.0.0.1` with a random free port.
//! 2. Open the system browser at the consent URL, with a PKCE challenge and a
//!    random `state` value.
//! 3. Google redirects to `http://127.0.0.1:<port>/?code=...&state=...`;
//!    the listener reads that one request and replies with a short page.
//! 4. Exchange the code (plus the PKCE verifier) for tokens.
//!
//! Access tokens are refreshed on demand with [`refresh`], not on a timer.

use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use url::Url;

use crate::Error;
use crate::config::ClientConfig;

const SCOPE_PREFIX: &str = "https://www.googleapis.com/auth/googlehealth";

/// The read-only scopes VitalGlass asks for. Nothing else.
pub const SCOPES: [&str; 5] = [
    "https://www.googleapis.com/auth/googlehealth.activity_and_fitness.readonly",
    "https://www.googleapis.com/auth/googlehealth.health_metrics_and_measurements.readonly",
    "https://www.googleapis.com/auth/googlehealth.sleep.readonly",
    "https://www.googleapis.com/auth/googlehealth.profile.readonly",
    "https://www.googleapis.com/auth/googlehealth.settings.readonly",
];

/// How long to wait for the user to finish the consent screen.
const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);

/// Tokens from a successful login or refresh.
#[derive(Debug, Clone)]
pub struct Tokens {
    pub access_token: String,
    /// Only returned on the first exchange (with `access_type=offline`).
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
    /// Scopes the user actually granted. May be fewer than [`SCOPES`] if
    /// they unticked some on the consent screen.
    pub granted_scopes: Vec<String>,
}

impl Tokens {
    /// True if the access token expires within the next minute.
    pub fn is_expired(&self) -> bool {
        self.expires_at - chrono::Duration::seconds(60) <= Utc::now()
    }

    pub fn has_scope(&self, scope: &str) -> bool {
        self.granted_scopes.iter().any(|s| s == scope)
    }

    /// Requested scopes the user did not grant, as short names
    /// (e.g. `sleep.readonly`).
    pub fn missing_scopes(&self) -> Vec<&'static str> {
        SCOPES
            .iter()
            .filter(|s| !self.has_scope(s))
            .map(|s| s.trim_start_matches(SCOPE_PREFIX).trim_start_matches('.'))
            .collect()
    }
}

/// Raw token endpoint response.
#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: i64,
    refresh_token: Option<String>,
    #[serde(default)]
    scope: String,
}

impl TokenResponse {
    fn into_tokens(self) -> Tokens {
        Tokens {
            access_token: self.access_token,
            refresh_token: self.refresh_token,
            expires_at: Utc::now() + chrono::Duration::seconds(self.expires_in),
            granted_scopes: self.scope.split_whitespace().map(str::to_owned).collect(),
        }
    }
}

/// Google's error body from the token endpoint, e.g.
/// `{"error": "invalid_grant", "error_description": "Token has been expired or revoked."}`.
#[derive(Deserialize)]
struct TokenError {
    error: String,
    #[serde(default)]
    error_description: String,
}

/// Run the interactive browser login and return fresh tokens.
pub async fn login(client: &ClientConfig, http: &reqwest::Client) -> Result<Tokens, Error> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}");

    let verifier = random_token();
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let state = random_token();

    let mut auth_url = Url::parse(&client.auth_uri).map_err(|e| Error::Config(e.to_string()))?;
    auth_url
        .query_pairs_mut()
        .append_pair("client_id", &client.client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", &SCOPES.join(" "))
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", &state)
        // `offline` + `consent` makes Google return a refresh token every time.
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent");

    eprintln!("Opening your browser to sign in. If it doesn't open, visit:\n\n  {auth_url}\n");
    if let Err(e) = open::that(auth_url.as_str()) {
        eprintln!("(could not launch a browser: {e})");
    }

    let code = tokio::time::timeout(LOGIN_TIMEOUT, wait_for_code(&listener, &state))
        .await
        .map_err(|_| Error::Auth("timed out waiting for the browser sign-in".into()))??;

    let resp = http
        .post(&client.token_uri)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("client_id", client.client_id.as_str()),
            ("client_secret", client.client_secret.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("code_verifier", verifier.as_str()),
        ])
        .send()
        .await?;
    token_response(resp).await
}

/// Swap a refresh token for a new access token.
///
/// An `invalid_grant` error means the refresh token is dead (revoked, or the
/// 7-day limit while the consent screen is in Testing) and the user has to
/// sign in again; that comes back as [`Error::SessionExpired`].
pub async fn refresh(client: &ClientConfig, http: &reqwest::Client, refresh_token: &str) -> Result<Tokens, Error> {
    let resp = http
        .post(&client.token_uri)
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", client.client_id.as_str()),
            ("client_secret", client.client_secret.as_str()),
        ])
        .send()
        .await?;
    let mut tokens = token_response(resp).await?;
    // Google doesn't resend the refresh token on refresh; keep the old one.
    tokens.refresh_token.get_or_insert_with(|| refresh_token.to_owned());
    Ok(tokens)
}

async fn token_response(resp: reqwest::Response) -> Result<Tokens, Error> {
    if resp.status().is_success() {
        return Ok(resp.json::<TokenResponse>().await?.into_tokens());
    }
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    match serde_json::from_str::<TokenError>(&body) {
        Ok(e) if e.error == "invalid_grant" => Err(Error::SessionExpired),
        Ok(e) => Err(Error::Auth(format!("{}: {}", e.error, e.error_description))),
        Err(_) => Err(Error::Auth(format!("token endpoint returned {status}: {body}"))),
    }
}

/// Accept connections until one carries the OAuth redirect, then return its code.
async fn wait_for_code(listener: &TcpListener, expected_state: &str) -> Result<String, Error> {
    loop {
        let (mut stream, _) = listener.accept().await?;
        let mut buf = vec![0u8; 8192];
        let n = stream.read(&mut buf).await?;
        let request = String::from_utf8_lossy(&buf[..n]);

        // First line looks like: GET /?state=...&code=...&scope=... HTTP/1.1
        let Some(target) = request.lines().next().and_then(|l| l.split_whitespace().nth(1)) else {
            continue;
        };
        let url = Url::parse(&format!("http://127.0.0.1{target}")).map_err(|e| Error::Auth(e.to_string()))?;
        let param = |name: &str| url.query_pairs().find(|(k, _)| k == name).map(|(_, v)| v.into_owned());

        // Browsers also ask for /favicon.ico etc.; ignore anything without OAuth params.
        let (code, error) = (param("code"), param("error"));
        if code.is_none() && error.is_none() {
            reply(&mut stream, "404 Not Found", "").await;
            continue;
        }

        let result = if param("state").as_deref() != Some(expected_state) {
            Err(Error::Auth("state mismatch in OAuth redirect".into()))
        } else if let Some(err) = error {
            Err(Error::Auth(format!("sign-in was cancelled or refused ({err})")))
        } else {
            Ok(code.unwrap())
        };

        let page = match &result {
            Ok(_) => "VitalGlass: signed in. You can close this tab.",
            Err(_) => "VitalGlass: sign-in failed. Check the terminal for details.",
        };
        reply(&mut stream, "200 OK", page).await;
        return result;
    }
}

async fn reply(stream: &mut tokio::net::TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    // Best effort: the code is already in hand even if the browser hung up.
    let _ = stream.write_all(response.as_bytes()).await;
}

/// 32 random bytes, base64url-encoded (43 chars): valid as a PKCE verifier
/// and as an unguessable `state`.
fn random_token() -> String {
    URL_SAFE_NO_PAD.encode(rand::random::<[u8; 32]>())
}
