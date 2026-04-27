import { useEffect, useState } from 'react'

type ThemePreference = 'system' | 'light' | 'dark'
type ActiveTheme = 'light' | 'dark'

const THEME_PREFERENCE_KEY = 'backlayer.theme.preference'

function detectSystemTheme(): ActiveTheme {
  if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') {
    return 'light'
  }
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

function readThemePreference(): ThemePreference {
  if (typeof window === 'undefined') {
    return 'system'
  }
  const stored = window.localStorage.getItem(THEME_PREFERENCE_KEY)
  if (stored === 'light' || stored === 'dark' || stored === 'system') {
    return stored
  }
  return 'system'
}

export function useTheme() {
  const [preference, setPreference] = useState<ThemePreference>(() => readThemePreference())
  const [systemTheme, setSystemTheme] = useState<ActiveTheme>(() => detectSystemTheme())

  const activeTheme: ActiveTheme = preference === 'system' ? systemTheme : preference

  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)')
    const update = (event?: MediaQueryListEvent) => {
      setSystemTheme(event?.matches ?? media.matches ? 'dark' : 'light')
    }
    update()
    media.addEventListener('change', update)
    return () => media.removeEventListener('change', update)
  }, [])

  useEffect(() => {
    window.localStorage.setItem(THEME_PREFERENCE_KEY, preference)
  }, [preference])

  useEffect(() => {
    document.documentElement.dataset.theme = activeTheme
    document.documentElement.style.colorScheme = activeTheme
    return () => {
      delete document.documentElement.dataset.theme
      document.documentElement.style.removeProperty('color-scheme')
    }
  }, [activeTheme])

  return { activeTheme, preference, setPreference }
}

export type { ThemePreference, ActiveTheme }
