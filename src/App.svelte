<!--
  App shell. Owns the account status and decides what to show:
  - signed in (or offline with a cache): header, dashboard, footer
  - anything else: the account panel with the action that fixes it
  Starts a sync on launch and after sign-in; the backend also syncs
  every 15 minutes and reports progress through events.
-->
<script lang="ts">
  import { onDestroy, onMount } from 'svelte'
  import AccountPanel from './lib/AccountPanel.svelte'
  import Dashboard from './lib/Dashboard.svelte'
  import Icon from './lib/Icon.svelte'
  import Logo from './lib/Logo.svelte'
  import ThemeMenu from './lib/ThemeMenu.svelte'
  import { initTheme, theme } from './lib/theme.svelte'
  import { accountStatus, signIn, signOut, type AccountStatus } from './lib/account'
  import {
    onSyncFinished,
    onSyncProgress,
    quitApp,
    syncNow,
    type Dashboard as DashboardData,
    type SyncFinished,
  } from './lib/dashboard'
  import { fmtAgo, fmtEnum } from './lib/format'

  let status = $state<AccountStatus | null>(null)
  let data = $state<DashboardData | null>(null)
  let syncing = $state(false)
  let progress = $state('')
  let lastSync = $state<SyncFinished | null>(null)
  /** Ticks every minute so "x min ago" stays current. */
  let now = $state(Date.now())

  // Offline still shows the dashboard from cache, with a banner.
  const showDashboard = $derived(status?.state === 'signed_in' || status?.state === 'offline')
  const missingScopes = $derived(status?.state === 'signed_in' ? status.missing_scopes : [])

  const device = $derived(data?.devices[0])
  const lastApiSync = $derived(
    data?.sync
      .map((s) => s.last_success_utc)
      .filter((t): t is string => !!t)
      .sort()
      .at(-1),
  )

  async function startSync() {
    // false means one is already running; it'll still report back.
    await syncNow()
    syncing = true
    progress = 'Starting…'
  }

  async function reauthorise() {
    status = await signIn()
  }

  async function doSignOut() {
    status = await signOut()
    data = null
  }

  // Sync whenever we become signed in: on launch, and after sign-in.
  let wasSignedIn = false
  $effect(() => {
    const signedIn = status?.state === 'signed_in'
    if (signedIn && !wasSignedIn) startSync()
    wasSignedIn = signedIn
  })

  // Sticky header: compact once the top 40px have scrolled away. An
  // IntersectionObserver is cheaper than a scroll listener and never runs
  // on scroll frames where nothing changes.
  let compact = $state(false)
  let sentinel = $state<HTMLElement>()
  $effect(() => {
    if (!sentinel) return
    const observer = new IntersectionObserver(([entry]) => (compact = !entry.isIntersecting))
    observer.observe(sentinel)
    return () => observer.disconnect()
  })

  const unlisteners: (() => void)[] = [initTheme()]
  let timer: ReturnType<typeof setInterval>
  onMount(async () => {
    unlisteners.push(await onSyncProgress((line) => (progress = line)))
    unlisteners.push(
      await onSyncFinished((f) => {
        syncing = false
        progress = ''
        lastSync = f
        // Account problems (expired, not linked...) switch to the panel;
        // offline keeps the dashboard with a banner.
        if (f.account && f.account.state !== 'error') status = f.account
      }),
    )
    timer = setInterval(() => (now = Date.now()), 60_000)
    status = await accountStatus()
  })
  // Ctrl+Q quits; closing the window only hides it to the tray.
  function onKey(e: KeyboardEvent) {
    if (e.ctrlKey && e.key.toLowerCase() === 'q') quitApp()
  }

  onDestroy(() => {
    unlisteners.forEach((u) => u())
    clearInterval(timer)
  })
</script>

<svelte:window onkeydown={onKey} />

