import { useThemeStore, type ThemeMode } from '../stores/themeStore'
import { useI18n } from '../i18n/setup'
import './ThemeToggle.css'

/**
 * 主题切换器组件
 */
export function ThemeToggle() {
  const { theme, setTheme } = useThemeStore()
  const { t } = useI18n()

  const options: { value: ThemeMode; icon: string; label: string }[] = [
    { value: 'light', icon: '☀️', label: t('settings.lightMode') },
    { value: 'dark', icon: '🌙', label: t('settings.darkMode') },
    { value: 'auto', icon: '🌓', label: t('settings.systemTheme') },
  ]

  return (
    <div className="theme-toggle">
      {options.map(opt => (
        <button
          key={opt.value}
          className={`theme-toggle__btn ${theme === opt.value ? 'theme-toggle__btn--active' : ''}`}
          onClick={() => setTheme(opt.value)}
          aria-label={opt.label}
        >
          <span className="theme-toggle__icon">{opt.icon}</span>
          <span className="theme-toggle__label">{opt.label}</span>
        </button>
      ))}
    </div>
  )
}
