<!--
  The dashboard. Reads everything from the local store via the `dashboard`
  command and re-reads when a sync reports new data. Never waits on the
  network, so it renders instantly with the last known data.
-->
<script lang="ts">
  import { onDestroy, onMount } from 'svelte'
  import Card from './Card.svelte'
  import Icon, { type IconName } from './Icon.svelte'
  import Chart from './Chart.svelte'
  import Stat from './Stat.svelte'
  import WorkoutList from './WorkoutList.svelte'
  import { cardPermission, getDashboard, onDataChanged, type Dashboard } from './dashboard'
  import { civilDays, fmtCivil, fmtLongDate, fmtInt, fmtKm, fmtMinutes, fmtTime } from './format'

  interface Props {
    onreauth: () => void
    /** Bumped by the parent to force a re-read. */
    data?: Dashboard | null
  }

  let { onreauth, data = $bindable(null) }: Props = $props()

  const RANGES = [7, 30, 90] as const
  const RANGE_KEY = 'vitalglass.range'
  let range = $state<number>(loadRange())
  let error = $state<string | null>(null)

  function loadRange(): number {
    try {
      const n = Number(localStorage.getItem(RANGE_KEY))
      return (RANGES as readonly number[]).includes(n) ? n : 30
    } catch {
      return 30
    }
  }

  async function load() {
    try {
      data = await getDashboard(range)
      error = null
    } catch (e) {
      error = String(e)
    }
  }

  function setRange(n: number) {
    range = n
    try {
      localStorage.setItem(RANGE_KEY, String(n))
    } catch {
      // Per-viewer convenience only.
    }
    load()
  }

  let unlisten: (() => void) | undefined
  onMount(async () => {
    load()
    unlisten = await onDataChanged(load)
  })
  onDestroy(() => unlisten?.())

  // --- derived views of the data -------------------------------------------

  const days = $derived(data ? civilDays(data.range_start, data.today) : [])
  const labels = $derived(days.map(fmtCivil))
  const activity = $derived(new Map(data?.activity.map((a) => [a.date, a]) ?? []))
  const vitals = $derived(new Map(data?.vitals.map((v) => [v.date, v]) ?? []))
  const today = $derived(data ? activity.get(data.today) : undefined)

  /** Most recent non-null value of a vitals field, with its date. */
  function latest<K extends keyof NonNullable<Dashboard>['vitals'][number]>(key: K) {
    const rows = data?.vitals ?? []
    for (let i = rows.length - 1; i >= 0; i--) {
      const v = rows[i][key]
      if (v != null) return { value: v as number, date: rows[i].date }
    }
    return null
  }

  const restingHr = $derived(data && latest('resting_hr'))
  const hrv = $derived(data && latest('hrv_ms'))
  const spo2 = $derived(data && latest('spo2_avg'))
  const breathing = $derived(data && latest('resp_rate'))
  const vo2 = $derived(data && latest('vo2max'))
  const weight = $derived(data?.weight.at(-1) ?? null)

  /** Main sleep per night (civil end date). */
  const nights = $derived(
    new Map((data?.sleep ?? []).filter((s) => s.main_sleep).map((s) => [s.civil_end_date, s])),
  )
  const lastSleep = $derived(data?.last_sleep ?? null)
  const sleepEfficiency = $derived(
    lastSleep?.minutes_asleep && lastSleep.minutes_in_period
      ? Math.round((100 * lastSleep.minutes_asleep) / lastSleep.minutes_in_period)
      : null,
  )

  // The API omits days with no zone minutes, so a day that has activity
  // data but no AZM values means 0, not "unknown".
  const azmToday = $derived(
    today ? (today.azm_fat_burn ?? 0) + (today.azm_cardio ?? 0) + (today.azm_peak ?? 0) : null,
  )

  const perm = (types: string[]) => (data ? cardPermission(data.sync, types) : 'ok')
  const sinceLabel = (date: string | undefined) => (date && date !== data?.today ? `on ${fmtCivil(date)}` : 'today')

  const hours = (min: number | null | undefined) => (min == null ? null : min / 60)

  /** Mean of the non-null values, or null if there are none. */
  function avg(values: (number | null | undefined)[]): number | null {
    const v = values.filter((x): x is number => x != null)
    return v.length ? v.reduce((a, b) => a + b, 0) / v.length : null
  }
  const fmtAvg = (values: (number | null | undefined)[], digits: number) => avg(values)?.toFixed(digits) ?? '–'
  const hrLabels = $derived(data?.heart_rate_today.map((b) => fmtTime(b.start_utc)) ?? [])
  /** Label the heart rate axis every 3 hours, on the hour. */
  const hrTickAt = $derived(
    data?.heart_rate_today.map((b) => {
      const t = new Date(b.start_utc)
      return t.getMinutes() === 0 && t.getHours() % 3 === 0
    }) ?? [],
  )
  const hrRange = $derived.by(() => {
    const v = data?.heart_rate_today.map((b) => b.bpm_avg) ?? []
    return v.length ? `${Math.round(Math.min(...v))}–${Math.round(Math.max(...v))} bpm` : ''
  })

  /** Sleep stages in chart order, with their series colour. */
  const stages = $derived(
    (
      [
        ['Deep', 'minutes_deep', '--series-1'],
        ['Light', 'minutes_light', '--series-2'],
        ['REM', 'minutes_rem', '--series-3'],
        ['Awake', 'minutes_awake', '--series-4'],
      ] as const
    ).map(([label, key, color]) => ({ label, key, color, minutes: lastSleep?.[key] ?? null })),
  )

  /** The small stats in the Vitals card. VO2 max and weight only when present. */
  const vitalTiles = $derived.by(() => {
    const denied = (t: string) => perm([t]) === 'permission_denied'
    const tiles: { label: string; icon: IconName; value: string | null; unit: string; sub: string; denied: boolean }[] = [
      { label: 'HRV', icon: 'activity', value: hrv?.value.toFixed(0) ?? null, unit: 'ms', sub: sinceLabel(hrv?.date), denied: denied('daily-heart-rate-variability') },
      { label: 'SpO₂', icon: 'droplet', value: spo2?.value.toFixed(1) ?? null, unit: '%', sub: sinceLabel(spo2?.date), denied: denied('daily-oxygen-saturation') },
      { label: 'Breathing', icon: 'lungs', value: breathing?.value.toFixed(1) ?? null, unit: 'br/min', sub: sinceLabel(breathing?.date), denied: denied('daily-respiratory-rate') },
    ]
    if (vo2) tiles.push({ label: 'VO₂ max', icon: 'run', value: vo2.value.toFixed(0), unit: 'ml/kg/min', sub: sinceLabel(vo2.date), denied: false })
    if (weight) tiles.push({ label: 'Weight', icon: 'scale', value: (weight.grams / 1000).toFixed(1), unit: 'kg', sub: sinceLabel(weight.civil_date), denied: false })
    return tiles
  })
