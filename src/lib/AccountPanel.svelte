<!--
  Shows the account state and the one action that fixes it
  (sign in, sign in again, retry...). Every AccountStatus variant has a case.
  App.svelte owns the status and shows this panel whenever the dashboard
  can't be shown; actions here update the shared status.
-->
<script lang="ts">
  import { accountStatus, signIn, signOut, type AccountStatus } from './account'

  let { status = $bindable() }: { status: AccountStatus | null } = $props()
  let busy = $state(false)

  async function run(action: () => Promise<AccountStatus>) {
    busy = true
    try {
      status = await action()
    } catch (e) {
      status = { state: 'error', message: String(e) }
    } finally {
      busy = false
    }
  }
</script>

<section class="panel" aria-busy={busy}>
  {#if status === null}
    <p class="muted">Checking account…</p>
  {:else if status.state === 'signed_in'}
    <h2><span class="dot ok"></span>Connected to Google Health</h2>
    {#if status.missing_scopes.length > 0}
      <p class="warn">
        Permission not granted for: {status.missing_scopes.join(', ')}. Related cards will be empty.
      </p>
      <button onclick={() => run(signIn)} disabled={busy}>Re-authorise</button>
    {/if}
    <button class="secondary" onclick={() => run(signOut)} disabled={busy}>Sign out</button>
  {:else if status.state === 'signed_out'}
    <h2><span class="dot"></span>Not signed in</h2>
    <p class="muted">Sign in with the Google account you use in the Google Health app.</p>
    <button onclick={() => run(signIn)} disabled={busy}>{busy ? 'Waiting for browser…' : 'Sign in with Google'}</button>
  {:else if status.state === 'session_expired'}
    <h2><span class="dot warn"></span>Session expired</h2>
    <p class="muted">Google no longer accepts the saved sign-in. While the app is in Testing mode this happens every 7 days.</p>
    <button onclick={() => run(signIn)} disabled={busy}>{busy ? 'Waiting for browser…' : 'Sign in again'}</button>
  {:else if status.state === 'not_linked'}
    <h2><span class="dot warn"></span>Account not linked</h2>
    <p class="muted">
      Signed in, but this Google account isn't linked to Google Health yet. Open the Google Health app on your phone,
      sign in with the same account, then retry.
    </p>
    <button onclick={() => run(accountStatus)} disabled={busy}>Retry</button>
    <button class="secondary" onclick={() => run(signOut)} disabled={busy}>Use another account</button>
  {:else if status.state === 'offline'}
    <h2><span class="dot warn"></span>Offline</h2>
    <p class="muted">Can't reach Google. Check your connection.</p>
    <button onclick={() => run(accountStatus)} disabled={busy}>Retry</button>
  {:else if status.state === 'no_config'}
    <h2><span class="dot bad"></span>OAuth client not configured</h2>
    <p class="muted">Put the client JSON from Google Cloud Console at <code>{status.path}</code>.</p>
    <p class="detail">{status.message}</p>
    <button onclick={() => run(accountStatus)} disabled={busy}>Retry</button>
  {:else if status.state === 'keyring_unavailable'}
    <h2><span class="dot bad"></span>Keyring unavailable</h2>
    <p class="muted">VitalGlass stores your sign-in in KWallet (Secret Service). Make sure it's enabled and unlocked.</p>
    <p class="detail">{status.message}</p>
    <button onclick={() => run(accountStatus)} disabled={busy}>Retry</button>
  {:else}
    <h2><span class="dot bad"></span>Something went wrong</h2>
    <p class="detail">{status.message}</p>
    <button onclick={() => run(accountStatus)} disabled={busy}>Retry</button>
  {/if}
</section>

<style>
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 1.25rem 1.5rem;
    max-width: 36rem;
  }
  h2 {
    font-size: 1.1rem;
    margin: 0 0 0.5rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .dot {
    width: 0.6rem;
    height: 0.6rem;
    border-radius: 50%;
    background: var(--muted);
  }
  .dot.ok {
    background: var(--ok);
  }
  .dot.warn {
    background: var(--warn);
  }
  .dot.bad {
    background: var(--bad);
  }
  p.warn {
    color: var(--warn);
  }
  .detail {
    font-family: monospace;
    font-size: 0.85rem;
    color: var(--muted);
    word-break: break-word;
  }
  button + button {
    margin-left: 0.5rem;
  }
</style>
