//! Writes from the sync layer.
//!
//! Daily tables are upserted column by column, because several data types
//! fill one row. Time-ranged tables (sleep, exercise, weight, heart rate)
//! use delete-then-insert over the fetched range, so something deleted in
//! the Google Health app also disappears here on the next sync.

use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::params;
use rusqlite::types::Value as Sql;
use serde_json::Value;

use super::{Store, utc};
use crate::Error;
use crate::models::{DailyRollup, DataPoint, PairedDevice, Rollup};
use crate::units::{civil_date, duration_secs, mm_to_m};

impl Store {
    /// Store `dailyRollUp` results for one data type into `daily_activity`.
    pub fn save_daily_rollups(&self, data_type: &str, points: &[DailyRollup]) -> Result<usize, Error> {
        let conn = self.conn();
        let tx = conn.unchecked_transaction()?;
        let mut n = 0;
        for p in points {
            let Some(date) = p.civil_start_time.date.naive() else {
                continue;
            };
            let cols: Vec<(&str, Sql)> = match data_type {
                "steps" => vec![("steps", int(p.steps.as_ref().and_then(|v| v.count_sum)))],
                "floors" => vec![("floors", int(p.floors.as_ref().and_then(|v| v.count_sum)))],
                "distance" => vec![(
                    "distance_m",
                    real(
                        p.distance
                            .as_ref()
                            .and_then(|v| v.millimeters_sum)
                            .map(|mm| mm_to_m(mm as f64)),
                    ),
                )],
                "total-calories" => vec![(
                    "calories_kcal",
                    real(p.total_calories.as_ref().and_then(|v| v.kcal_sum)),
                )],
                "active-zone-minutes" => {
                    let azm = p.active_zone_minutes.as_ref();
                    vec![
                        ("azm_fat_burn", int(azm.and_then(|v| v.sum_in_fat_burn_heart_zone))),
                        ("azm_cardio", int(azm.and_then(|v| v.sum_in_cardio_heart_zone))),
                        ("azm_peak", int(azm.and_then(|v| v.sum_in_peak_heart_zone))),
                    ]
                }
                other => return Err(Error::Config(format!("no daily_activity column for {other}"))),
            };
            upsert_day(&tx, "daily_activity", date, &cols)?;
            n += 1;
        }
        tx.commit()?;
        Ok(n)
    }

    /// Store points from the `daily-*` list types into `daily_vitals`.
    pub fn save_daily_vitals(&self, points: &[DataPoint]) -> Result<usize, Error> {
        let conn = self.conn();
        let tx = conn.unchecked_transaction()?;
        let mut n = 0;
        for p in points {
            let (date, cols) = if let Some(v) = &p.daily_resting_heart_rate {
                (v.date, vec![("resting_hr", int(v.beats_per_minute))])
            } else if let Some(v) = &p.daily_heart_rate_variability {
                (
                    v.date,
                    vec![("hrv_ms", real(v.average_heart_rate_variability_milliseconds))],
                )
            } else if let Some(v) = &p.daily_oxygen_saturation {
                (
                    v.date,
                    vec![
                        ("spo2_avg", real(v.average_percentage)),
                        ("spo2_low", real(v.lower_bound_percentage)),
                        ("spo2_high", real(v.upper_bound_percentage)),
                    ],
                )
            } else if let Some(v) = &p.daily_respiratory_rate {
                (v.date, vec![("resp_rate", real(v.breaths_per_minute))])
            } else if let Some(v) = &p.daily_vo2_max {
                (v.date, vec![("vo2max", real(v.vo2_max))])
            } else {
                continue;
            };
            let Some(date) = date.naive() else { continue };
            upsert_day(&tx, "daily_vitals", date, &cols)?;
            n += 1;
        }
        tx.commit()?;
        Ok(n)
    }

