//! Account and sign-in commands called from the frontend.
//!
//! Every command returns an [`AccountStatus`], which the UI renders
//! directly. Failures become a status (signed out, expired, not linked,
//! offline...) rather than an exception, so the UI never has to parse
//! error strings.

use serde::Serialize;
use tauri::State;
use vitalglass_core::Error;
use vitalglass_core::config::{self, ClientConfig};
use vitalglass_core::session::Session;

use crate::state::AppState;

/// What the UI should show for the account. Serialised as
/// `{"state": "signed_in", "missing_scopes": [...]}` and so on.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum AccountStatus {
    /// `client_secret.json` is missing or unreadable.
    NoConfig {
        path: String,
        message: String,
    },
    /// No refresh token stored: first run, or the user signed out.
    SignedOut,
    /// Google rejected the stored refresh token (7-day Testing limit, or revoked).
    SessionExpired,
    /// Signed in, but the account isn't linked in the Google Health app.
    NotLinked,
    /// Couldn't reach Google.
    Offline {
        message: String,
    },
    /// Secret Service / KWallet unavailable.
    KeyringUnavailable {
        message: String,
    },
    /// Signed in and the account is linked. `missing_scopes` lists any
    /// scopes the user unticked on the consent screen.
    SignedIn {
        missing_scopes: Vec<String>,
    },
    Error {
        message: String,
    },
}

impl From<Error> for AccountStatus {
    fn from(e: Error) -> Self {
        match e {
            Error::SessionExpired | Error::Unauthorized => Self::SessionExpired,
            Error::AccountNotLinked => Self::NotLinked,
            Error::Keyring(message) => Self::KeyringUnavailable { message },
            Error::Http(e) if e.is_connect() || e.is_timeout() => Self::Offline { message: e.to_string() },
            other => Self::Error {
                message: other.to_string(),
            },
        }
    }
}

/// Current status: restores the session from the keyring if needed, then
/// checks the account with `GET identity`.
#[tauri::command]
pub async fn account_status(state: State<'_, AppState>) -> Result<AccountStatus, ()> {
    let mut guard = state.session.lock().await;
    let session = match ensure_session(&mut guard) {
        Ok(s) => s,
        Err(status) => return Ok(status),
    };
    if session.tokens().is_none() {
        match session.restore().await {
            Ok(true) => {}
            Ok(false) => return Ok(AccountStatus::SignedOut),
            Err(e) => return Ok(e.into()),
        }
    }
    Ok(check_identity(session).await)
}

/// Open the system browser for Google sign-in, then check the account.
#[tauri::command]
pub async fn sign_in(state: State<'_, AppState>) -> Result<AccountStatus, ()> {
    let mut guard = state.session.lock().await;
    let session = match ensure_session(&mut guard) {
        Ok(s) => s,
        Err(status) => return Ok(status),
    };
    if let Err(e) = session.sign_in().await {
        return Ok(e.into());
    }
    Ok(check_identity(session).await)
}

/// Revoke the token and remove it from the keyring.
#[tauri::command]
pub async fn sign_out(state: State<'_, AppState>) -> Result<AccountStatus, ()> {
    let mut guard = state.session.lock().await;
    let session = match ensure_session(&mut guard) {
        Ok(s) => s,
        Err(status) => return Ok(status),
    };
    Ok(match session.sign_out().await {
        Ok(()) => AccountStatus::SignedOut,
        Err(e) => e.into(),
    })
}

/// Create the session on first use, loading `client_secret.json`.
pub fn ensure_session(slot: &mut Option<Session>) -> Result<&mut Session, AccountStatus> {
    if slot.is_none() {
        let no_config = |message: String| AccountStatus::NoConfig {
            path: config::client_secret_path()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            message,
        };
        let client = ClientConfig::load().map_err(|e| no_config(e.to_string()))?;
        *slot = Some(Session::new(client).map_err(AccountStatus::from)?);
    }
    Ok(slot.as_mut().expect("set above"))
}

async fn check_identity(session: &mut Session) -> AccountStatus {
    let api = match session.api().await {
        Ok(api) => api,
        Err(e) => return e.into(),
    };
    match api.identity().await {
        Ok(_) => AccountStatus::SignedIn {
            missing_scopes: session
                .tokens()
                .map(|t| t.missing_scopes().into_iter().map(String::from).collect())
                .unwrap_or_default(),
        },
        Err(e) => e.into(),
    }
}
