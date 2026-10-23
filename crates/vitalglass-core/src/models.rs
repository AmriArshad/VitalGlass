//! Typed views of the API's JSON, covering only the fields VitalGlass uses.
//!
//! Field names follow the API (camelCase on the wire). Unknown fields are
//! ignored, so new API fields don't break parsing. Integer fields that the
//! API sends as strings go through [`crate::units::de_opt_i64`].

use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;

use crate::units::de_opt_i64;

/// `google.type.Date`.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct ApiDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl ApiDate {
    pub fn naive(self) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(self.year, self.month, self.day)
    }
}

/// `CivilDateTime`. The time part is ignored: we only need the local date.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct CivilDateTime {
    pub date: ApiDate,
}

// --- dailyRollUp ------------------------------------------------------------

/// One civil day from `dailyRollUp`. Only the field for the requested
/// data type is set.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyRollup {
    pub civil_start_time: CivilDateTime,
    pub steps: Option<CountSum>,
    pub distance: Option<DistanceSum>,
    pub floors: Option<CountSum>,
    pub active_zone_minutes: Option<AzmSum>,
    pub total_calories: Option<KcalSum>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CountSum {
    #[serde(default, deserialize_with = "de_opt_i64")]
    pub count_sum: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistanceSum {
    #[serde(default, deserialize_with = "de_opt_i64")]
    pub millimeters_sum: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AzmSum {
    #[serde(default, deserialize_with = "de_opt_i64")]
    pub sum_in_fat_burn_heart_zone: Option<i64>,
    #[serde(default, deserialize_with = "de_opt_i64")]
    pub sum_in_cardio_heart_zone: Option<i64>,
    #[serde(default, deserialize_with = "de_opt_i64")]
    pub sum_in_peak_heart_zone: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KcalSum {
    pub kcal_sum: Option<f64>,
}

// --- rollUp (physical time buckets) -----------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rollup {
    pub start_time: DateTime<Utc>,
    pub heart_rate: Option<HeartRateRollup>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeartRateRollup {
    pub beats_per_minute_avg: Option<f64>,
    pub beats_per_minute_min: Option<f64>,
    pub beats_per_minute_max: Option<f64>,
}

// --- list (raw data points) -------------------------------------------------

/// A data point from `list`. Exactly one of the data fields is set,
/// matching the data type that was listed.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataPoint {
    /// Resource name; set for identifiable types such as sleep and exercise.
    pub name: Option<String>,
    pub data_source: Option<DataSource>,
    pub daily_resting_heart_rate: Option<DailyRestingHeartRate>,
    pub daily_heart_rate_variability: Option<DailyHeartRateVariability>,
    pub daily_oxygen_saturation: Option<DailyOxygenSaturation>,
    pub daily_respiratory_rate: Option<DailyRespiratoryRate>,
    pub daily_vo2_max: Option<DailyVo2Max>,
    pub sleep: Option<Sleep>,
    pub exercise: Option<Exercise>,
    pub weight: Option<Weight>,
}

/// Where a data point came from.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataSource {
    /// `FITBIT`, `HEALTH_CONNECT`, ...
    pub platform: Option<String>,
    pub application: Option<Application>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Application {
    /// Android package name for Health Connect sources, e.g. `com.hevy`.
    pub package_name: Option<String>,
}

impl DataPoint {
    pub fn source_platform(&self) -> Option<&str> {
        self.data_source.as_ref()?.platform.as_deref()
    }

    pub fn source_app(&self) -> Option<&str> {
        self.data_source.as_ref()?.application.as_ref()?.package_name.as_deref()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyRestingHeartRate {
    pub date: ApiDate,
    #[serde(default, deserialize_with = "de_opt_i64")]
    pub beats_per_minute: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyHeartRateVariability {
    pub date: ApiDate,
    pub average_heart_rate_variability_milliseconds: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyOxygenSaturation {
    pub date: ApiDate,
    pub average_percentage: Option<f64>,
    pub lower_bound_percentage: Option<f64>,
    pub upper_bound_percentage: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyRespiratoryRate {
    pub date: ApiDate,
    pub breaths_per_minute: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyVo2Max {
    pub date: ApiDate,
    pub vo2_max: Option<f64>,
}

/// Start/end of a session (sleep, exercise) with the local UTC offsets.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInterval {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub start_utc_offset: Option<String>,
    pub end_utc_offset: Option<String>,
    pub civil_start_time: Option<CivilDateTime>,
    pub civil_end_time: Option<CivilDateTime>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sleep {
    pub interval: SessionInterval,
    /// `CLASSIC` or `STAGES`.
    pub r#type: Option<String>,
    pub summary: Option<SleepSummary>,
    pub metadata: Option<SleepMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SleepSummary {
    #[serde(default, deserialize_with = "de_opt_i64")]
    pub minutes_asleep: Option<i64>,
    #[serde(default, deserialize_with = "de_opt_i64")]
    pub minutes_awake: Option<i64>,
    #[serde(default, deserialize_with = "de_opt_i64")]
    pub minutes_in_sleep_period: Option<i64>,
    #[serde(default, deserialize_with = "de_opt_i64")]
    pub minutes_to_fall_asleep: Option<i64>,
    #[serde(default)]
    pub stages_summary: Vec<StageSummary>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageSummary {
    /// `AWAKE`, `LIGHT`, `DEEP`, `REM` (stages) or `ASLEEP`, `RESTLESS` (classic).
    pub r#type: String,
    #[serde(default, deserialize_with = "de_opt_i64")]
    pub minutes: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SleepMetadata {
    pub main_sleep: Option<bool>,
    pub nap: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Exercise {
    pub interval: SessionInterval,
    pub exercise_type: Option<String>,
    pub display_name: Option<String>,
    /// Duration excluding pauses, e.g. `"1820s"`.
    pub active_duration: Option<String>,
    pub metrics_summary: Option<MetricsSummary>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricsSummary {
    #[serde(default, deserialize_with = "de_opt_i64")]
    pub average_heart_rate_beats_per_minute: Option<i64>,
    pub calories_kcal: Option<f64>,
    pub distance_millimeters: Option<f64>,
    #[serde(default, deserialize_with = "de_opt_i64")]
    pub steps: Option<i64>,
    #[serde(default, deserialize_with = "de_opt_i64")]
    pub active_zone_minutes: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Weight {
    pub sample_time: SampleTime,
    pub weight_grams: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SampleTime {
    pub physical_time: DateTime<Utc>,
    pub utc_offset: Option<String>,
}

// --- pairedDevices ----------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairedDevice {
    pub name: String,
    /// Product name, e.g. "Pixel Watch 4".
    pub device_version: Option<String>,
    /// `TRACKER` or `SCALE`.
    pub device_type: Option<String>,
    pub battery_level: Option<i32>,
    pub battery_status: Option<String>,
    pub last_sync_time: Option<DateTime<Utc>>,
}
