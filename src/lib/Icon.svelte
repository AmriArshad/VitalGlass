<!--
  Tabler outline icons (MIT), inlined from @tabler/icons so they inherit
  currentColor and size with the text. Only the icons listed here are
  bundled.
-->
<script lang="ts" module>
  import activity from '@tabler/icons/outline/activity.svg?raw'
  import deviceWatch from '@tabler/icons/outline/device-watch.svg?raw'
  import droplet from '@tabler/icons/outline/droplet.svg?raw'
  import flame from '@tabler/icons/outline/flame.svg?raw'
  import heart from '@tabler/icons/outline/heart.svg?raw'
  import heartbeat from '@tabler/icons/outline/heartbeat.svg?raw'
  import lungs from '@tabler/icons/outline/lungs.svg?raw'
  import moon from '@tabler/icons/outline/moon.svg?raw'
  import refresh from '@tabler/icons/outline/refresh.svg?raw'
  import run from '@tabler/icons/outline/run.svg?raw'
  import scale from '@tabler/icons/outline/scale.svg?raw'
  import settings from '@tabler/icons/outline/settings.svg?raw'
  import shoe from '@tabler/icons/outline/shoe.svg?raw'
  import cloud from '@tabler/icons/outline/cloud.svg?raw'
  import cloudOff from '@tabler/icons/outline/cloud-off.svg?raw'

  const ICONS = {
    activity,
    cloud,
    'cloud-off': cloudOff,
    'device-watch': deviceWatch,
    droplet,
    flame,
    heart,
    heartbeat,
    lungs,
    moon,
    refresh,
    run,
    scale,
    settings,
    shoe,
  }

  export type IconName = keyof typeof ICONS
</script>

<script lang="ts">
  let { name, size = 16, spin = false }: { name: IconName; size?: number; spin?: boolean } = $props()
</script>

<!-- eslint-disable-next-line svelte/no-at-html-tags -- bundled SVG files, not user input -->
<span class="icon" class:spin style:width="{size}px" style:height="{size}px" aria-hidden="true">{@html ICONS[name]}</span>

<style>
  .icon {
    display: inline-flex;
    flex: none;
  }
  .icon :global(svg) {
    width: 100%;
    height: 100%;
  }
  .spin {
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
    }
  }
</style>
