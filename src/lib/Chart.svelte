<!--
  Thin wrapper around Chart.js for the dashboard's line and bar charts.
  Colours are CSS custom properties from app.css (e.g. '--series-1'), read
  once at creation so the chart matches the theme.
-->
<script lang="ts" module>
  import {
    BarController,
    BarElement,
    CategoryScale,
    Chart,
    Filler,
    Legend,
    LinearScale,
    LineController,
    LineElement,
    PointElement,
    Tooltip,
  } from 'chart.js'

  // Register only what we use, so the bundle stays small.
  Chart.register(
    BarController,
    BarElement,
    CategoryScale,
    Filler,
    Legend,
    LinearScale,
    LineController,
    LineElement,
    PointElement,
    Tooltip,
  )

  export interface Series {
    label: string
    data: (number | null)[]
    /** CSS custom property name, e.g. '--series-1' */
    color: string
  }
</script>

<script lang="ts">
  import { onDestroy, onMount } from 'svelte'

  interface Props {
    type: 'line' | 'bar'
    /** X-axis tick labels. */
    labels: string[]
    series: Series[]
    stacked?: boolean
    /** Formats y-axis ticks and tooltip values. */
    format?: (v: number) => string
    /** Accessible description of the chart. */
    ariaLabel: string
    /** Pixel height, or 'fill' to grow into the card. */
    height?: number | 'fill'
    /** Which x-axis labels to show, by index. Default: Chart.js auto-skip. */
    showTick?: (index: number) => boolean
  }

  let {
    type,
    labels,
    series,
    stacked = false,
    format = (v) => String(v),
    ariaLabel,
    height = 190,
    showTick,
  }: Props = $props()

  let canvas: HTMLCanvasElement
  let chart: Chart | null = null

  const css = (name: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim()

  function datasets() {
    const surface = css('--surface')
    return series.map((s) => {
      const color = css(s.color)
      return type === 'bar'
        ? {
            label: s.label,
            data: s.data,
            backgroundColor: color,
            // A surface-coloured border gives the 2px gap between stacked segments.
            borderColor: surface,
            borderWidth: stacked ? 1 : 0,
            borderRadius: stacked ? 0 : 4,
            maxBarThickness: 18,
          }
        : {
            label: s.label,
            data: s.data,
            borderColor: color,
            backgroundColor: color,
            borderWidth: 2,
            pointRadius: 0,
            pointHoverRadius: 4,
            pointHoverBorderColor: surface,
            pointHoverBorderWidth: 2,
            // Monotone smoothing never overshoots the data, unlike plain tension.
            cubicInterpolationMode: 'monotone' as const,
            spanGaps: false,
          }
    })
  }

  onMount(() => {
    // Charts use the theme's UI font (monospace in Monitor, for example).
    Chart.defaults.font.family = css('--font-ui')
    const grid = css('--grid')
    const muted = css('--muted')
    chart = new Chart(canvas, {
      type,
      data: { labels, datasets: datasets() },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        animation: false,
        interaction: { mode: 'index', intersect: false },
        plugins: {
          legend: {
            display: series.length > 1,
            position: 'top',
            align: 'start',
            labels: { color: muted, boxWidth: 10, boxHeight: 10, usePointStyle: true, pointStyle: 'rectRounded' },
          },
          tooltip: {
            backgroundColor: css('--surface-raised'),
            borderColor: css('--border'),
            borderWidth: 1,
            titleColor: css('--text'),
            bodyColor: css('--text'),
            callbacks: {
              label: (ctx) => (ctx.parsed.y == null ? '' : `${ctx.dataset.label}: ${format(ctx.parsed.y)}`),
            },
          },
        },
        scales: {
          x: {
            stacked,
            grid: { display: false },
            border: { color: grid },
            ticks: showTick
              ? {
                  color: muted,
                  maxRotation: 0,
                  autoSkip: false,
                  callback(value, index) {
                    return showTick(index) ? this.getLabelForValue(Number(value)) : ''
                  },
                }
              : { color: muted, maxRotation: 0, autoSkipPadding: 16 },
          },
          y: {
            stacked,
            beginAtZero: type === 'bar',
            grid: { color: grid },
            border: { display: false },
            // Whole-number ticks: every series here is bpm, ms, hours or
            // steps, and fractional ticks would round to duplicate labels.
            ticks: { color: muted, maxTicksLimit: 5, precision: 0, callback: (v) => format(Number(v)) },
          },
        },
      },
    })
  })

  // Re-render when the data changes (range toggle, new sync).
  $effect(() => {
    const next = datasets()
    if (!chart) return
    chart.data.labels = labels
    chart.data.datasets = next as typeof chart.data.datasets
    chart.update()
  })

  onDestroy(() => chart?.destroy())
</script>

<div
  class="chart"
  class:fill={height === 'fill'}
  style:height={height === 'fill' ? null : `${height}px`}
  role="img"
  aria-label={ariaLabel}
>
  <canvas bind:this={canvas}></canvas>
</div>

<style>
  .chart {
    position: relative;
    width: 100%;
  }
  .chart.fill {
    flex: 1;
    min-height: 220px;
  }
  /* The canvas is taken out of the layout so only the container sets the
     chart's size. Otherwise a chart that grew with its card (e.g. when the
     workout list expands) would hold the card open when it should shrink. */
  .chart canvas {
    position: absolute;
    inset: 0;
  }
</style>
