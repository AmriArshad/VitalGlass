//! Fetches data from the API into the [`Store`].
//!
//! A sync runs in two passes:
//!
//! 1. **Hot**: paired devices, today's intraday heart rate, and the last
//!    [`HOT_DAYS`] days of every other type. Recent days are always
//!    re-fetched because data keeps changing as the watch syncs.
//! 2. **Backfill**: older history back to [`HISTORY_DAYS`], newest chunk
//!    first. Progress is saved after each chunk in `sync_state`, so an
//!    interrupted backfill resumes where it stopped.
//!
//! Types are fetched one at a time to stay well under the per-user rate
//! limit. A failure on one type (e.g. a scope the user didn't grant) is
//! recorded for that type and the sync carries on; auth and network
//! failures stop the whole sync.

use std::time::Duration;

use chrono::{Days, Local, NaiveDate, TimeZone, Utc};
use serde_json::Value;

use crate::Error;
use crate::api::HealthClient;
use crate::models::{DailyRollup, DataPoint, PairedDevice, Rollup};
use crate::session::Session;
use crate::store::Store;

/// Recent days re-fetched on every sync.
pub const HOT_DAYS: u64 = 14;
/// How far back the backfill goes (the longest chart range).
pub const HISTORY_DAYS: u64 = 90;
/// Intraday heart rate bucket size.
const HEART_RATE_WINDOW: Duration = Duration::from_secs(300);

/// How a data type is fetched and stored.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    /// `dailyRollUp` into `daily_activity`.
    DailyRollUp,
    /// `list` of a `daily-*` type into `daily_vitals`.
    DailyList,
    Sleep,
    Exercise,
    Weight,
}

#[derive(Debug, Clone, Copy)]
struct Job {
    /// Hyphenated API name.
    data_type: &'static str,
    /// Short scope name, as in [`crate::auth::Tokens::missing_scopes`].
    scope: &'static str,
    kind: Kind,
    /// Longest range one request may cover.
    max_days: u64,
}

const ACTIVITY: &str = "activity_and_fitness.readonly";
const METRICS: &str = "health_metrics_and_measurements.readonly";
const SLEEP: &str = "sleep.readonly";

const fn job(data_type: &'static str, scope: &'static str, kind: Kind, max_days: u64) -> Job {
    Job {
        data_type,
        scope,
        kind,
        max_days,
    }
}

/// Everything except intraday heart rate and devices, which are special.
const JOBS: &[Job] = &[
    job("steps", ACTIVITY, Kind::DailyRollUp, 90),
    job("distance", ACTIVITY, Kind::DailyRollUp, 90),
    job("floors", ACTIVITY, Kind::DailyRollUp, 90),
    job("active-zone-minutes", ACTIVITY, Kind::DailyRollUp, 90),
    job("total-calories", ACTIVITY, Kind::DailyRollUp, 14),
    job("daily-resting-heart-rate", METRICS, Kind::DailyList, 90),
    job("daily-heart-rate-variability", METRICS, Kind::DailyList, 90),
    job("daily-oxygen-saturation", METRICS, Kind::DailyList, 90),
    job("daily-respiratory-rate", METRICS, Kind::DailyList, 90),
    job("daily-vo2-max", ACTIVITY, Kind::DailyList, 90),
    job("weight", METRICS, Kind::Weight, 90),
    job("sleep", SLEEP, Kind::Sleep, 90),
    job("exercise", ACTIVITY, Kind::Exercise, 90),
];

const HEART_RATE: Job = job("heart-rate", METRICS, Kind::DailyRollUp, 14);

/// Which passes to run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    HotOnly,
    Full,
}

/// Outcome for one data type.
#[derive(Debug)]
pub struct JobResult {
    pub data_type: &'static str,
    pub status: &'static str,
    pub items: usize,
    pub message: Option<String>,
}

/// Progress callback: receives a short line per step, for a CLI or a UI
/// status bar. `Send` so a sync can run on a background task.
pub type Progress<'a> = &'a mut (dyn FnMut(&str) + Send);

