/**
 * 主题管理 Store
 * 支持深色模式、浅色模式、跟随系统
 */

import { useState, useEffect, useCallback } from 'react'

export type ThemeMode = 'light' | 'dark' | 'auto'

const THEME_STORAGE_KEY = 'omnilink_theme'

/**
 * 获取保存的主题偏好
 */
function getSavedTheme(): ThemeMode {
  try {
    const saved = localStorage.getItem(THEME_STORAGE_KEY) as ThemeMode
    if (saved && ['light', 'dark', 'auto'].includes(saved)) return saved
  } catch {}
  return 'auto'
}

/**
 * 保存主题偏好
 */
function saveTheme(theme: ThemeMode): void {
  try {
    localStorage.setItem(THEME_STORAGE_KEY, theme)
  } catch {}
}

/**
 * 应用主题到 DOM
 */
function applyTheme(theme: ThemeMode): void {
  const root = document.documentElement

  if (theme === 'auto') {
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches
    root.setAttribute('data-theme', prefersDark ? 'dark' : 'light')
  } else {
    root.setAttribute('data-theme', theme)
  }
}

/**
 * 主题管理 Hook
 */
export function useThemeStore() {
  const [theme, setThemeState] = useState<ThemeMode>(getSavedTheme)
  const [isDark, setIsDark] = useState(false)

  // 应用主题
  useEffect(() => {
    applyTheme(theme)
    setIsDark(
      theme === 'dark' ||
      (theme === 'auto' && window.matchMedia('(prefers-color-scheme: dark)').matches)
    )
  }, [theme])

  // 监听系统主题变化
  useEffect(() => {
    if (theme !== 'auto') return

    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
    const handler = (e: MediaQueryListEvent) => {
      setIsDark(e.matches)
      applyTheme('auto')
    }

    mediaQuery.addEventListener('change', handler)
    return () => mediaQuery.removeEventListener('change', handler)
  }, [theme])

  const setTheme = useCallback((newTheme: ThemeMode) => {
    setThemeState(newTheme)
    saveTheme(newTheme)
  }, [])

  const toggleTheme = useCallback(() => {
    setTheme(isDark ? 'light' : 'dark')
  }, [isDark, setTheme])

  return { theme, setTheme, isDark, toggleTheme }
}
