//! Loading the OAuth client credentials from outside the repo.
//!
//! The client secret JSON downloaded from Google Cloud Console lives at
//! `$XDG_CONFIG_HOME/vitalglass/client_secret.json` (normally
//! `~/.config/vitalglass/client_secret.json`). It is never read from, or
//! written to, the working tree.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::Error;

/// The parts of Google's client secret file that the OAuth flow needs.
#[derive(Debug, Clone, Deserialize)]
pub struct ClientConfig {
    pub client_id: String,
    pub client_secret: String,
    pub auth_uri: String,
    pub token_uri: String,
}

/// Google wraps the fields in a key named after the client type:
/// `"installed"` for a Desktop app client, `"web"` for a Web client.
#[derive(Deserialize)]
struct ClientSecretFile {
    installed: Option<ClientConfig>,
    web: Option<ClientConfig>,
}

/// `~/.config/vitalglass`, honouring `$XDG_CONFIG_HOME`.
pub fn config_dir() -> Result<PathBuf, Error> {
    dirs::config_dir()
        .map(|d| d.join("vitalglass"))
        .ok_or_else(|| Error::Config("could not determine the config directory".into()))
}

/// Default location of the client secret file.
pub fn client_secret_path() -> Result<PathBuf, Error> {
    Ok(config_dir()?.join("client_secret.json"))
}

impl ClientConfig {
    /// Load from the default location.
    pub fn load() -> Result<Self, Error> {
        Self::load_from(&client_secret_path()?)
    }

    pub fn load_from(path: &Path) -> Result<Self, Error> {
        let raw = std::fs::read_to_string(path).map_err(|e| {
            Error::Config(format!(
                "cannot read {}: {e}. Download the OAuth client JSON from Google Cloud Console and put it there.",
                path.display()
            ))
        })?;
        let file: ClientSecretFile = serde_json::from_str(&raw)
            .map_err(|e| Error::Config(format!("{} is not a client secret file: {e}", path.display())))?;
        file.installed
            .or(file.web)
            .ok_or_else(|| Error::Config(format!("{} has no \"installed\" or \"web\" section", path.display())))
    }
}
