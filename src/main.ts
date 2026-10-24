import { mount } from 'svelte'
import './themes.css'
import './app.css'
import App from './App.svelte'

// Dev only: /?mock swaps the Rust backend for generated data (see mock.ts).
if (import.meta.env.DEV && new URLSearchParams(location.search).has('mock')) {
  await import('./mock')
}

const app = mount(App, {
  target: document.getElementById('app')!,
})

export default app
