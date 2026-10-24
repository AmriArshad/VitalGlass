//! Background sync: on launch (triggered by the frontend once signed in),
//! from the Sync button, and every [`INTERVAL`] while the app is open.
//!
//! Events sent to the frontend:
//! - `sync-progress` (string): one line per step, for the status bar
//! - `data-changed`: after the hot pass and after the backfill, so the
//!   dashboard can re-query the store
//! - `sync-finished` ([`SyncFinished`])

use std::sync::atomic::Ordering;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use vitalglass_core::sync;

use crate::account::AccountStatus;
use crate::state::AppState;

pub const INTERVAL: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone, Serialize)]
pub struct SyncFinished {
    pub ok: bool,
    /// Set when the sync stopped because of the account (expired,
    /// offline, not linked...), so the UI can switch to that state.
    pub account: Option<AccountStatus>,
    /// Data types that failed while the rest synced.
    pub failures: Vec<Failure>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Failure {
    pub data_type: String,
    pub status: String,
    pub message: Option<String>,
}

/// Start a sync now. Returns `false` if one is already running.
#[tauri::command]
pub fn sync_now(app: AppHandle) -> bool {
    start(app)
}

/// Spawn the 15-minute timer. The first tick is skipped: the frontend
/// triggers the launch sync itself once it knows the user is signed in.
pub fn spawn_timer(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(INTERVAL);
        ticker.tick().await;
        loop {
            ticker.tick().await;
            start(app.clone());
        }
    });
}

/// Start a sync in the background unless one is running. Returns `false`
/// if one already was.
pub fn start(app: AppHandle) -> bool {
    let state = app.state::<AppState>();
    if state.syncing.swap(true, Ordering::SeqCst) {
        return false;
    }
    tauri::async_runtime::spawn(async move {
        let finished = run(&app).await;
        app.state::<AppState>().syncing.store(false, Ordering::SeqCst);
        let _ = app.emit("sync-finished", finished);
    });
    true
}

async fn run(app: &AppHandle) -> SyncFinished {
    let state = app.state::<AppState>();
    let store = match &state.store {
        Ok(store) => store,
        Err(message) => {
            return stopped(AccountStatus::Error {
                message: message.clone(),
            });
        }
    };

    let mut guard = state.session.lock().await;
    let session = match crate::account::ensure_session(&mut guard) {
        Ok(s) => s,
        Err(status) => return stopped(status),
    };
    if session.tokens().is_none() {
        match session.restore().await {
            Ok(true) => {}
            Ok(false) => return stopped(AccountStatus::SignedOut),
            Err(e) => return stopped(e.into()),
        }
    }

    let mut progress = |line: &str| {
        log::info!("sync: {line}");
        let _ = app.emit("sync-progress", line);
    };
    let mut results = match sync::hot(session, store, &mut progress).await {
        Ok(r) => r,
        Err(e) => return stopped(e.into()),
    };
    let _ = app.emit("data-changed", ());

    match sync::backfill(session, store, &mut progress).await {
        Ok(r) if !r.is_empty() => {
            results.extend(r);
            let _ = app.emit("data-changed", ());
        }
        Ok(_) => {}
        Err(e) => return stopped(e.into()),
    }

    let failures: Vec<Failure> = results
        .into_iter()
        .filter(|r| r.status != "ok")
        .map(|r| Failure {
            data_type: r.data_type.to_owned(),
            status: r.status.to_owned(),
            message: r.message,
        })
        .collect();
    SyncFinished {
        ok: failures.is_empty(),
        account: None,
        failures,
    }
}

fn stopped(account: AccountStatus) -> SyncFinished {
    SyncFinished {
        ok: false,
        account: Some(account),
        failures: Vec::new(),
    }
}
