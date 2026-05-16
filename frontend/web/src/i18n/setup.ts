/**
 * i18n 初始化
 * 注册翻译资源并导出便捷方法
 */

import { registerTranslations, type Locale } from './index'
import zhCN from './locales/zh-CN'
import enUS from './locales/en-US'

// 注册翻译资源
registerTranslations('zh-CN', zhCN)
registerTranslations('en-US', enUS)

// 导出所有 i18n 相关内容
export { I18nProvider, useI18n, getAvailableLocales, type Locale } from './index'
export type { InterpolationParams } from './index'