    /// Replace sleep sessions that ended on `[from, to)` with `points`.
    /// Each item pairs the parsed point with its raw JSON.
    pub fn replace_sleep(&self, from: NaiveDate, to: NaiveDate, points: &[(DataPoint, Value)]) -> Result<usize, Error> {
        let conn = self.conn();
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM sleep_sessions WHERE civil_end_date >= ?1 AND civil_end_date < ?2",
            params![from.to_string(), to.to_string()],
        )?;
        let mut n = 0;
        for (p, raw) in points {
            let Some(s) = &p.sleep else { continue };
            let i = &s.interval;
            let civil_end = i
                .civil_end_time
                .and_then(|c| c.date.naive())
                .unwrap_or_else(|| civil_date(i.end_time, i.end_utc_offset.as_deref()));
            let summary = s.summary.as_ref();
            let stage = |name: &str| {
                summary
                    .and_then(|s| s.stages_summary.iter().find(|st| st.r#type == name))
                    .and_then(|st| st.minutes)
            };
            let meta = s.metadata.as_ref();
            tx.execute(
                "INSERT OR REPLACE INTO sleep_sessions (
                     id, start_utc, end_utc, start_offset_s, end_offset_s, civil_end_date,
                     sleep_type, main_sleep, nap, minutes_asleep, minutes_awake, minutes_in_period,
                     minutes_to_fall_asleep, minutes_deep, minutes_light, minutes_rem, raw_json,
                     source_platform, source_app)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
                params![
                    p.name.clone().unwrap_or_else(|| format!("sleep/{}", utc(i.start_time))),
                    utc(i.start_time),
                    utc(i.end_time),
                    i.start_utc_offset.as_deref().and_then(duration_secs),
                    i.end_utc_offset.as_deref().and_then(duration_secs),
                    civil_end.to_string(),
                    s.r#type,
                    meta.and_then(|m| m.main_sleep),
                    meta.and_then(|m| m.nap),
                    summary.and_then(|s| s.minutes_asleep),
                    summary.and_then(|s| s.minutes_awake),
                    summary.and_then(|s| s.minutes_in_sleep_period),
                    summary.and_then(|s| s.minutes_to_fall_asleep),
                    stage("DEEP"),
                    stage("LIGHT"),
                    stage("REM"),
                    raw.to_string(),
                    p.source_platform(),
                    p.source_app(),
                ],
            )?;
            n += 1;
        }
        tx.commit()?;
        Ok(n)
    }

