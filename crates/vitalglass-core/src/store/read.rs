//! Queries for the dashboard and the CLI summary.
//!
//! Row structs derive `Serialize` so Tauri can hand them straight to the
//! frontend. Dates are `YYYY-MM-DD` strings, instants RFC 3339 UTC.
//! Session queries read the `visible_*` views, which hide duplicate
//! sessions echoed through Health Connect (see migration 2).

use chrono::NaiveDate;
use rusqlite::{Row, params};
use serde::Serialize;

use super::Store;
use crate::Error;

#[derive(Debug, Serialize)]
pub struct DailyActivity {
    pub date: String,
    pub steps: Option<i64>,
    pub distance_m: Option<f64>,
    pub floors: Option<i64>,
    pub azm_fat_burn: Option<i64>,
    pub azm_cardio: Option<i64>,
    pub azm_peak: Option<i64>,
    pub calories_kcal: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct DailyVitals {
    pub date: String,
    pub resting_hr: Option<i64>,
    pub hrv_ms: Option<f64>,
    pub spo2_avg: Option<f64>,
    pub spo2_low: Option<f64>,
    pub spo2_high: Option<f64>,
    pub resp_rate: Option<f64>,
    pub vo2max: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct SleepSession {
    pub id: String,
    pub start_utc: String,
    pub end_utc: String,
    pub civil_end_date: String,
    pub sleep_type: Option<String>,
    pub main_sleep: Option<bool>,
    pub minutes_asleep: Option<i64>,
    pub minutes_awake: Option<i64>,
    pub minutes_in_period: Option<i64>,
    pub minutes_deep: Option<i64>,
    pub minutes_light: Option<i64>,
    pub minutes_rem: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ExerciseRow {
    pub id: String,
    pub start_utc: String,
    pub end_utc: String,
    pub start_offset_s: Option<i64>,
    pub exercise_type: Option<String>,
    pub display_name: Option<String>,
    /// `FITBIT` or `HEALTH_CONNECT`.
    pub source_platform: Option<String>,
    /// Health Connect app, e.g. `com.hevy`.
    pub source_app: Option<String>,
    pub active_duration_s: Option<i64>,
    pub avg_hr: Option<i64>,
    pub calories_kcal: Option<f64>,
    pub distance_m: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct HeartRateBucket {
    pub start_utc: String,
    pub bpm_avg: f64,
    pub bpm_min: Option<f64>,
    pub bpm_max: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct WeightSample {
    pub sampled_utc: String,
    pub civil_date: String,
    pub grams: f64,
}

#[derive(Debug, Serialize)]
pub struct Device {
    pub name: String,
    pub product: Option<String>,
    pub device_type: Option<String>,
    pub battery_level: Option<i64>,
    pub battery_status: Option<String>,
    pub last_sync_utc: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SyncState {
    pub data_type: String,
    pub status: String,
    pub message: Option<String>,
    pub last_success_utc: Option<String>,
    pub backfilled_from: Option<String>,
}

impl Store {
    /// Days in `[from, to)`, oldest first.
    pub fn daily_activity(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<DailyActivity>, Error> {
        self.query(
            "SELECT date, steps, distance_m, floors, azm_fat_burn, azm_cardio, azm_peak, calories_kcal
             FROM daily_activity WHERE date >= ?1 AND date < ?2 ORDER BY date",
            params![from.to_string(), to.to_string()],
            |r| {
                Ok(DailyActivity {
                    date: r.get(0)?,
                    steps: r.get(1)?,
                    distance_m: r.get(2)?,
                    floors: r.get(3)?,
                    azm_fat_burn: r.get(4)?,
                    azm_cardio: r.get(5)?,
                    azm_peak: r.get(6)?,
                    calories_kcal: r.get(7)?,
                })
            },
        )
    }

    /// Days in `[from, to)`, oldest first.
    pub fn daily_vitals(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<DailyVitals>, Error> {
        self.query(
            "SELECT date, resting_hr, hrv_ms, spo2_avg, spo2_low, spo2_high, resp_rate, vo2max
             FROM daily_vitals WHERE date >= ?1 AND date < ?2 ORDER BY date",
            params![from.to_string(), to.to_string()],
            |r| {
                Ok(DailyVitals {
                    date: r.get(0)?,
                    resting_hr: r.get(1)?,
                    hrv_ms: r.get(2)?,
                    spo2_avg: r.get(3)?,
                    spo2_low: r.get(4)?,
                    spo2_high: r.get(5)?,
                    resp_rate: r.get(6)?,
                    vo2max: r.get(7)?,
                })
            },
        )
    }

    /// Sleep sessions that ended on `[from, to)`, oldest first.
    pub fn sleep_sessions(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<SleepSession>, Error> {
        self.query(
            "SELECT id, start_utc, end_utc, civil_end_date, sleep_type, main_sleep, minutes_asleep,
                    minutes_awake, minutes_in_period, minutes_deep, minutes_light, minutes_rem
             FROM visible_sleep_sessions WHERE civil_end_date >= ?1 AND civil_end_date < ?2 ORDER BY end_utc",
            params![from.to_string(), to.to_string()],
            sleep_row,
        )
    }

    /// The most recent main sleep (last night, usually).
    pub fn latest_main_sleep(&self) -> Result<Option<SleepSession>, Error> {
        Ok(self
            .query(
                "SELECT id, start_utc, end_utc, civil_end_date, sleep_type, main_sleep, minutes_asleep,
                        minutes_awake, minutes_in_period, minutes_deep, minutes_light, minutes_rem
                 FROM visible_sleep_sessions WHERE main_sleep = 1 ORDER BY end_utc DESC LIMIT 1",
                [],
                sleep_row,
            )?
            .pop())
    }

    /// Newest exercises first.
    pub fn recent_exercises(&self, limit: u32) -> Result<Vec<ExerciseRow>, Error> {
        self.query(
            "SELECT id, start_utc, end_utc, start_offset_s, exercise_type, display_name,
                    source_platform, source_app, active_duration_s, avg_hr, calories_kcal, distance_m
             FROM visible_exercises ORDER BY start_utc DESC LIMIT ?1",
            [limit],
            |r| {
                Ok(ExerciseRow {
                    id: r.get(0)?,
                    start_utc: r.get(1)?,
                    end_utc: r.get(2)?,
                    start_offset_s: r.get(3)?,
                    exercise_type: r.get(4)?,
                    display_name: r.get(5)?,
                    source_platform: r.get(6)?,
                    source_app: r.get(7)?,
                    active_duration_s: r.get(8)?,
                    avg_hr: r.get(9)?,
                    calories_kcal: r.get(10)?,
                    distance_m: r.get(11)?,
                })
            },
        )
    }

    /// Heart rate buckets with `start_utc` in `[start, end)` (RFC 3339 UTC).
    pub fn heart_rate(&self, start: &str, end: &str) -> Result<Vec<HeartRateBucket>, Error> {
        self.query(
            "SELECT start_utc, bpm_avg, bpm_min, bpm_max FROM heart_rate_5min
             WHERE start_utc >= ?1 AND start_utc < ?2 ORDER BY start_utc",
            params![start, end],
            |r| {
                Ok(HeartRateBucket {
                    start_utc: r.get(0)?,
                    bpm_avg: r.get(1)?,
                    bpm_min: r.get(2)?,
                    bpm_max: r.get(3)?,
                })
            },
        )
    }

    /// Weight samples on local dates `[from, to)`, oldest first.
    pub fn weight(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<WeightSample>, Error> {
        self.query(
            "SELECT sampled_utc, civil_date, grams FROM weight
             WHERE civil_date >= ?1 AND civil_date < ?2 ORDER BY sampled_utc",
            params![from.to_string(), to.to_string()],
            |r| {
                Ok(WeightSample {
                    sampled_utc: r.get(0)?,
                    civil_date: r.get(1)?,
                    grams: r.get(2)?,
                })
            },
        )
    }

    pub fn devices(&self) -> Result<Vec<Device>, Error> {
        self.query(
            "SELECT name, product, device_type, battery_level, battery_status, last_sync_utc
             FROM paired_devices ORDER BY last_sync_utc DESC",
            [],
            |r| {
                Ok(Device {
                    name: r.get(0)?,
                    product: r.get(1)?,
                    device_type: r.get(2)?,
                    battery_level: r.get(3)?,
                    battery_status: r.get(4)?,
                    last_sync_utc: r.get(5)?,
                })
            },
        )
    }

    pub fn sync_states(&self) -> Result<Vec<SyncState>, Error> {
        self.query(
            "SELECT data_type, status, message, last_success_utc, backfilled_from
             FROM sync_state ORDER BY data_type",
            [],
            |r| {
                Ok(SyncState {
                    data_type: r.get(0)?,
                    status: r.get(1)?,
                    message: r.get(2)?,
                    last_success_utc: r.get(3)?,
                    backfilled_from: r.get(4)?,
                })
            },
        )
    }

    fn query<T>(
        &self,
        sql: &str,
        params: impl rusqlite::Params,
        map: impl FnMut(&Row) -> rusqlite::Result<T>,
    ) -> Result<Vec<T>, Error> {
        let mut stmt = self.conn.prepare_cached(sql)?;
        let rows = stmt.query_map(params, map)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

fn sleep_row(r: &Row) -> rusqlite::Result<SleepSession> {
    Ok(SleepSession {
        id: r.get(0)?,
        start_utc: r.get(1)?,
        end_utc: r.get(2)?,
        civil_end_date: r.get(3)?,
        sleep_type: r.get(4)?,
        main_sleep: r.get(5)?,
        minutes_asleep: r.get(6)?,
        minutes_awake: r.get(7)?,
        minutes_in_period: r.get(8)?,
        minutes_deep: r.get(9)?,
        minutes_light: r.get(10)?,
        minutes_rem: r.get(11)?,
    })
}
