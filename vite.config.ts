import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'

// Tauri loads the dev server at a fixed address, so fail rather than
// silently moving to another port.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    // Don't rebuild the frontend when Rust files change.
    watch: { ignored: ['**/src-tauri/**', '**/crates/**', '**/target/**'] },
  },
})
