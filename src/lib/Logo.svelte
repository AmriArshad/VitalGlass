<!--
  The VitalGlass mark: a rounded glass pane in the accent colour with a
  heartbeat line across it and a soft highlight in the corner. The same
  shape is used for the app icon (src-tauri/icons/icon.svg).
-->
<script lang="ts">
  let { size = 24 }: { size?: number } = $props()
</script>

<svg width={size} height={size} viewBox="0 0 64 64" aria-hidden="true">
  <rect x="4" y="4" width="56" height="56" rx="14" fill="var(--accent)" />
  <path d="M4 26 Q4 4 26 4 L44 4 Q20 10 4 34 Z" fill="var(--on-accent)" opacity="0.18" />
  <polyline
    points="12,36 22,36 27,24 33,46 38,30 42,36 52,36"
    fill="none"
    stroke="var(--on-accent)"
    stroke-width="4.5"
    stroke-linecap="round"
    stroke-linejoin="round"
  />
</svg>
