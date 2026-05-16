import { useI18n, getAvailableLocales } from '../i18n/setup'
import './LanguageSwitcher.css'

/**
 * 语言切换器组件
 * 显示为下拉菜单，支持切换中英文
 */
export function LanguageSwitcher({ compact = false }: { compact?: boolean }) {
  const { locale, setLocale } = useI18n()
  const locales = getAvailableLocales()

  return (
    <div className={`lang-switcher ${compact ? 'lang-switcher--compact' : ''}`}>
      <select
        className="lang-switcher__select"
        value={locale}
        onChange={e => setLocale(e.target.value as any)}
        aria-label="切换语言"
      >
        {locales.map(l => (
          <option key={l.code} value={l.code}>
            {compact ? l.flag : `${l.flag} ${l.label}`}
          </option>
        ))}
      </select>
    </div>
  )
}