</script>

{#if error}
  <p class="error">Couldn't read the local cache: {error}</p>
{:else if data}
  <div class="section-head">
    <h2>Today</h2>
    <span class="muted">{fmtLongDate(data.today)}</span>
  </div>
  <div class="grid four">
    <Card title="Steps" icon="shoe" state={perm(['steps'])} {onreauth}>
      <Stat
        value={fmtInt(today?.steps)}
        sub="{fmtKm(today?.distance_m)} km{today?.floors != null ? ` · ${today.floors} floors` : ''}"
      />
    </Card>
    <Card title="Active zone minutes" icon="heartbeat" state={perm(['active-zone-minutes'])} {onreauth}>
      <Stat
        value={fmtInt(azmToday)}
        unit="min"
        sub={today
          ? `fat burn ${today.azm_fat_burn ?? 0} · cardio ${today.azm_cardio ?? 0} · peak ${today.azm_peak ?? 0}`
          : undefined}
      />
    </Card>
    <Card title="Calories" icon="flame" state={perm(['total-calories'])} {onreauth}>
      <Stat value={fmtInt(today?.calories_kcal)} unit="kcal" sub="burned so far" />
    </Card>
    <Card
      title="Resting heart rate"
      icon="heart"
      state={perm(['daily-resting-heart-rate']) === 'ok' && !restingHr ? 'empty' : perm(['daily-resting-heart-rate'])}
      {onreauth}
    >
      <Stat value={fmtInt(restingHr?.value)} unit="bpm" sub={sinceLabel(restingHr?.date)} />
    </Card>
  </div>

  <div class="section-head">
    <h2>Last night</h2>
  </div>
  <div class="grid two">
    <Card
      title="Sleep"
      icon="moon"
      state={perm(['sleep']) === 'ok' && !lastSleep ? 'empty' : perm(['sleep'])}
      emptyText="No sleep recorded"
      {onreauth}
    >
      {#if lastSleep}
        <Stat
          value={fmtMinutes(lastSleep.minutes_asleep)}
          unit="asleep"
          sub="{fmtTime(lastSleep.start_utc)} – {fmtTime(lastSleep.end_utc)}{sleepEfficiency
            ? ` · ${sleepEfficiency}% efficient`
            : ''}{lastSleep.civil_end_date !== data.today ? ` · ${fmtCivil(lastSleep.civil_end_date)}` : ''}"
        />
        {#if lastSleep.minutes_deep != null}
          <div class="stagebar" role="img" aria-label="Sleep stages">
            {#each stages as st (st.label)}
              <span style:flex-grow={st.minutes ?? 0} style:background="var({st.color})"></span>
            {/each}
          </div>
          <p class="legend">
            {#each stages as st (st.label)}
              <span><i style:background="var({st.color})"></i>{st.label} <b>{fmtMinutes(st.minutes)}</b></span>
            {/each}
          </p>
        {/if}
      {/if}
    </Card>
    <Card title="Vitals" icon="activity">
      <div class="minis">
        {#each vitalTiles as m (m.label)}
          <div class="mini">
            <p class="mini-label"><Icon name={m.icon} size={14} />{m.label}</p>
            {#if m.denied}
              <p class="mini-sub">Permission not granted</p>
            {:else}
              <p class="mini-value">{m.value ?? '–'}<span class="unit">{m.unit}</span></p>
              <p class="mini-sub">{m.sub}</p>
            {/if}
          </div>
        {/each}
      </div>
    </Card>
  </div>

  <div class="section-head">
    <h2>Trends</h2>
    <div class="toggle" role="group" aria-label="Trend range">
      {#each RANGES as n (n)}
        <button class:active={range === n} aria-pressed={range === n} onclick={() => setRange(n)}>{n} days</button>
      {/each}
    </div>
  </div>
  <div class="grid two">
    <Card title="Resting heart rate" icon="heart" state={perm(['daily-resting-heart-rate'])} {onreauth}>
      {#snippet extra()}avg {fmtAvg(days.map((d) => vitals.get(d)?.resting_hr), 0)} bpm{/snippet}
      <Chart
        type="line"
        {labels}
        series={[{ label: 'Resting HR', color: '--series-1', data: days.map((d) => vitals.get(d)?.resting_hr ?? null) }]}
        format={(v) => `${Math.round(v)} bpm`}
        ariaLabel="Resting heart rate per day"
      />
    </Card>
    <Card title="Heart rate variability" icon="activity" state={perm(['daily-heart-rate-variability'])} {onreauth}>
      {#snippet extra()}avg {fmtAvg(days.map((d) => vitals.get(d)?.hrv_ms), 0)} ms{/snippet}
      <Chart
        type="line"
        {labels}
        series={[{ label: 'HRV', color: '--series-1', data: days.map((d) => vitals.get(d)?.hrv_ms ?? null) }]}
        format={(v) => `${Math.round(v)} ms`}
        ariaLabel="Heart rate variability per night"
      />
    </Card>
    <Card title="Sleep" icon="moon" state={perm(['sleep'])} {onreauth}>
      {#snippet extra()}avg {fmtMinutes(avg(days.map((d) => nights.get(d)?.minutes_asleep)))} asleep{/snippet}
      <Chart
        type="bar"
        stacked
        {labels}
        series={stages.map((st) => ({
          label: st.label,
          color: st.color,
          data: days.map((d) => hours(nights.get(d)?.[st.key])),
        }))}
        format={(v) => fmtMinutes(v * 60)}
        ariaLabel="Sleep stages per night"
      />
    </Card>
    <Card title="Steps" icon="shoe" state={perm(['steps'])} {onreauth}>
      {#snippet extra()}avg {fmtInt(avg(days.map((d) => activity.get(d)?.steps)))} a day{/snippet}
      <Chart
        type="bar"
        {labels}
        series={[{ label: 'Steps', color: '--series-1', data: days.map((d) => activity.get(d)?.steps ?? null) }]}
        format={(v) => (v >= 1000 ? `${(v / 1000).toFixed(v >= 10000 ? 0 : 1)}K` : String(Math.round(v)))}
        ariaLabel="Steps per day"
      />
    </Card>
  </div>

  <div class="section-head">
    <h2>Activity</h2>
  </div>
  <div class="grid wide">
    <Card
      title="Heart rate today"
      icon="heartbeat"
      state={perm(['heart-rate']) === 'ok' && data.heart_rate_today.length === 0 ? 'empty' : perm(['heart-rate'])}
      {onreauth}
    >
      {#snippet extra()}{hrRange}{/snippet}
      <Chart
        type="line"
        labels={hrLabels}
        series={[{ label: 'Heart rate', color: '--series-1', data: data.heart_rate_today.map((b) => b.bpm_avg) }]}
        format={(v) => `${Math.round(v)} bpm`}
        showTick={(i) => hrTickAt[i]}
        ariaLabel="Heart rate today in 5-minute averages"
        height="fill"
      />
    </Card>
    <Card
      title="Recent workouts"
      icon="run"
      state={perm(['exercise']) === 'ok' && data.workouts.length === 0 ? 'empty' : perm(['exercise'])}
      emptyText="No workouts recorded"
      {onreauth}
    >
      <WorkoutList workouts={data.workouts} />
    </Card>
  </div>
{:else}
  <p class="muted">Loading…</p>
{/if}

<style>
  .section-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 1rem;
    margin: 1.4rem 0 0.6rem;
  }
  .section-head:first-child {
    margin-top: 0.4rem;
  }
  .section-head h2 {
    font-size: 1rem;
    font-weight: 600;
    margin: 0;
  }
  .section-head .muted {
    font-size: 0.85rem;
  }
  .grid {
    display: grid;
    gap: 0.75rem;
  }
  .grid.four {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }
  .grid.two {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
  .grid.wide {
    grid-template-columns: minmax(0, 7fr) minmax(0, 5fr);
  }
  @media (max-width: 1000px) {
    .grid.four {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .grid.two,
    .grid.wide {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  /* Last night's sleep: one bar split by stage, 2px gaps between segments. */
  .stagebar {
    display: flex;
    gap: 2px;
    height: 10px;
    margin: 0.9rem 0 0.5rem;
  }
  .stagebar span {
    flex-basis: 0;
    min-width: 2px;
  }
  .stagebar span:first-child {
    border-radius: 4px 0 0 4px;
  }
  .stagebar span:last-child {
    border-radius: 0 4px 4px 0;
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem 1rem;
    font-size: 0.8rem;
    color: var(--muted);
    margin: 0;
  }
  .legend b {
    color: var(--text);
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }
  .legend i {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 2px;
    margin-right: 0.35rem;
  }

  /* Vitals card: small stats in equal columns, divided by hairlines. */
  .minis {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(90px, 1fr));
    flex: 1;
    align-items: center;
  }
  .mini {
    padding: 0.2rem 0.9rem;
    border-left: 1px solid var(--border);
  }
  .mini:first-child {
    padding-left: 0;
    border-left: none;
  }
  .mini p {
    margin: 0;
  }
  .mini-label {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.8rem;
    color: var(--muted);
  }
  .mini-value {
    font-size: 1.45rem;
    font-weight: 600;
    line-height: 1.3;
    font-variant-numeric: tabular-nums;
  }
  .mini-value .unit {
    font-size: 0.8rem;
    font-weight: 400;
    color: var(--muted);
    margin-left: 0.25rem;
  }
  .mini-sub {
    font-size: 0.8rem;
    color: var(--muted);
  }

  .toggle {
    display: flex;
    border: 1px solid var(--border);
    border-radius: 5px;
    overflow: hidden;
  }
  .toggle button {
    background: var(--surface);
    border: none;
    border-radius: 0;
    color: var(--muted);
    padding: 0.2rem 0.75rem;
    font-size: 0.85rem;
  }
  .toggle button + button {
    border-left: 1px solid var(--border);
  }
  .toggle button.active {
    background: var(--accent);
    color: var(--on-accent);
  }
  .error {
    color: var(--bad);
  }
</style>
