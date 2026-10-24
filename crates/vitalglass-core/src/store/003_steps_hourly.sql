-- Migration 3: hourly steps for today's intraday view (today and yesterday
-- only, like heart_rate_5min).

CREATE TABLE steps_hourly (
    start_utc TEXT PRIMARY KEY,
    steps     INTEGER NOT NULL
);
