-- Migration 1: initial schema.
--
-- Conventions:
--   * civil (local calendar) dates are TEXT 'YYYY-MM-DD'
--   * instants are TEXT RFC 3339 in UTC ('2026-10-24T07:15:00Z'), which sort correctly
--   * UTC offsets are INTEGER seconds (NZDT = 46800)
--   * distances are metres (converted from the API's millimetres)

-- One row per day, filled column by column by the dailyRollUp jobs.
CREATE TABLE daily_activity (
    date          TEXT PRIMARY KEY,
    steps         INTEGER,
    distance_m    REAL,
    floors        INTEGER,
    azm_fat_burn  INTEGER,
    azm_cardio    INTEGER,
    azm_peak      INTEGER,
    calories_kcal REAL
);

-- One row per day from the daily-* list types (overnight metrics).
CREATE TABLE daily_vitals (
    date       TEXT PRIMARY KEY,
    resting_hr INTEGER,
    hrv_ms     REAL,
    spo2_avg   REAL,
    spo2_low   REAL,
    spo2_high  REAL,
    resp_rate  REAL,
    vo2max     REAL
);

-- 5-minute heart rate buckets (today and yesterday only).
CREATE TABLE heart_rate_5min (
    start_utc TEXT PRIMARY KEY,
    bpm_avg   REAL,
    bpm_min   REAL,
    bpm_max   REAL
);

CREATE TABLE weight (
    sampled_utc  TEXT PRIMARY KEY,
    utc_offset_s INTEGER,
    civil_date   TEXT NOT NULL,
    grams        REAL NOT NULL
);
CREATE INDEX weight_civil_date ON weight (civil_date);

CREATE TABLE sleep_sessions (
    id                     TEXT PRIMARY KEY,
    start_utc              TEXT NOT NULL,
    end_utc                TEXT NOT NULL,
    start_offset_s         INTEGER,
    end_offset_s           INTEGER,
    -- The night a sleep belongs to is the local date it ended on.
    civil_end_date         TEXT NOT NULL,
    sleep_type             TEXT,
    main_sleep             INTEGER,
    nap                    INTEGER,
    minutes_asleep         INTEGER,
    minutes_awake          INTEGER,
    minutes_in_period      INTEGER,
    minutes_to_fall_asleep INTEGER,
    minutes_deep           INTEGER,
    minutes_light          INTEGER,
    minutes_rem            INTEGER,
    -- Full data point, including stage segments, for later use.
    raw_json               TEXT NOT NULL
);
CREATE INDEX sleep_civil_end_date ON sleep_sessions (civil_end_date);

CREATE TABLE exercises (
    id                TEXT PRIMARY KEY,
    start_utc         TEXT NOT NULL,
    end_utc           TEXT NOT NULL,
    start_offset_s    INTEGER,
    civil_start_date  TEXT NOT NULL,
    exercise_type     TEXT,
    display_name      TEXT,
    active_duration_s INTEGER,
    avg_hr            INTEGER,
    calories_kcal     REAL,
    distance_m        REAL,
    steps             INTEGER,
    azm               INTEGER,
    -- Full data point, including splits and HR zones, for later use.
    raw_json          TEXT NOT NULL
);
CREATE INDEX exercises_civil_start_date ON exercises (civil_start_date);

CREATE TABLE paired_devices (
    name           TEXT PRIMARY KEY,
    product        TEXT,
    device_type    TEXT,
    battery_level  INTEGER,
    battery_status TEXT,
    last_sync_utc  TEXT
);

-- Per data type: last outcome and how far back history has been fetched.
CREATE TABLE sync_state (
    data_type        TEXT PRIMARY KEY,
    -- 'ok', 'permission_denied' or 'error'
    status           TEXT NOT NULL,
    message          TEXT,
    last_success_utc TEXT,
    -- Oldest civil date fetched so far; NULL until the first backfill chunk.
    backfilled_from  TEXT,
    updated_utc      TEXT NOT NULL
);