    /// Replace exercises that started on `[from, to)` with `points`.
    pub fn replace_exercises(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        points: &[(DataPoint, Value)],
    ) -> Result<usize, Error> {
        let conn = self.conn();
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM exercises WHERE civil_start_date >= ?1 AND civil_start_date < ?2",
            params![from.to_string(), to.to_string()],
        )?;
        let mut n = 0;
        for (p, raw) in points {
            let Some(e) = &p.exercise else { continue };
            let i = &e.interval;
            let civil_start = i
                .civil_start_time
                .and_then(|c| c.date.naive())
                .unwrap_or_else(|| civil_date(i.start_time, i.start_utc_offset.as_deref()));
            let m = e.metrics_summary.as_ref();
            tx.execute(
                "INSERT OR REPLACE INTO exercises (
                     id, start_utc, end_utc, start_offset_s, civil_start_date, exercise_type,
                     display_name, active_duration_s, avg_hr, calories_kcal, distance_m, steps, azm, raw_json,
                     source_platform, source_app)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                params![
                    p.name
                        .clone()
                        .unwrap_or_else(|| format!("exercise/{}", utc(i.start_time))),
                    utc(i.start_time),
                    utc(i.end_time),
                    i.start_utc_offset.as_deref().and_then(duration_secs),
                    civil_start.to_string(),
                    e.exercise_type,
                    e.display_name,
                    e.active_duration.as_deref().and_then(duration_secs),
                    m.and_then(|m| m.average_heart_rate_beats_per_minute),
                    m.and_then(|m| m.calories_kcal),
                    m.and_then(|m| m.distance_millimeters).map(mm_to_m),
                    m.and_then(|m| m.steps),
                    m.and_then(|m| m.active_zone_minutes),
                    raw.to_string(),
                    p.source_platform(),
                    p.source_app(),
                ],
            )?;
            n += 1;
        }
        tx.commit()?;
        Ok(n)
    }

    /// Replace weight samples taken on local dates `[from, to)`.
    pub fn replace_weight(&self, from: NaiveDate, to: NaiveDate, points: &[DataPoint]) -> Result<usize, Error> {
        let conn = self.conn();
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM weight WHERE civil_date >= ?1 AND civil_date < ?2",
            params![from.to_string(), to.to_string()],
        )?;
        let mut n = 0;
        for p in points {
            let Some(w) = &p.weight else { continue };
            let offset = w.sample_time.utc_offset.as_deref().and_then(duration_secs);
            tx.execute(
                "INSERT OR REPLACE INTO weight (sampled_utc, utc_offset_s, civil_date, grams) VALUES (?1, ?2, ?3, ?4)",
                params![
                    utc(w.sample_time.physical_time),
                    offset,
                    civil_date(w.sample_time.physical_time, w.sample_time.utc_offset.as_deref()).to_string(),
                    w.weight_grams
                ],
            )?;
            n += 1;
        }
        tx.commit()?;
        Ok(n)
    }

    /// Replace heart rate buckets starting in `[start, end)`.
    pub fn replace_heart_rate(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        buckets: &[Rollup],
    ) -> Result<usize, Error> {
        let conn = self.conn();
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM heart_rate_5min WHERE start_utc >= ?1 AND start_utc < ?2",
            params![utc(start), utc(end)],
        )?;
        let mut n = 0;
        for b in buckets {
            // Windows with no readings (watch off) come back without a value.
            let Some(hr) = &b.heart_rate else { continue };
            if hr.beats_per_minute_avg.is_none() {
                continue;
            }
            tx.execute(
                "INSERT OR REPLACE INTO heart_rate_5min (start_utc, bpm_avg, bpm_min, bpm_max) VALUES (?1, ?2, ?3, ?4)",
                params![
                    utc(b.start_time),
                    hr.beats_per_minute_avg,
                    hr.beats_per_minute_min,
                    hr.beats_per_minute_max
                ],
            )?;
            n += 1;
        }
        tx.commit()?;
        Ok(n)
    }

    /// Replace the paired device list.
    pub fn replace_paired_devices(&self, devices: &[PairedDevice]) -> Result<usize, Error> {
        let conn = self.conn();
        let tx = conn.unchecked_transaction()?;
        tx.execute("DELETE FROM paired_devices", [])?;
        for d in devices {
            tx.execute(
                "INSERT INTO paired_devices (name, product, device_type, battery_level, battery_status, last_sync_utc)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    d.name,
                    d.device_version,
                    d.device_type,
                    d.battery_level,
                    d.battery_status,
                    d.last_sync_time.map(utc)
                ],
            )?;
        }
        tx.commit()?;
        Ok(devices.len())
    }
}

/// `INSERT ... ON CONFLICT(date) DO UPDATE` for just the given columns,
/// leaving the row's other columns alone. Table and column names come
/// from the code above, never from API data, so formatting them in is safe.
fn upsert_day(tx: &rusqlite::Transaction, table: &str, date: NaiveDate, cols: &[(&str, Sql)]) -> Result<(), Error> {
    let names: Vec<&str> = cols.iter().map(|(c, _)| *c).collect();
    let placeholders: Vec<String> = (2..=cols.len() + 1).map(|i| format!("?{i}")).collect();
    let updates: Vec<String> = names.iter().map(|c| format!("{c} = excluded.{c}")).collect();
    let sql = format!(
        "INSERT INTO {table} (date, {}) VALUES (?1, {}) ON CONFLICT (date) DO UPDATE SET {}",
        names.join(", "),
        placeholders.join(", "),
        updates.join(", ")
    );
    let mut values: Vec<Sql> = vec![Sql::Text(date.to_string())];
    values.extend(cols.iter().map(|(_, v)| v.clone()));
    tx.execute(&sql, rusqlite::params_from_iter(values))?;
    Ok(())
}

fn int(v: Option<i64>) -> Sql {
    v.map_or(Sql::Null, Sql::Integer)
}

fn real(v: Option<f64>) -> Sql {
    v.map_or(Sql::Null, Sql::Real)
}
