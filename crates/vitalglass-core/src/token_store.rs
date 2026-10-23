//! Refresh token storage in the OS keyring.
//!
//! On Linux this is the Secret Service D-Bus API, which KWallet (and GNOME
//! Keyring) implement. The token never touches a plaintext file. Look for it
//! in KWalletManager under the service name `vitalglass`.
//!
//! These calls block on D-Bus, so async callers should go through
//! `tokio::task::spawn_blocking` (as [`crate::session`] does).

use keyring::Entry;

use crate::Error;

const SERVICE: &str = "vitalglass";
const ACCOUNT: &str = "google-refresh-token";

fn entry() -> Result<Entry, Error> {
    Entry::new(SERVICE, ACCOUNT).map_err(keyring_error)
}

/// The stored refresh token, or `None` if nobody has signed in yet.
pub fn load() -> Result<Option<String>, Error> {
    match entry()?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(keyring_error(e)),
    }
}

pub fn save(refresh_token: &str) -> Result<(), Error> {
    entry()?.set_password(refresh_token).map_err(keyring_error)
}

/// Remove the stored token. Not an error if there wasn't one.
pub fn clear() -> Result<(), Error> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(keyring_error(e)),
    }
}

fn keyring_error(e: keyring::Error) -> Error {
    Error::Keyring(e.to_string())
}
