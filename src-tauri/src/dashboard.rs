//! The `dashboard` command: everything the dashboard shows, read from the
//! local store in one call. It never touches the network, so it's instant
//! and works offline.

use chrono::{Days, Local, NaiveDate, SecondsFormat, TimeZone, Utc};
use serde::Serialize;
use tauri::{AppHandle, Manager};
use vitalglass_core::store::{
    DailyActivity, DailyVitals, Device, ExerciseRow, HeartRateBucket, SleepSession, SleepStageSegment, StepsBucket,
    Store, SyncState, WeightSample,
};

use crate::state::AppState;

/// Days averaged for the "vs your average" comparison.
const BASELINE_DAYS: u64 = 30;

/// How many workouts the list shows.
const WORKOUT_LIMIT: u32 = 10;

#[derive(Debug, Serialize)]
pub struct Dashboard {
    /// Local date the data was read for, `YYYY-MM-DD`.
    pub today: String,
    /// First day of the trend range.
    pub range_start: String,
    pub range_days: u32,
    /// Daily rows in the range, oldest first. Days with no data are absent.
    pub activity: Vec<DailyActivity>,
    pub vitals: Vec<DailyVitals>,
    /// Sleep sessions that ended in the range (main sleeps and naps).
    pub sleep: Vec<SleepSession>,
    pub last_sleep: Option<SleepSession>,
    /// 5-minute buckets since local midnight.
    pub heart_rate_today: Vec<HeartRateBucket>,
    /// Hourly steps since local midnight.
    pub steps_today: Vec<StepsBucket>,
    /// Stage timeline of `last_sleep`.
    pub last_sleep_stages: Vec<SleepStageSegment>,
    /// The last 30 days of vitals regardless of range, for "vs your average".
    pub baseline_vitals: Vec<DailyVitals>,
    /// Sleep sessions over the same 30 days, for the sleep duration average.
    pub baseline_sleep: Vec<SleepSession>,
    pub workouts: Vec<ExerciseRow>,
    /// Weight in the range; empty if the user doesn't log weight.
    pub weight: Vec<WeightSample>,
    pub devices: Vec<Device>,
    pub sync: Vec<SyncState>,
    pub syncing: bool,
}

/// Read the dashboard for the last `range_days` days (7, 30 or 90).
#[tauri::command]
pub async fn dashboard(app: AppHandle, range_days: u32) -> Result<Dashboard, String> {
    let range_days = range_days.clamp(1, vitalglass_core::sync::HISTORY_DAYS as u32);
    // SQLite calls block, so run them off the async runtime.
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let store = state.store.as_ref().map_err(Clone::clone)?;
        let syncing = state.syncing.load(std::sync::atomic::Ordering::SeqCst);
        read(store, range_days, syncing).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

fn read(store: &Store, range_days: u32, syncing: bool) -> Result<Dashboard, vitalglass_core::Error> {
    let today = Local::now().date_naive();
    let tomorrow = today + Days::new(1);
    let start = today - Days::new(u64::from(range_days) - 1);
    let last_sleep = store.latest_main_sleep()?;
    let last_sleep_stages = match &last_sleep {
        Some(s) => store.sleep_stages(&s.id)?,
        None => Vec::new(),
    };
    let (midnight, next_midnight) = (local_midnight_utc(today), local_midnight_utc(tomorrow));
    Ok(Dashboard {
        today: today.to_string(),
        range_start: start.to_string(),
        range_days,
        activity: store.daily_activity(start, tomorrow)?,
        vitals: store.daily_vitals(start, tomorrow)?,
        sleep: store.sleep_sessions(start, tomorrow)?,
        last_sleep,
        last_sleep_stages,
        heart_rate_today: store.heart_rate(&midnight, &next_midnight)?,
        steps_today: store.steps_hourly(&midnight, &next_midnight)?,
        baseline_vitals: store.daily_vitals(today - Days::new(BASELINE_DAYS), today)?,
        baseline_sleep: store.sleep_sessions(today - Days::new(BASELINE_DAYS), today)?,
        workouts: store.recent_exercises(WORKOUT_LIMIT)?,
        weight: store.weight(start, tomorrow)?,
        devices: store.devices()?,
        sync: store.sync_states()?,
        syncing,
    })
}

/// Local midnight as an RFC 3339 UTC string, matching the store's format.
fn local_midnight_utc(date: NaiveDate) -> String {
    let midnight = date.and_hms_opt(0, 0, 0).expect("midnight is valid");
    Local
        .from_local_datetime(&midnight)
        .earliest()
        .map(|t| t.with_timezone(&Utc))
        .unwrap_or_else(|| Utc.from_utc_datetime(&midnight))
        .to_rfc3339_opts(SecondsFormat::Secs, true)
}
