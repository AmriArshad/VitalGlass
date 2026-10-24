<!--
  Last night's sleep as a hypnogram: one row per stage (Awake at the top,
  Deep at the bottom), each segment a bar at its place in the night.
  Labels are HTML so they stay crisp; only the bars are SVG, stretched to
  the card width.
-->
<script lang="ts">
  import type { SleepStageSegment } from './dashboard'
  import { fmtMinutes, fmtTime } from './format'

  let { segments }: { segments: SleepStageSegment[] } = $props()

  // Rows top to bottom, with the series colour each stage uses everywhere else.
  const ROWS = [
    { label: 'Awake', color: '--series-4', types: ['AWAKE', 'RESTLESS'] },
    { label: 'REM', color: '--series-3', types: ['REM'] },
    { label: 'Light', color: '--series-2', types: ['LIGHT', 'ASLEEP'] },
    { label: 'Deep', color: '--series-1', types: ['DEEP'] },
  ]
  const ROW_H = 30

  const t = (iso: string) => new Date(iso).getTime()
  const start = $derived(segments.length ? t(segments[0].start_utc) : 0)
  const end = $derived(segments.length ? Math.max(...segments.map((s) => t(s.end_utc))) : 1)
  const span = $derived(Math.max(end - start, 1))
  const pct = (ms: number) => ((ms - start) / span) * 100

  const bars = $derived(
    segments.flatMap((s) => {
      const row = ROWS.findIndex((r) => r.types.includes(s.stage))
      if (row < 0) return []
      const minutes = (t(s.end_utc) - t(s.start_utc)) / 60_000
      return [
        {
          x: pct(t(s.start_utc)),
          w: Math.max(pct(t(s.end_utc)) - pct(t(s.start_utc)), 0.15),
          row,
          color: ROWS[row].color,
          title: `${ROWS[row].label} ${fmtTime(s.start_utc)}–${fmtTime(s.end_utc)} (${fmtMinutes(minutes)})`,
        },
      ]
    }),
  )

  /** Whole hours inside the night: every hour for short sleeps, every 2 otherwise. */
  const ticks = $derived.by(() => {
    const step = span > 6 * 3_600_000 ? 2 : 1
    const first = new Date(start)
    first.setMinutes(0, 0, 0)
    first.setHours(first.getHours() + 1)
    const out: { left: number; label: string }[] = []
    for (let d = first; d.getTime() < end; d = new Date(d.getTime() + step * 3_600_000)) {
      out.push({ left: pct(d.getTime()), label: fmtTime(d.toISOString()) })
    }
    return out
  })
</script>

<div class="timeline">
  <div class="labels">
    {#each ROWS as r (r.label)}
      <span style:height="{ROW_H}px">{r.label}</span>
    {/each}
  </div>
  <div class="plot">
    <svg viewBox="0 0 100 {ROWS.length * ROW_H}" preserveAspectRatio="none" role="img" aria-label="Sleep stages through the night">
      {#each ROWS as _, i (i)}
        <line x1="0" x2="100" y1={(i + 0.5) * ROW_H} y2={(i + 0.5) * ROW_H} class="guide" vector-effect="non-scaling-stroke" />
      {/each}
      {#each bars as b, i (i)}
        <rect x={b.x} y={b.row * ROW_H + 6} width={b.w} height={ROW_H - 12} rx="0.4" style:fill="var({b.color})">
          <title>{b.title}</title>
        </rect>
      {/each}
    </svg>
    <div class="axis">
      {#each ticks as tick (tick.left)}
        <span style:left="{tick.left}%">{tick.label}</span>
      {/each}
    </div>
  </div>
</div>

<style>
  /* Fixed row heights (stretching would turn bars into blocks), centred
     vertically when the card is taller than the chart. */
  .timeline {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.6rem;
    margin: auto 0;
  }
  .labels {
    display: flex;
    flex-direction: column;
    font-size: 0.75rem;
    color: var(--muted);
  }
  .labels span {
    display: flex;
    align-items: center;
  }
  .plot {
    min-width: 0;
  }
  svg {
    display: block;
    width: 100%;
    height: 120px;
  }
  .guide {
    stroke: var(--grid);
    stroke-width: 1;
  }
  rect:hover {
    opacity: 0.8;
  }
  .axis {
    position: relative;
    height: 1.2rem;
    margin-top: 0.3rem;
    font-size: 0.75rem;
    color: var(--muted);
  }
  .axis span {
    position: absolute;
    transform: translateX(-50%);
    white-space: nowrap;
  }
</style>
