//! State shared by every command and the background sync task.

use std::sync::atomic::AtomicBool;

use tokio::sync::Mutex;
use vitalglass_core::session::Session;
use vitalglass_core::store::Store;

pub struct AppState {
    /// Created lazily (see `account::ensure_session`) so a missing config
    /// file can be fixed without restarting the app. An async Mutex because
    /// sign-in and sync hold it across network calls.
    pub session: Mutex<Option<Session>>,
    /// The SQLite cache, opened at startup. `Err` holds the reason it
    /// couldn't be opened, shown in the UI instead of crashing.
    pub store: Result<Store, String>,
    /// Set while a sync runs, so overlapping triggers are ignored.
    pub syncing: AtomicBool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            session: Mutex::new(None),
            store: Store::open_default().map_err(|e| e.to_string()),
            syncing: AtomicBool::new(false),
        }
    }
}
