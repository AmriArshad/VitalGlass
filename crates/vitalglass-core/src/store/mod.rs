//! Local SQLite cache at `$XDG_DATA_HOME/vitalglass/vitalglass.db`.
//!
//! The dashboard only ever reads from here; [`crate::sync`] fills it.
//! Writes live in `write.rs`, dashboard queries in `read.rs`.
//!
//! rusqlite is synchronous. Calls are short local writes and reads, so the
//! sync code calls them directly; the UI layer wraps reads in
//! `spawn_blocking`.

mod read;
mod write;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, params};

pub use read::*;

use crate::Error;

/// Schema migrations, applied in order. SQLite's `user_version` pragma
/// records how many have run. Never edit a released migration; add a new one.
const MIGRATIONS: &[&str] = &[
    include_str!("001_initial.sql"),
    include_str!("002_session_sources.sql"),
    include_str!("003_steps_hourly.sql"),
];

pub struct Store {
    // A Mutex makes Store shareable between the sync task and UI commands.
    // Each method holds the lock only for its own short query.
    conn: Mutex<Connection>,
}

/// `~/.local/share/vitalglass/vitalglass.db`, honouring `$XDG_DATA_HOME`.
pub fn default_path() -> Result<PathBuf, Error> {
    dirs::data_dir()
        .map(|d| d.join("vitalglass").join("vitalglass.db"))
        .ok_or_else(|| Error::Config("could not determine the data directory".into()))
}

impl Store {
    pub fn open_default() -> Result<Self, Error> {
        Self::open(&default_path()?)
    }

    /// Open (creating if needed) and migrate. The directory is created
    /// with mode 0700 and the file set to 0600, since they hold health data.
    pub fn open(path: &Path) -> Result<Self, Error> {
        if let Some(dir) = path.parent() {
            create_private_dir(dir)?;
        }
        let mut conn = Connection::open(path)?;
        make_private(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", true)?;
        migrate(&mut conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// In-memory database for tests.
    pub fn open_in_memory() -> Result<Self, Error> {
        let mut conn = Connection::open_in_memory()?;
        migrate(&mut conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Lock the connection. A panic while holding the lock can't leave
    /// SQLite inconsistent (transactions roll back), so ignore poisoning.
    fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Oldest date fetched for `data_type`, if any backfill has run.
    pub fn backfilled_from(&self, data_type: &str) -> Result<Option<NaiveDate>, Error> {
        let s: Option<Option<String>> = self
            .conn()
            .query_row(
                "SELECT backfilled_from FROM sync_state WHERE data_type = ?1",
                [data_type],
                |r| r.get(0),
            )
            .optional()?;
        Ok(s.flatten().and_then(|s| s.parse().ok()))
    }

    /// Record the outcome of a sync step. `backfilled_from` only moves
    /// further back, never forward.
    pub fn set_sync_state(
        &self,
        data_type: &str,
        status: &str,
        message: Option<&str>,
        backfilled_from: Option<NaiveDate>,
    ) -> Result<(), Error> {
        let now = utc(Utc::now());
        let success = (status == "ok").then(|| now.clone());
        self.conn().execute(
            "INSERT INTO sync_state (data_type, status, message, last_success_utc, backfilled_from, updated_utc)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT (data_type) DO UPDATE SET
                 status = excluded.status,
                 message = excluded.message,
                 last_success_utc = COALESCE(excluded.last_success_utc, last_success_utc),
                 backfilled_from = CASE
                     WHEN excluded.backfilled_from IS NULL THEN backfilled_from
                     WHEN backfilled_from IS NULL OR excluded.backfilled_from < backfilled_from
                         THEN excluded.backfilled_from
                     ELSE backfilled_from END,
                 updated_utc = excluded.updated_utc",
            params![
                data_type,
                status,
                message,
                success,
                backfilled_from.map(|d| d.to_string()),
                now
            ],
        )?;
        Ok(())
    }
}

fn migrate(conn: &mut Connection) -> Result<(), Error> {
    let applied: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(applied as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", i as i64 + 1)?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(unix)]
fn create_private_dir(dir: &Path) -> Result<(), Error> {
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new().recursive(true).mode(0o700).create(dir)?;
    Ok(())
}

#[cfg(not(unix))]
fn create_private_dir(dir: &Path) -> Result<(), Error> {
    std::fs::create_dir_all(dir)?;
    Ok(())
}

/// chmod 600. SQLite creates the `-wal`/`-shm` files with the same mode.
#[cfg(unix)]
fn make_private(file: &Path) -> Result<(), Error> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn make_private(_file: &Path) -> Result<(), Error> {
    Ok(())
}

/// Instants are stored as RFC 3339 UTC strings with second precision.
fn utc(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_and_tracks_backfill() {
        let store = Store::open_in_memory().unwrap();
        let d = |s: &str| s.parse::<NaiveDate>().unwrap();
        assert_eq!(store.backfilled_from("steps").unwrap(), None);

        store
            .set_sync_state("steps", "ok", None, Some(d("2026-09-01")))
            .unwrap();
        // A later, newer date must not move the marker forward.
        store
            .set_sync_state("steps", "ok", None, Some(d("2026-10-01")))
            .unwrap();
        store.set_sync_state("steps", "error", Some("boom"), None).unwrap();
        assert_eq!(store.backfilled_from("steps").unwrap(), Some(d("2026-09-01")));
    }

    #[test]
    fn hides_fitness_app_echoes() {
        let store = Store::open_in_memory().unwrap();
        let insert = |id: &str, app: Option<&str>| {
            store
                .conn()
                .execute(
                    "INSERT INTO exercises (id, start_utc, end_utc, civil_start_date, raw_json, source_app)
                     VALUES (?1, '2026-10-23T05:01:00Z', '2026-10-23T06:30:00Z', '2026-10-23', '{}', ?2)",
                    params![id, app],
                )
                .unwrap();
        };
        insert("fitbit", None);
        insert("echo", Some("com.google.android.apps.fitness"));
        insert("hevy", Some("com.hevy"));
        let mut ids: Vec<String> = store.recent_exercises(10).unwrap().into_iter().map(|e| e.id).collect();
        ids.sort();
        assert_eq!(ids, ["fitbit", "hevy"]);
    }
}
