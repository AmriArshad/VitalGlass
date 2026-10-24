// Dev-only mock backend: open http://localhost:5173/?mock in a normal
// browser to run the UI without Tauri, Google or real health data.
// Used for design work and README screenshots. Never bundled in release
// builds (main.ts only imports it when import.meta.env.DEV).
//
// All numbers are generated from a fixed seed, so screenshots are stable.

import { emit } from '@tauri-apps/api/event'
import { mockIPC } from '@tauri-apps/api/mocks'
import { theme } from './lib/theme.svelte'
import type { Dashboard, DailyActivity, DailyVitals, HeartRateBucket, SleepSession, Workout } from './lib/dashboard'

const HISTORY = 90

// ?mock&theme=breeze-light&range=7 preselects a theme and trend range,
// so screenshots can be scripted.
const params = new URLSearchParams(location.search)
// The theme store has already read its saved preference by now, so set it
// directly; the range is read later, when the dashboard mounts.
if (params.get('theme')) theme.setPreference(params.get('theme')!)
if (params.get('range')) localStorage.setItem('vitalglass.range', params.get('range')!)

/** Small deterministic PRNG (mulberry32). */
function rng(seed: number) {
  return () => {
    seed = (seed + 0x6d2b79f5) | 0
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed)
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

const civil = (d: Date) =>
  `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
const utc = (d: Date) => d.toISOString().replace(/\.\d{3}Z$/, 'Z')

function daysAgo(n: number, hour = 0, minute = 0): Date {
  const d = new Date()
  d.setHours(hour, minute, 0, 0)
  d.setDate(d.getDate() - n)
  return d
}

function build(rangeDays: number): Dashboard {
  const r = rng(42)
  const around = (mid: number, spread: number) => mid + (r() - 0.5) * 2 * spread
  const now = new Date()
  const dayFraction = (now.getHours() * 60 + now.getMinutes()) / 1440

  const activity: DailyActivity[] = []
  const vitals: DailyVitals[] = []
  const sleep: SleepSession[] = []
  for (let n = HISTORY - 1; n >= 0; n--) {
    const date = civil(daysAgo(n))
    const scale = n === 0 ? dayFraction : 1
    const steps = Math.round(around(10500, 3500) * scale)
    const fat = Math.round(around(20, 18) * scale)
    const cardio = r() > 0.6 ? Math.round(around(14, 12) * scale) : 0
    activity.push({
      date,
      steps,
      distance_m: steps * 0.72,
      floors: null,
      azm_fat_burn: Math.max(fat, 0),
      azm_cardio: Math.max(cardio, 0),
      azm_peak: r() > 0.9 ? 4 : 0,
      calories_kcal: Math.round(around(2750, 250) * (n === 0 ? 0.35 + dayFraction * 0.65 : 1)),
    })
    vitals.push({
      date,
      resting_hr: Math.round(around(60, 2.4)),
      hrv_ms: around(60, 9),
      spo2_avg: around(96, 1),
      spo2_low: 93,
      spo2_high: 98,
      resp_rate: around(17.3, 0.6),
      vo2max: null,
    })
    const asleep = Math.round(around(450, 60))
    const deep = Math.round(asleep * around(0.15, 0.04))
    const rem = Math.round(asleep * around(0.2, 0.05))
    const awake = Math.round(around(40, 15))
    const end = daysAgo(n, 7, Math.round(around(15, 40)))
    const start = new Date(end.getTime() - (asleep + awake) * 60_000)
    sleep.push({
      id: `sleep-${n}`,
      start_utc: utc(start),
      end_utc: utc(end),
      civil_end_date: date,
      sleep_type: 'STAGES',
      main_sleep: true,
      minutes_asleep: asleep,
      minutes_awake: awake,
      minutes_in_period: asleep + awake,
      minutes_deep: deep,
      minutes_light: asleep - deep - rem,
      minutes_rem: rem,
    })
  }

  // Heart rate since midnight: sleep, waking, a midday walk.
  const heart_rate_today: HeartRateBucket[] = []
  const midnight = daysAgo(0)
  for (let t = midnight.getTime(); t < now.getTime(); t += 5 * 60_000) {
    const hour = new Date(t).getHours() + new Date(t).getMinutes() / 60
    const base = hour < 7.2 ? 55 : hour > 12 && hour < 12.6 ? 112 : 74
    const bpm = around(base, hour < 7.2 ? 3 : 8)
    heart_rate_today.push({ start_utc: utc(new Date(t)), bpm_avg: bpm, bpm_min: bpm - 6, bpm_max: bpm + 9 })
  }

  const kinds: [string, string, number, number | null, number | null, string | null][] = [
    ['WALKING', 'Walk', 28, 104, 1900, null],
    ['STRENGTH_TRAINING', 'Strength training', 62, 121, null, null],
    ['WALKING', 'Walk', 41, 98, 3100, null],
    ['RUNNING', 'Run', 32, 152, 5200, null],
    ['STRENGTH_TRAINING', 'Strength training', 55, null, null, 'com.hevy'],
  ]
  const workouts: Workout[] = Array.from({ length: 10 }, (_, i) => {
    const [type, name, mins, hr, dist, app] = kinds[i % kinds.length]
    const start = daysAgo(Math.floor(i / 2), i % 2 ? 18 : 12, 10)
    return {
      id: `w-${i}`,
      start_utc: utc(start),
      end_utc: utc(new Date(start.getTime() + mins * 60_000)),
      start_offset_s: -start.getTimezoneOffset() * 60,
      exercise_type: type,
      display_name: name,
      source_platform: app ? 'HEALTH_CONNECT' : 'FITBIT',
      source_app: app,
      active_duration_s: mins * 60,
      avg_hr: hr,
      calories_kcal: hr ? Math.round(mins * (hr - 60) * 0.12 + 40) : null,
      distance_m: dist,
    }
  })

  const startDate = civil(daysAgo(rangeDays - 1))
  const inRange = <T extends { date: string }>(rows: T[]) => rows.filter((x) => x.date >= startDate)
  const syncedAt = utc(new Date(now.getTime() - 4 * 60_000))
  return {
    today: civil(now),
    range_start: startDate,
    range_days: rangeDays,
    activity: inRange(activity),
    vitals: inRange(vitals),
    sleep: sleep.filter((s) => s.civil_end_date >= startDate),
    last_sleep: sleep.at(-1)!,
    heart_rate_today,
    workouts,
    weight: [{ sampled_utc: utc(daysAgo(1, 7, 30)), civil_date: civil(daysAgo(1)), grams: 72_400 }],
    devices: [
      {
        name: 'users/me/pairedDevices/1',
        product: 'Pixel Watch 3',
        device_type: 'TRACKER',
        battery_level: 68,
        battery_status: 'Medium',
        last_sync_utc: utc(new Date(now.getTime() - 7 * 60_000)),
      },
    ],
    sync: ['steps', 'sleep', 'exercise', 'heart-rate'].map((t) => ({
      data_type: t,
      status: 'ok' as const,
      message: null,
      last_success_utc: syncedAt,
      backfilled_from: civil(daysAgo(HISTORY - 1)),
    })),
    syncing: false,
  }
}

mockIPC(
  (cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>
    switch (cmd) {
      case 'account_status':
      case 'sign_in':
        return { state: 'signed_in', missing_scopes: [] }
      case 'sign_out':
        return { state: 'signed_out' }
      case 'dashboard':
        return build(Number(a.rangeDays) || 30)
      case 'desktop_theme':
        return { accent: null, dark: null }
      case 'quit':
        return null
      case 'sync_now':
        // Pretend to sync for a couple of seconds.
        setTimeout(() => emit('sync-progress', 'steps: 14 items'), 400)
        setTimeout(() => emit('sync-finished', { ok: true, account: null, failures: [] }), 2000)
        return true
    }
  },
  { shouldMockEvents: true },
)
