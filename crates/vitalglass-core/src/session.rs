//! A signed-in session: OAuth client config, HTTP client and current tokens.
//!
//! Only the refresh token is persisted (in the keyring, see
//! [`crate::token_store`]). The access token lives in memory and is
//! refreshed on demand when it's about to expire.

use crate::api::HealthClient;
use crate::auth::{self, Tokens};
use crate::config::ClientConfig;
use crate::{Error, token_store};

const REVOKE_URL: &str = "https://oauth2.googleapis.com/revoke";

pub struct Session {
    client: ClientConfig,
    http: reqwest::Client,
    tokens: Option<Tokens>,
}

impl Session {
    pub fn new(client: ClientConfig) -> Result<Self, Error> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("vitalglass/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            client,
            http,
            tokens: None,
        })
    }

    /// Current tokens, or `None` if signed out.
    pub fn tokens(&self) -> Option<&Tokens> {
        self.tokens.as_ref()
    }

    /// Try to resume from a refresh token in the keyring.
    ///
    /// Returns `Ok(false)` if there is no stored token. A stored token that
    /// Google rejects is removed and reported as [`Error::SessionExpired`].
    pub async fn restore(&mut self) -> Result<bool, Error> {
        let Some(refresh_token) = blocking(token_store::load).await? else {
            return Ok(false);
        };
        self.tokens = Some(self.refresh_with(&refresh_token).await?);
        Ok(true)
    }

    /// Interactive browser sign-in. Replaces any stored refresh token.
    pub async fn sign_in(&mut self) -> Result<(), Error> {
        let tokens = auth::login(&self.client, &self.http).await?;
        let refresh_token = tokens
            .refresh_token
            .clone()
            .ok_or_else(|| Error::Auth("Google returned no refresh token".into()))?;
        blocking(move || token_store::save(&refresh_token)).await?;
        self.tokens = Some(tokens);
        Ok(())
    }

    /// Forget the session locally and revoke the token at Google.
    pub async fn sign_out(&mut self) -> Result<(), Error> {
        if let Some(token) = self.tokens.take().and_then(|t| t.refresh_token) {
            // Best effort: signing out locally matters more than the revoke.
            let _ = self.http.post(REVOKE_URL).form(&[("token", token)]).send().await;
        }
        blocking(token_store::clear).await
    }

    /// A Health API client with a valid access token, refreshing first if
    /// the current one has expired.
    pub async fn api(&mut self) -> Result<HealthClient, Error> {
        let tokens = self.tokens.as_ref().ok_or(Error::SessionExpired)?;
        if tokens.is_expired() {
            let refresh_token = tokens.refresh_token.clone().ok_or(Error::SessionExpired)?;
            self.tokens = Some(self.refresh_with(&refresh_token).await?);
        }
        let access_token = self.tokens.as_ref().expect("set above").access_token.clone();
        Ok(HealthClient::new(self.http.clone(), access_token))
    }

    async fn refresh_with(&mut self, refresh_token: &str) -> Result<Tokens, Error> {
        match auth::refresh(&self.client, &self.http, refresh_token).await {
            Err(Error::SessionExpired) => {
                self.tokens = None;
                blocking(token_store::clear).await?;
                Err(Error::SessionExpired)
            }
            other => other,
        }
    }
}

/// Run a blocking keyring call off the async runtime's worker threads.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, Error> + Send + 'static) -> Result<T, Error> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| Error::Keyring(e.to_string()))?
}