{#if showDashboard}
  <!-- Scrolling past this marks the header compact (see onMount). -->
  <div class="scroll-sentinel" bind:this={sentinel} aria-hidden="true"></div>
  <header class="top" class:compact>
    <h1><span class="logo"><Logo size={26} /></span> VitalGlass</h1>
    <div class="actions">
      <span class="muted status" aria-live="polite">
        {syncing ? progress : `Updated ${fmtAgo(lastApiSync, now)}`}
      </span>
      <button class="primary" onclick={startSync} disabled={syncing}>
        <Icon name="refresh" spin={syncing} />
        {syncing ? 'Syncing' : 'Sync'}
      </button>
      <ThemeMenu />
    </div>
  </header>
  <!-- Holds the full header's space, since the header itself is fixed. -->
  <div class="header-space" aria-hidden="true"></div>

  <main>
    {#if status?.state === 'offline'}
      <p class="banner warn">Offline. Showing data from the last sync.</p>
    {:else if lastSync?.account?.state === 'error'}
      <p class="banner bad">Sync failed: {lastSync.account.message}</p>
    {:else if lastSync && lastSync.failures.length > 0}
      <p class="banner warn">
        Some data didn't sync:
        {lastSync.failures.map((f) => `${fmtEnum(f.data_type.replace(/-/g, '_'))} (${f.status.replace('_', ' ')})`).join(', ')}
      </p>
    {/if}
    {#if missingScopes.length > 0}
      <p class="banner warn">
        Permission not granted for {missingScopes.join(', ')}.
        <button class="link" onclick={reauthorise}>Re-authorise</button>
      </p>
    {/if}

    <!-- Re-mount on theme change so charts pick up the new colours. -->
    {#key theme.resolved.id}
      <Dashboard bind:data onreauth={reauthorise} />
    {/key}
  </main>

  <footer>
    <span class="item">
      <Icon name="device-watch" />
      {#if device}
        {device.product ?? 'Device'} synced {fmtAgo(device.last_sync_utc, now)}{device.battery_level != null
          ? ` · battery ${device.battery_level}%`
          : ''}
      {:else}
        No paired device
      {/if}
    </span>
    <span class="item">
      <Icon name={status?.state === 'offline' ? 'cloud-off' : 'cloud'} />
      {status?.state === 'offline' ? 'Offline' : 'Connected to Google Health'}
      · <button class="link" onclick={doSignOut}>Sign out</button>
    </span>
  </footer>
{:else}
  <main class="centered">
    <h1><Logo size={30} /> VitalGlass</h1>
    {#if status === null}
      <p class="muted">Checking account…</p>
    {:else}
      <AccountPanel bind:status />
    {/if}
  </main>
{/if}

<style>
  .scroll-sentinel {
    position: absolute;
    top: 0;
    height: 40px;
    width: 1px;
    pointer-events: none;
  }

  /* The header stays at the top and shrinks into a compact bar once you
     scroll. It's fixed rather than part of the page flow, with
     .header-space holding its full height, so shrinking it never moves
     the content underneath. */
  .top {
    --header-full: 3.6rem;
    --header-compact: 2.6rem;
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    z-index: 20;
    box-sizing: border-box;
    height: var(--header-full);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 1.5rem;
    border-bottom: 1px solid transparent;
    background: transparent;
    transition:
      height 220ms ease,
      background-color 220ms ease,
      border-color 220ms ease,
      box-shadow 220ms ease;
  }
  .top.compact {
    height: var(--header-compact);
    background: var(--bg);
    border-bottom-color: var(--border);
    box-shadow: 0 4px 14px rgb(0 0 0 / 18%);
  }
  .header-space {
    height: 3.6rem;
  }
  /* Buttons shrink with the bar. */
  .top :global(button) {
    transition:
      padding 220ms ease,
      font-size 220ms ease,
      border-color 120ms,
      background-color 120ms;
  }
  .compact :global(button) {
    padding: 0.15rem 0.6rem;
    font-size: 0.85rem;
  }
  .compact :global(button.icon-only) {
    padding: 0.2rem;
  }
  h1 {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    font-size: 1.3rem;
    font-weight: 600;
    margin: 0;
    transition: font-size 220ms ease;
  }
  .compact h1 {
    font-size: 1.05rem;
  }
  .logo {
    display: inline-flex;
    transition: transform 220ms ease;
  }
  .compact .logo {
    transform: scale(0.8);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  .status {
    font-size: 0.85rem;
    max-width: 28rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    transition: font-size 220ms ease;
  }
  .compact .status {
    font-size: 0.8rem;
  }
  @media (prefers-reduced-motion: reduce) {
    .top,
    .top :global(button),
    h1,
    .logo,
    .status {
      transition: none;
    }
  }
  main {
    padding: 0.6rem 1.5rem 1rem;
  }
  .centered {
    max-width: 40rem;
    margin: 3rem auto;
  }
  .centered h1 {
    margin-bottom: 1.25rem;
  }
  .banner {
    border: 1px solid var(--border);
    border-left-width: 4px;
    border-radius: 4px;
    background: var(--surface);
    padding: 0.5rem 0.75rem;
    margin: 0 0 0.75rem;
  }
  .banner.warn {
    border-left-color: var(--warn);
  }
  .banner.bad {
    border-left-color: var(--bad);
  }
  footer {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 0.5rem 1.5rem;
    padding: 0.6rem 1.5rem 1rem;
    color: var(--muted);
    font-size: 0.85rem;
    border-top: 1px solid var(--border);
  }
  footer .item {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
  }
</style>
