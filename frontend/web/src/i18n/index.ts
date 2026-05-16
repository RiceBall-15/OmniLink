/**
 * 国际化模块
 * 轻量级 i18n 实现，支持嵌套键、插值、复数
 */

import { useState, useCallback, useEffect, createContext, useContext, type ReactNode } from 'react'

/** 支持的语言 */
export type Locale = 'zh-CN' | 'en-US'

/** 翻译字典 */
export type TranslationDict = Record<string, string | TranslationDict>

/** 插值参数 */
export type InterpolationParams = Record<string, string | number>

/** i18n 上下文类型 */
interface I18nContextType {
  locale: Locale
  setLocale: (locale: Locale) => void
  t: (key: string, params?: InterpolationParams) => string
}

/** 语言配置 */
const LOCALE_CONFIG: Record<Locale, { label: string; flag: string }> = {
  'zh-CN': { label: '简体中文', flag: '🇨🇳' },
  'en-US': { label: 'English', flag: '🇺🇸' },
}

/** localStorage 键名 */
const LOCALE_STORAGE_KEY = 'omnilink_locale'

/**
 * 获取浏览器默认语言
 */
function getBrowserLocale(): Locale {
  const lang = navigator.language || 'zh-CN'
  if (lang.startsWith('en')) return 'en-US'
  return 'zh-CN'
}

/**
 * 获取保存的语言偏好
 */
function getSavedLocale(): Locale {
  try {
    const saved = localStorage.getItem(LOCALE_STORAGE_KEY) as Locale
    if (saved && LOCALE_CONFIG[saved]) return saved
  } catch {
    // localStorage 不可用
  }
  return getBrowserLocale()
}

/**
 * 保存语言偏好
 */
function saveLocale(locale: Locale): void {
  try {
    localStorage.setItem(LOCALE_STORAGE_KEY, locale)
    document.documentElement.lang = locale
  } catch {
    // localStorage 不可用
  }
}

/**
 * 从嵌套字典中获取翻译
 */
function getNestedValue(dict: TranslationDict, key: string): string | undefined {
  const parts = key.split('.')
  let current: string | TranslationDict | undefined = dict

  for (const part of parts) {
    if (typeof current !== 'object' || current === null) {
      return undefined
    }
    current = current[part]
  }

  return typeof current === 'string' ? current : undefined
}

/**
 * 插值替换
 * 将 {{name}} 替换为参数值
 */
function interpolate(template: string, params?: InterpolationParams): string {
  if (!params) return template

  return template.replace(/\{\{(\w+)\}\}/g, (_, key) => {
    const value = params[key]
    return value !== undefined ? String(value) : `{{${key}}}`
  })
}

/** 翻译资源注册表 */
const translationRegistry: Record<Locale, TranslationDict> = {} as any

/**
 * 注册翻译资源
 */
export function registerTranslations(locale: Locale, translations: TranslationDict): void {
  translationRegistry[locale] = translations
}

/**
 * 创建 i18n 上下文
 */
export const I18nContext = createContext<I18nContextType>({
  locale: 'zh-CN',
  setLocale: () => {},
  t: (key: string) => key,
})

/**
 * i18n Provider 组件
 */
export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(getSavedLocale)

  const setLocale = useCallback((newLocale: Locale) => {
    setLocaleState(newLocale)
    saveLocale(newLocale)
  }, [])

  const t = useCallback(
    (key: string, params?: InterpolationParams): string => {
      const dict = translationRegistry[locale]
      if (!dict) return key

      const value = getNestedValue(dict, key)
      if (value === undefined) {
        // 降级到中文
        if (locale !== 'zh-CN') {
          const fallback = translationRegistry['zh-CN']
          if (fallback) {
            const fallbackValue = getNestedValue(fallback, key)
            if (fallbackValue !== undefined) {
              return interpolate(fallbackValue, params)
            }
          }
        }
        console.warn(`[i18n] Missing translation: ${key}`)
        return key
      }

      return interpolate(value, params)
    },
    [locale]
  )

  useEffect(() => {
    document.documentElement.lang = locale
  }, [locale])

  return (
    <I18nContext.Provider value={{ locale, setLocale, t }}>
      {children}
    </I18nContext.Provider>
  )
}

/**
 * 使用 i18n 的钩子
 */
export function useI18n() {
  return useContext(I18nContext)
}

/**
 * 获取所有可用语言
 */
export function getAvailableLocales() {
  return Object.entries(LOCALE_CONFIG).map(([code, config]) => ({
    code: code as Locale,
    ...config,
  }))
}
