<!--
  Theme picker: a settings button that opens a small menu of radio items.
  "System" follows the Plasma colour scheme.
-->
<script lang="ts">
  import Icon from './Icon.svelte'
  import { THEMES, theme } from './theme.svelte'

  let open = $state(false)
  let root: HTMLElement

  const options = [{ id: 'system', name: 'System (follow Plasma)' }, ...THEMES]

  function choose(id: string) {
    theme.setPreference(id)
    open = false
  }

  // Close on outside click or Escape.
  function onWindowClick(e: MouseEvent) {
    if (open && !root.contains(e.target as Node)) open = false
  }
  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') open = false
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={onKey} />

<div class="theme-menu" bind:this={root}>
  <button
    class="icon-only"
    aria-label="Theme"
    aria-haspopup="menu"
    aria-expanded={open}
    title="Theme"
    onclick={() => (open = !open)}
  >
    <Icon name="settings" size={18} />
  </button>
  {#if open}
    <div class="menu" role="menu" aria-label="Theme">
      <p class="label">Theme</p>
      {#each options as o (o.id)}
        <button role="menuitemradio" aria-checked={theme.preference === o.id} onclick={() => choose(o.id)}>
          <span class="dot" class:on={theme.preference === o.id}></span>
          {o.name}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .theme-menu {
    position: relative;
  }
  .menu {
    position: absolute;
    right: 0;
    top: calc(100% + 6px);
    z-index: 10;
    min-width: 13rem;
    background: var(--surface-raised);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 6px 18px rgb(0 0 0 / 25%);
    padding: 0.35rem;
  }
  .label {
    font-size: 0.75rem;
    color: var(--muted);
    margin: 0.2rem 0.5rem 0.3rem;
  }
  .menu button {
    width: 100%;
    background: none;
    border: 1px solid transparent;
    justify-content: flex-start;
    padding: 0.35rem 0.5rem;
  }
  .menu button:hover {
    background: var(--surface);
  }
  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    border: 1.5px solid var(--muted);
    flex: none;
  }
  .dot.on {
    border-color: var(--accent);
    background: var(--accent);
    box-shadow: inset 0 0 0 2px var(--surface-raised);
  }
</style>
