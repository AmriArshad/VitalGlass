<!--
  A dashboard card: title, optional header extras, and one of three states.
  'permission_denied' and 'empty' replace the content with a message, so
  every card handles missing data the same way.
-->
<script lang="ts">
  import type { Snippet } from 'svelte'

  interface Props {
    title: string
    state?: 'ok' | 'empty' | 'permission_denied'
    emptyText?: string
    onreauth?: () => void
    extra?: Snippet
    children: Snippet
  }

  let { title, state = 'ok', emptyText = 'No data yet', onreauth, extra, children }: Props = $props()
</script>

<section class="card">
  <header>
    <h2>{title}</h2>
    {#if extra}{@render extra()}{/if}
  </header>
  {#if state === 'permission_denied'}
    <p class="message">
      Permission not granted.
      {#if onreauth}<button class="link" onclick={onreauth}>Re-authorise</button>{/if}
    </p>
  {:else if state === 'empty'}
    <p class="message">{emptyText}</p>
  {:else}
    {@render children()}
  {/if}
</section>

<style>
  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0.9rem 1.1rem 1rem;
    min-width: 0;
  }
  header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }
  h2 {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--muted);
    margin: 0;
  }
  .message {
    color: var(--muted);
    margin: 0.75rem 0;
  }
  .link {
    background: none;
    border: none;
    padding: 0;
    color: var(--accent);
    text-decoration: underline;
  }
</style>
