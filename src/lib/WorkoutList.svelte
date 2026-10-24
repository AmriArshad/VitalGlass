<!-- Recent workouts: type, when, duration, average HR, calories, distance. -->
<script lang="ts">
  import type { Workout } from './dashboard'
  import { fmtDay, fmtEnum, fmtInt, fmtKm, fmtMinutes, fmtTime } from './format'

  let { workouts, limit = 5 }: { workouts: Workout[]; limit?: number } = $props()
  let expanded = $state(false)
  const shown = $derived(expanded ? workouts : workouts.slice(0, limit))

  /** Health Connect apps other than Google's own are labelled, e.g. "Hevy". */
  function source(w: Workout): string | null {
    if (w.source_platform !== 'HEALTH_CONNECT' || !w.source_app) return null
    const name = w.source_app.split('.').at(-1) ?? w.source_app
    return name.charAt(0).toUpperCase() + name.slice(1)
  }
</script>

<table>
  <thead>
    <tr>
      <th>Workout</th>
      <th class="num">Time</th>
      <th class="num">Avg HR</th>
      <th class="num">kcal</th>
      <th class="num">km</th>
    </tr>
  </thead>
  <tbody>
    {#each shown as w (w.id)}
      <tr>
        <td>
          <div class="name">
            {w.display_name || fmtEnum(w.exercise_type)}
            {#if source(w)}<span class="tag">{source(w)}</span>{/if}
          </div>
          <div class="when">{fmtDay(w.start_utc)} {fmtTime(w.start_utc)}</div>
        </td>
        <td class="num">{fmtMinutes(w.active_duration_s == null ? null : w.active_duration_s / 60)}</td>
        <td class="num">{fmtInt(w.avg_hr)}</td>
        <td class="num">{fmtInt(w.calories_kcal)}</td>
        <td class="num">{w.distance_m ? fmtKm(w.distance_m) : '–'}</td>
      </tr>
    {/each}
  </tbody>
</table>
{#if workouts.length > limit}
  <button class="link more" onclick={() => (expanded = !expanded)}>
    {expanded ? 'Show fewer' : `Show ${workouts.length - limit} more`}
  </button>
{/if}

<style>
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.9rem;
  }
  th {
    text-align: left;
    font-weight: 400;
    color: var(--muted);
    font-size: 0.8rem;
    padding: 0 0.4rem 0.3rem;
  }
  td {
    padding: 0.4rem;
    border-top: 1px solid var(--border);
    vertical-align: top;
  }
  .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .when {
    color: var(--muted);
    font-size: 0.8rem;
  }
  .more {
    margin-top: 0.5rem;
    font-size: 0.85rem;
    align-self: flex-start;
  }
  .tag {
    font-size: 0.7rem;
    color: var(--muted);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 0 0.3rem;
    margin-left: 0.3rem;
  }
</style>
