// Types and calls for the dashboard and sync commands in
// src-tauri/src/dashboard.rs and src-tauri/src/sync.rs.
// Keep the types in sync with the Rust structs (vitalglass-core store/read.rs).

import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { AccountStatus } from './account'

export interface DailyActivity {
  date: string
  steps: number | null
  distance_m: number | null
  floors: number | null
  azm_fat_burn: number | null
  azm_cardio: number | null
  azm_peak: number | null
  calories_kcal: number | null
}

export interface DailyVitals {
  date: string
  resting_hr: number | null
  hrv_ms: number | null
  spo2_avg: number | null
  spo2_low: number | null
  spo2_high: number | null
  resp_rate: number | null
  vo2max: number | null
}

export interface SleepSession {
  id: string
  start_utc: string
  end_utc: string
  civil_end_date: string
  sleep_type: string | null
  main_sleep: boolean | null
  minutes_asleep: number | null
  minutes_awake: number | null
  minutes_in_period: number | null
  minutes_deep: number | null
  minutes_light: number | null
  minutes_rem: number | null
}

export interface Workout {
  id: string
  start_utc: string
  end_utc: string
  start_offset_s: number | null
  exercise_type: string | null
  display_name: string | null
  source_platform: string | null
  source_app: string | null
  active_duration_s: number | null
  avg_hr: number | null
  calories_kcal: number | null
  distance_m: number | null
}

export interface HeartRateBucket {
  start_utc: string
  bpm_avg: number
  bpm_min: number | null
  bpm_max: number | null
}

export interface StepsBucket {
  start_utc: string
  steps: number
}

export interface SleepStageSegment {
  start_utc: string
  end_utc: string
  /** AWAKE, LIGHT, DEEP, REM (stages) or ASLEEP, RESTLESS (classic). */
  stage: string
}

export interface WeightSample {
  sampled_utc: string
  civil_date: string
  grams: number
}

export interface Device {
  name: string
  product: string | null
  device_type: string | null
  battery_level: number | null
  battery_status: string | null
  last_sync_utc: string | null
}

export interface SyncState {
  data_type: string
  status: 'ok' | 'permission_denied' | 'error'
  message: string | null
  last_success_utc: string | null
  backfilled_from: string | null
}

export interface Dashboard {
  today: string
  range_start: string
  range_days: number
  activity: DailyActivity[]
  vitals: DailyVitals[]
  sleep: SleepSession[]
  last_sleep: SleepSession | null
  heart_rate_today: HeartRateBucket[]
  steps_today: StepsBucket[]
  last_sleep_stages: SleepStageSegment[]
  /** Last 30 days of vitals and sleep, whatever the range, for averages. */
  baseline_vitals: DailyVitals[]
  baseline_sleep: SleepSession[]
  workouts: Workout[]
  weight: WeightSample[]
  devices: Device[]
  sync: SyncState[]
  syncing: boolean
}

export interface SyncFinished {
  ok: boolean
  account: AccountStatus | null
  failures: { data_type: string; status: string; message: string | null }[]
}

export const getDashboard = (rangeDays: number) => invoke<Dashboard>('dashboard', { rangeDays })

/** Exit the app (closing the window only hides it to the tray). */
export const quitApp = () => invoke('quit')

/** Start a sync. Resolves to false if one is already running. */
export const syncNow = () => invoke<boolean>('sync_now')

export const onSyncProgress = (cb: (line: string) => void): Promise<UnlistenFn> =>
  listen<string>('sync-progress', (e) => cb(e.payload))

export const onDataChanged = (cb: () => void): Promise<UnlistenFn> => listen('data-changed', () => cb())

export const onSyncFinished = (cb: (f: SyncFinished) => void): Promise<UnlistenFn> =>
  listen<SyncFinished>('sync-finished', (e) => cb(e.payload))

/**
 * Sync state for the data types behind a card: 'permission_denied' if any of
 * them lacks a granted scope, otherwise 'ok'.
 */
export function cardPermission(sync: SyncState[], dataTypes: string[]): 'ok' | 'permission_denied' {
  return sync.some((s) => dataTypes.includes(s.data_type) && s.status === 'permission_denied')
    ? 'permission_denied'
    : 'ok'
}
