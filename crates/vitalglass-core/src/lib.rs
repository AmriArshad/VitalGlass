//! VitalGlass core: config, Google OAuth and the Google Health API client.
//!
//! Shared by the spike CLI now and the Tauri backend later.

pub mod config;

// Re-exported so callers build the HTTP client with the same version.
pub use reqwest;

/// Every failure the core can report. The UI maps these to empty/error
/// states (not linked, permission missing, session expired, offline...).
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("config: {0}")]
    Config(String),

    #[error("sign-in failed: {0}")]
    Auth(String),

    /// Refresh token rejected (`invalid_grant`): revoked, or the 7-day
    /// limit while the OAuth consent screen is in Testing.
    #[error("session expired, sign in again")]
    SessionExpired,

    /// Access token rejected (HTTP 401).
    #[error("access token rejected")]
    Unauthorized,

    /// HTTP 400 `ACCOUNT_NOT_LINKED`.
    #[error(
        "this Google account isn't linked to Google Health yet: sign in to the Google Health app on your phone with it, then try again"
    )]
    AccountNotLinked,

    /// HTTP 403, usually a scope the user didn't grant.
    #[error("permission not granted: {0}")]
    PermissionDenied(String),

    #[error("API error {status} {status_text}: {message}")]
    Api {
        status: u16,
        status_text: String,
        message: String,
    },

    #[error("network: {0}")]
    Http(#[from] reqwest::Error),

    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
}
