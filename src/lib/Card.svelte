<!--
  A dashboard card: icon and title, an optional summary on the right of
  the header, and one of three states. 'permission_denied' and 'empty'
  replace the content with a message, so every card handles missing data
  the same way.
-->
<script lang="ts">
  import type { Snippet } from 'svelte'
  import Icon, { type IconName } from './Icon.svelte'

  interface Props {
    title: string
    icon?: IconName
    state?: 'ok' | 'empty' | 'permission_denied'
    emptyText?: string
    onreauth?: () => void
    extra?: Snippet
    children: Snippet
  }

  let { title, icon, state = 'ok', emptyText = 'No data yet', onreauth, extra, children }: Props = $props()
</script>

<section class="card">
  <header>
    <h2>
      {#if icon}<Icon name={icon} size={15} />{/if}
      {title}
    </h2>
    {#if extra && state === 'ok'}<div class="extra">{@render extra()}</div>{/if}
  </header>
  {#if state === 'permission_denied'}
    <p class="message">
      Permission not granted.
      {#if onreauth}<button class="link" onclick={onreauth}>Re-authorise</button>{/if}
    </p>
  {:else if state === 'empty'}
    <p class="message">{emptyText}</p>
  {:else}
    <div class="body">{@render children()}</div>
  {/if}
</section>

<style>
  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: var(--shadow);
    padding: 0.85rem 1.1rem 1rem;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  /* Content fills the card, so cards in a row share one height and a
     chart can grow into the space. */
  .body {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    margin-bottom: 0.45rem;
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--muted);
    margin: 0;
  }
  .extra {
    font-size: 0.8rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .message {
    color: var(--muted);
    margin: 0.75rem 0;
  }
</style>
