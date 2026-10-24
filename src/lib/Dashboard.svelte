<!--
  The dashboard. Reads everything from the local store via the `dashboard`
  command and re-reads when a sync reports new data. Never waits on the
  network, so it renders instantly with the last known data.
-->
<script lang="ts">
  import { onDestroy, onMount } from 'svelte'
  import Card from './Card.svelte'
  import Chart from './Chart.svelte'
  import Stat from './Stat.svelte'
  import WorkoutList from './WorkoutList.svelte'
  import { cardPermission, getDashboard, onDataChanged, type Dashboard } from './dashboard'
  import { civilDays, fmtCivil, fmtInt, fmtKm, fmtMinutes, fmtTime } from './format'

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
  const hrLabels = $derived(data?.heart_rate_today.map((b) => fmtTime(b.start_utc)) ?? [])
</script>

{#if error}
  <p class="error">Couldn't read the local cache: {error}</p>
{:else if data}
  <div class="tiles">
    <Card title="Steps today" state={perm(['steps'])} {onreauth}>
      <Stat
        value={fmtInt(today?.steps)}
        sub="{fmtKm(today?.distance_m)} km{today?.floors != null ? ` · ${today.floors} floors` : ''}"
      />
    </Card>
    <Card title="Active zone minutes" state={perm(['active-zone-minutes'])} {onreauth}>
      <Stat
        value={fmtInt(azmToday)}
        unit="min"
        sub={today
          ? `fat burn ${today.azm_fat_burn ?? 0} · cardio ${today.azm_cardio ?? 0} · peak ${today.azm_peak ?? 0}`
          : undefined}
      />
    </Card>
    <Card title="Calories" state={perm(['total-calories'])} {onreauth}>
      <Stat value={fmtInt(today?.calories_kcal)} unit="kcal" sub="so far today" />
    </Card>
    <Card
      title="Resting heart rate"
      state={perm(['daily-resting-heart-rate']) === 'ok' && !restingHr ? 'empty' : perm(['daily-resting-heart-rate'])}
      {onreauth}
    >
      <Stat value={fmtInt(restingHr?.value)} unit="bpm" sub={sinceLabel(restingHr?.date)} />
    </Card>
    <Card
      title="Last sleep"
      state={perm(['sleep']) === 'ok' && !lastSleep ? 'empty' : perm(['sleep'])}
      emptyText="No sleep recorded"
      {onreauth}
    >
      {#if lastSleep}
        <Stat
          value={fmtMinutes(lastSleep.minutes_asleep)}
          sub="{fmtTime(lastSleep.start_utc)}–{fmtTime(lastSleep.end_utc)}{sleepEfficiency
            ? ` · ${sleepEfficiency}% efficient`
            : ''}"
        />
        {#if lastSleep.minutes_deep != null}
          <p class="stages">
            <span><i style:background="var(--series-1)"></i>Deep {fmtMinutes(lastSleep.minutes_deep)}</span>
            <span><i style:background="var(--series-2)"></i>Light {fmtMinutes(lastSleep.minutes_light)}</span>
            <span><i style:background="var(--series-3)"></i>REM {fmtMinutes(lastSleep.minutes_rem)}</span>
            <span><i style:background="var(--series-4)"></i>Awake {fmtMinutes(lastSleep.minutes_awake)}</span>
          </p>
        {/if}
      {/if}
    </Card>
  </div>

  <div class="tiles small">
    <Card title="HRV" state={perm(['daily-heart-rate-variability']) === 'ok' && !hrv ? 'empty' : perm(['daily-heart-rate-variability'])} {onreauth}>
      <Stat value={hrv ? hrv.value.toFixed(0) : '–'} unit="ms" sub={sinceLabel(hrv?.date)} />
    </Card>
    <Card title="SpO₂" state={perm(['daily-oxygen-saturation']) === 'ok' && !spo2 ? 'empty' : perm(['daily-oxygen-saturation'])} {onreauth}>
      <Stat value={spo2 ? spo2.value.toFixed(1) : '–'} unit="%" sub={sinceLabel(spo2?.date)} />
    </Card>
    <Card title="Breathing rate" state={perm(['daily-respiratory-rate']) === 'ok' && !breathing ? 'empty' : perm(['daily-respiratory-rate'])} {onreauth}>
      <Stat value={breathing ? breathing.value.toFixed(1) : '–'} unit="br/min" sub={sinceLabel(breathing?.date)} />
    </Card>
    {#if vo2}
      <Card title="VO₂ max">
        <Stat value={vo2.value.toFixed(0)} unit="ml/kg/min" sub={sinceLabel(vo2.date)} />
      </Card>
    {/if}
    {#if weight}
      <Card title="Weight">
        <Stat value={(weight.grams / 1000).toFixed(1)} unit="kg" sub={sinceLabel(weight.civil_date)} />
      </Card>
    {/if}
  </div>

  <div class="section-head">
    <h2>Trends</h2>
    <div class="toggle" role="group" aria-label="Trend range">
      {#each RANGES as n (n)}
        <button class:active={range === n} aria-pressed={range === n} onclick={() => setRange(n)}>{n} days</button>
      {/each}
    </div>
  </div>

  <div class="charts">
    <Card title="Resting heart rate" state={perm(['daily-resting-heart-rate'])} {onreauth}>
      <Chart
        type="line"
        {labels}
        series={[{ label: 'Resting HR', color: '--series-1', data: days.map((d) => vitals.get(d)?.resting_hr ?? null) }]}
        format={(v) => `${Math.round(v)} bpm`}
        ariaLabel="Resting heart rate per day"
      />
    </Card>
    <Card title="Heart rate variability" state={perm(['daily-heart-rate-variability'])} {onreauth}>
      <Chart
        type="line"
        {labels}
        series={[{ label: 'HRV', color: '--series-1', data: days.map((d) => vitals.get(d)?.hrv_ms ?? null) }]}
        format={(v) => `${Math.round(v)} ms`}
        ariaLabel="Heart rate variability per night"
      />
    </Card>
    <Card title="Sleep" state={perm(['sleep'])} {onreauth}>
      <Chart
        type="bar"
        stacked
        {labels}
        series={[
          { label: 'Deep', color: '--series-1', data: days.map((d) => hours(nights.get(d)?.minutes_deep)) },
          { label: 'Light', color: '--series-2', data: days.map((d) => hours(nights.get(d)?.minutes_light)) },
          { label: 'REM', color: '--series-3', data: days.map((d) => hours(nights.get(d)?.minutes_rem)) },
          { label: 'Awake', color: '--series-4', data: days.map((d) => hours(nights.get(d)?.minutes_awake)) },
        ]}
        format={(v) => fmtMinutes(v * 60)}
        ariaLabel="Sleep stages per night"
      />
    </Card>
    <Card title="Steps" state={perm(['steps'])} {onreauth}>
      <Chart
        type="bar"
        {labels}
        series={[{ label: 'Steps', color: '--series-1', data: days.map((d) => activity.get(d)?.steps ?? null) }]}
        format={(v) => (v >= 1000 ? `${(v / 1000).toFixed(v >= 10000 ? 0 : 1)}K` : String(Math.round(v)))}
        ariaLabel="Steps per day"
      />
    </Card>
  </div>

  <div class="charts wide">
    <Card
      title="Heart rate today"
      state={perm(['heart-rate']) === 'ok' && data.heart_rate_today.length === 0 ? 'empty' : perm(['heart-rate'])}
      {onreauth}
    >
      <Chart
        type="line"
        labels={hrLabels}
        series={[{ label: 'Heart rate', color: '--series-1', data: data.heart_rate_today.map((b) => b.bpm_avg) }]}
        format={(v) => `${Math.round(v)} bpm`}
        ariaLabel="Heart rate today in 5-minute averages"
        height={200}
      />
    </Card>
    <Card
      title="Recent workouts"
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
  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 0.75rem;
    margin-bottom: 0.75rem;
  }
  .tiles.small :global(.value) {
    font-size: 1.4rem;
  }
  .stages {
    display: flex;
    flex-wrap: wrap;
    gap: 0.2rem 0.75rem;
    font-size: 0.8rem;
    color: var(--muted);
    margin: 0.5rem 0 0;
  }
  .stages i {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 2px;
    margin-right: 0.3rem;
  }
  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin: 1.25rem 0 0.75rem;
  }
  .section-head h2 {
    font-size: 1rem;
    margin: 0;
  }
  .toggle {
    display: flex;
    border: 1px solid var(--border);
    border-radius: 4px;
    overflow: hidden;
  }
  .toggle button {
    background: var(--surface);
    border: none;
    border-radius: 0;
    color: var(--muted);
    padding: 0.25rem 0.75rem;
  }
  .toggle button.active {
    background: var(--surface-raised);
    color: var(--text);
  }
  .charts {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(420px, 1fr));
    gap: 0.75rem;
    margin-bottom: 0.75rem;
  }
  .charts.wide {
    grid-template-columns: 3fr 2fr;
    /* Don't stretch the heart rate card to the workout list's height. */
    align-items: start;
  }
  @media (max-width: 1000px) {
    .charts.wide {
      grid-template-columns: 1fr;
    }
  }
  .error {
    color: var(--bad);
  }
</style>
