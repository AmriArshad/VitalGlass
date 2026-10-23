<script lang="ts">
  import AccountPanel from './lib/AccountPanel.svelte'
</script>

<main>
  <header>
    <h1>VitalGlass</h1>
  </header>
  <AccountPanel />
  <p class="muted placeholder">The dashboard cards arrive in phase 4.</p>
</main>

<style>
  main {
    padding: 1.5rem 2rem;
  }
  header h1 {
    font-size: 1.4rem;
    font-weight: 600;
    margin: 0 0 1.25rem;
  }
  .placeholder {
    margin-top: 2rem;
  }
</style>
