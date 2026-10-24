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
  import { accountStatus, signIn, signOut, type AccountStatus } from './lib/account'
  import {
    onSyncFinished,
    onSyncProgress,
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

  const unlisteners: (() => void)[] = []
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
  onDestroy(() => {
    unlisteners.forEach((u) => u())
    clearInterval(timer)
  })
</script>

{#if showDashboard}
  <header class="top">
    <h1>VitalGlass</h1>
    <div class="sync">
      {#if syncing}<span class="muted progress">{progress}</span>{/if}
      <button onclick={startSync} disabled={syncing}>{syncing ? 'Syncing…' : 'Sync'}</button>
    </div>
  </header>

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

    <Dashboard bind:data onreauth={reauthorise} />
  </main>

  <footer>
    <span>
      {#if device}
        {device.product ?? 'Device'} synced {fmtAgo(device.last_sync_utc, now)}{device.battery_level != null
          ? ` · battery ${device.battery_level}%`
          : ''}
      {:else}
        No paired device
      {/if}
    </span>
    <span>Data updated {fmtAgo(lastApiSync, now)}</span>
    <span>
      {status?.state === 'offline' ? 'Offline' : 'Connected to Google Health'}
      · <button class="link" onclick={doSignOut}>Sign out</button>
    </span>
  </footer>
{:else}
  <main class="centered">
    <h1>VitalGlass</h1>
    {#if status === null}
      <p class="muted">Checking account…</p>
    {:else}
      <AccountPanel bind:status />
    {/if}
  </main>
{/if}

<style>
  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.9rem 1.5rem 0;
  }
  h1 {
    font-size: 1.3rem;
    font-weight: 600;
    margin: 0;
  }
  .sync {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }
  .progress {
    font-size: 0.85rem;
    max-width: 28rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  main {
    padding: 1rem 1.5rem;
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
  .link {
    background: none;
    border: none;
    padding: 0;
    color: var(--accent);
    text-decoration: underline;
  }
</style>