/// Run a sync: the hot pass, then (for [`Mode::Full`]) the backfill.
pub async fn run(
    session: &mut Session,
    store: &Store,
    mode: Mode,
    progress: Progress<'_>,
) -> Result<Vec<JobResult>, Error> {
    let mut results = hot(session, store, progress).await?;
    if mode == Mode::Full {
        results.extend(backfill(session, store, progress).await?);
    }
    Ok(results)
}

/// Devices, today's intraday heart rate and the last [`HOT_DAYS`] days of
/// every type. Fast enough to run on every launch and every 15 minutes.
pub async fn hot(session: &mut Session, store: &Store, progress: Progress<'_>) -> Result<Vec<JobResult>, Error> {
    let today = Local::now().date_naive();
    let hot_start = today - Days::new(HOT_DAYS - 1);
    let tomorrow = today + Days::new(1);
    let mut results = Vec::new();

    // Devices first: "last synced" tells the user how fresh everything is.
    let api = session.api().await?;
    let devices: Vec<PairedDevice> = parse_all(api.paired_devices().await?);
    store.replace_paired_devices(&devices)?;
    progress(&format!("paired devices: {}", devices.len()));

    let r = run_job(session, store, HEART_RATE, today - Days::new(1), tomorrow, progress).await?;
    results.push(r);

    for job in JOBS {
        let r = run_job(session, store, *job, hot_start, tomorrow, progress).await?;
        results.push(r);
    }
    Ok(results)
}

/// History older than the hot window, back to [`HISTORY_DAYS`]. Resumes
/// from `sync_state`, so once complete it makes no requests. Only failed
/// types are returned.
pub async fn backfill(session: &mut Session, store: &Store, progress: Progress<'_>) -> Result<Vec<JobResult>, Error> {
    let today = Local::now().date_naive();
    let hot_start = today - Days::new(HOT_DAYS - 1);
    let history_start = today - Days::new(HISTORY_DAYS - 1);
    let mut results = Vec::new();
    for job in JOBS {
        // Resume from the oldest date already fetched.
        let done_from = store.backfilled_from(job.data_type)?.unwrap_or(hot_start);
        if done_from <= history_start {
            continue;
        }
        let r = run_job(session, store, *job, history_start, done_from, progress).await?;
        if r.status != "ok" {
            results.push(r);
        }
    }
    Ok(results)
}

/// Fetch `[from, to)` for one job in chunks of `job.max_days`, newest
/// first, and record the outcome in `sync_state`.
async fn run_job(
    session: &mut Session,
    store: &Store,
    job: Job,
    from: NaiveDate,
    to: NaiveDate,
    progress: Progress<'_>,
) -> Result<JobResult, Error> {
    let granted = session
        .tokens()
        .is_some_and(|t| t.missing_scopes().iter().all(|s| *s != job.scope));
    if !granted {
        let msg = format!("scope {} not granted", job.scope);
        store.set_sync_state(job.data_type, "permission_denied", Some(&msg), None)?;
        progress(&format!("{}: skipped ({msg})", job.data_type));
        return Ok(result(job, "permission_denied", 0, Some(msg)));
    }

    let mut items = 0;
    let mut chunk_end = to;
    while chunk_end > from {
        let chunk_start = from.max(chunk_end - Days::new(job.max_days));
        let api = session.api().await?;
        match fetch_chunk(&api, store, job, chunk_start, chunk_end).await {
            Ok(n) => {
                items += n;
                store.set_sync_state(job.data_type, "ok", None, Some(chunk_start))?;
            }
            Err(e) if is_fatal(&e) => return Err(e),
            Err(e) => {
                let status = match e {
                    Error::PermissionDenied(_) => "permission_denied",
                    _ => "error",
                };
                let msg = e.to_string();
                store.set_sync_state(job.data_type, status, Some(&msg), None)?;
                progress(&format!("{}: {status}: {msg}", job.data_type));
                return Ok(result(job, status, items, Some(msg)));
            }
        }
        chunk_end = chunk_start;
    }
    progress(&format!("{}: {items} items ({from} to {to})", job.data_type));
    Ok(result(job, "ok", items, None))
}

