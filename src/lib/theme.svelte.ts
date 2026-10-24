// Theme selection. The user picks a preference ('system' or a theme id);
// 'system' resolves to Breeze Dark or Light from the Plasma colour scheme,
// falling back to the prefers-color-scheme media query outside KDE.
// The resolved theme is applied as data-theme on <html>; colours live in
// src/themes.css.

import { invoke } from '@tauri-apps/api/core'

export interface Theme {
  id: string
  name: string
  dark: boolean
  /** Replace --accent with the KDE accent colour. */
  kdeAccent: boolean
}

/** Every theme in src/themes.css. Add new themes here too. */
export const THEMES: Theme[] = [
  { id: 'breeze-dark', name: 'Breeze Dark', dark: true, kdeAccent: true },
  { id: 'breeze-light', name: 'Breeze Light', dark: false, kdeAccent: true },
]

/** What 'system' resolves to for each mode. */
const SYSTEM = { dark: 'breeze-dark', light: 'breeze-light' }

const KEY = 'vitalglass.theme'

interface DesktopTheme {
  accent: string | null
  dark: boolean | null
}

class ThemeState {
  /** 'system' or a theme id. */
  preference = $state(loadPreference())
  systemDark = $state(true)
  systemAccent = $state<string | null>(null)

  readonly resolved: Theme = $derived.by(() => {
    const id = this.preference === 'system' ? (this.systemDark ? SYSTEM.dark : SYSTEM.light) : this.preference
    return THEMES.find((t) => t.id === id) ?? THEMES[0]
  })

  setPreference(p: string) {
    this.preference = p
    try {
      localStorage.setItem(KEY, p)
    } catch {
      // Remembering the choice is a convenience; the app works without it.
    }
  }
}

function loadPreference(): string {
  try {
    const p = localStorage.getItem(KEY)
    if (p === 'system' || THEMES.some((t) => t.id === p)) return p!
  } catch {
    // Storage unavailable: use the default.
  }
  return 'system'
}

export const theme = new ThemeState()

/** Read the desktop scheme now and whenever the window regains focus. */
export function initTheme(): () => void {
  const media = window.matchMedia('(prefers-color-scheme: dark)')

  async function refresh() {
    let desktop: DesktopTheme = { accent: null, dark: null }
    try {
      desktop = await invoke<DesktopTheme>('desktop_theme')
    } catch {
      // Not running under Tauri (e.g. plain Vite in a browser).
    }
    theme.systemDark = desktop.dark ?? media.matches
    theme.systemAccent = desktop.accent
  }

  refresh()
  window.addEventListener('focus', refresh)
  media.addEventListener('change', refresh)

  // Apply the resolved theme and accent to <html> whenever they change.
  const stop = $effect.root(() => {
    $effect(() => {
      const root = document.documentElement
      const t = theme.resolved
      root.dataset.theme = t.id
      if (t.kdeAccent && theme.systemAccent) {
        root.style.setProperty('--accent', theme.systemAccent)
        root.style.setProperty('--on-accent', readableOn(theme.systemAccent))
      } else {
        root.style.removeProperty('--accent')
        root.style.removeProperty('--on-accent')
      }
    })
  })

  return () => {
    stop()
    window.removeEventListener('focus', refresh)
    media.removeEventListener('change', refresh)
  }
}

/** Near-white or near-black text, whichever reads better on `hex`. */
function readableOn(hex: string): string {
  const n = parseInt(hex.slice(1), 16)
  const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255]
  return 0.299 * r + 0.587 * g + 0.114 * b > 170 ? '#232629' : '#fcfcfc'
}
