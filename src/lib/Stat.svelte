<!-- The big number inside a stat card, with its unit and a detail line. -->
<script lang="ts">
  interface Props {
    value: string
    unit?: string
    sub?: string
  }

  let { value, unit, sub }: Props = $props()
</script>

<p class="value">{value}{#if unit}<span class="unit">{unit}</span>{/if}</p>
{#if sub}<p class="sub">{sub}</p>{/if}

<style>
  .value {
    font-size: 1.9rem;
    font-family: var(--font-numeric);
    font-weight: var(--value-weight);
    line-height: 1.1;
    margin: 0;
  }
  .unit {
    font-size: 0.95rem;
    font-weight: 400;
    color: var(--muted);
    margin-left: 0.3rem;
  }
  .sub {
    color: var(--muted);
    font-size: 0.85rem;
    margin: 0.35rem 0 0;
  }
</style>