/// Auth, linking and network problems affect every type, so stop.
fn is_fatal(e: &Error) -> bool {
    matches!(
        e,
        Error::SessionExpired | Error::Unauthorized | Error::AccountNotLinked | Error::Http(_) | Error::Keyring(_)
    )
}

async fn fetch_chunk(
    api: &HealthClient,
    store: &Store,
    job: Job,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<usize, Error> {
    if job.data_type == HEART_RATE.data_type {
        let start = local_midnight_utc(from);
        let end = local_midnight_utc(to).min(Utc::now());
        let raw = api.roll_up(job.data_type, start, end, HEART_RATE_WINDOW).await?;
        let buckets: Vec<Rollup> = parse_all(raw);
        return store.replace_heart_rate(start, local_midnight_utc(to), &buckets);
    }

    match job.kind {
        Kind::DailyRollUp => {
            let points: Vec<DailyRollup> = parse_all(api.daily_roll_up(job.data_type, from, to).await?);
            store.save_daily_rollups(job.data_type, &points)
        }
        Kind::DailyList => {
            let field = job.data_type.replace('-', "_");
            let filter = format!(r#"{field}.date >= "{from}" AND {field}.date < "{to}""#);
            let points: Vec<DataPoint> = parse_all(api.list(job.data_type, &filter, None).await?);
            store.save_daily_vitals(&points)
        }
        Kind::Weight => {
            let filter =
                format!(r#"weight.sample_time.civil_time >= "{from}" AND weight.sample_time.civil_time < "{to}""#);
            let points: Vec<DataPoint> = parse_all(api.list(job.data_type, &filter, None).await?);
            store.replace_weight(from, to, &points)
        }
        Kind::Sleep => {
            // Sleep can only be filtered by end time.
            let filter =
                format!(r#"sleep.interval.civil_end_time >= "{from}" AND sleep.interval.civil_end_time < "{to}""#);
            let points = parse_with_raw(api.list(job.data_type, &filter, Some(25)).await?);
            store.replace_sleep(from, to, &points)
        }
        Kind::Exercise => {
            let filter = format!(
                r#"exercise.interval.civil_start_time >= "{from}" AND exercise.interval.civil_start_time < "{to}""#
            );
            let points = parse_with_raw(api.list(job.data_type, &filter, Some(25)).await?);
            store.replace_exercises(from, to, &points)
        }
    }
}

/// Parse each item, logging and skipping any that don't match the model
/// rather than failing the whole batch.
fn parse_all<T: serde::de::DeserializeOwned>(raw: Vec<Value>) -> Vec<T> {
    raw.into_iter()
        .filter_map(|v| match serde_json::from_value(v) {
            Ok(t) => Some(t),
            Err(e) => {
                log::warn!("skipping unparseable item: {e}");
                None
            }
        })
        .collect()
}

/// Like [`parse_all`] but keeps the raw JSON alongside, for `raw_json` columns.
fn parse_with_raw(raw: Vec<Value>) -> Vec<(DataPoint, Value)> {
    raw.into_iter()
        .filter_map(|v| match serde_json::from_value(v.clone()) {
            Ok(p) => Some((p, v)),
            Err(e) => {
                log::warn!("skipping unparseable item: {e}");
                None
            }
        })
        .collect()
}

/// Midnight at the start of `date` in the system time zone, as UTC.
/// (NZ changes clocks at 2–3 am, but some zones skip midnight itself; fall
/// back to treating it as UTC midnight there rather than panicking.)
fn local_midnight_utc(date: NaiveDate) -> chrono::DateTime<Utc> {
    let midnight = date.and_hms_opt(0, 0, 0).expect("midnight is valid");
    Local
        .from_local_datetime(&midnight)
        .earliest()
        .map(|t| t.with_timezone(&Utc))
        .unwrap_or_else(|| Utc.from_utc_datetime(&midnight))
}

fn result(job: Job, status: &'static str, items: usize, message: Option<String>) -> JobResult {
    JobResult {
        data_type: job.data_type,
        status,
        items,
        message,
    }
}
