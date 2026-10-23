-- Migration 2: record where each sleep and exercise session came from.
--
-- Google Health merges several sources. Health Connect re-publishes some
-- Fitbit sessions through Google's fitness app
-- (com.google.android.apps.fitness), which shows up as duplicate walks
-- and short "naps" overlapping real sleeps. Other Health Connect apps
-- (e.g. Hevy) are genuine. Storing the source lets queries hide the
-- echoes without losing anything.

ALTER TABLE sleep_sessions ADD COLUMN source_platform TEXT;
ALTER TABLE sleep_sessions ADD COLUMN source_app TEXT;
ALTER TABLE exercises ADD COLUMN source_platform TEXT;
ALTER TABLE exercises ADD COLUMN source_app TEXT;

-- Fill the new columns for rows synced before this migration.
UPDATE sleep_sessions SET
    source_platform = json_extract(raw_json, '$.dataSource.platform'),
    source_app = json_extract(raw_json, '$.dataSource.application.packageName');
UPDATE exercises SET
    source_platform = json_extract(raw_json, '$.dataSource.platform'),
    source_app = json_extract(raw_json, '$.dataSource.application.packageName');

-- What the dashboard reads: everything except the fitness app's echoes.
-- To hide another source, change these views in a new migration.
CREATE VIEW visible_sleep_sessions AS
    SELECT * FROM sleep_sessions
    WHERE COALESCE(source_app, '') <> 'com.google.android.apps.fitness';

CREATE VIEW visible_exercises AS
    SELECT * FROM exercises
    WHERE COALESCE(source_app, '') <> 'com.google.android.apps.fitness';
