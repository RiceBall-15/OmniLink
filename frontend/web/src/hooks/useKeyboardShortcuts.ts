/**
 * useKeyboardShortcuts Hook
 * 键盘快捷键管理 hook
 */

import { useEffect, useCallback, useRef } from 'react'

export interface KeyboardShortcut {
  /** 快捷键组合，如 'ctrl+k', 'ctrl+shift+n' */
  key: string
  /** 快捷键描述 */
  description: string
  /** 回调函数 */
  handler: (event: KeyboardEvent) => void
  /** 是否在输入框中也生效（默认 false） */
  enableInInput?: boolean
  /** 是否阻止默认行为（默认 true） */
  preventDefault?: boolean
}

interface ShortcutMap {
  [key: string]: KeyboardShortcut
}

/**
 * 解析快捷键字符串为匹配条件
 */
function parseShortcutKey(key: string): {
  ctrl: boolean
  shift: boolean
  alt: boolean
  meta: boolean
  code: string
} {
  const parts = key.toLowerCase().split('+').map(s => s.trim())
  return {
    ctrl: parts.includes('ctrl'),
    shift: parts.includes('shift'),
    alt: parts.includes('alt'),
    meta: parts.includes('meta') || parts.includes('cmd'),
    code: parts.find(p => !['ctrl', 'shift', 'alt', 'meta', 'cmd'].includes(p)) || '',
  }
}

/**
 * 检查事件是否匹配快捷键
 */
function matchShortcut(event: KeyboardEvent, key: string): boolean {
  const parsed = parseShortcutKey(key)

  // 检查修饰键
  if (parsed.ctrl !== event.ctrlKey) return false
  if (parsed.shift !== event.shiftKey) return false
  if (parsed.alt !== event.altKey) return false
  if (parsed.meta !== event.metaKey) return false

  // 检查主键
  const eventKey = event.key.toLowerCase()
  const eventCode = event.code.toLowerCase()

  return (
    eventKey === parsed.code ||
    eventCode === `key${parsed.code}` ||
    eventCode === parsed.code ||
    eventKey === parsed.code.replace('arrow', '')
  )
}

/**
 * 键盘快捷键管理 Hook
 * @param shortcuts 快捷键配置数组
 * @param enabled 是否启用（默认 true）
 *
 * @example
 * useKeyboardShortcuts([
 *   {
 *     key: 'ctrl+k',
 *     description: '打开搜索',
 *     handler: () => setSearchOpen(true),
 *   },
 *   {
 *     key: 'ctrl+n',
 *     description: '新建消息',
 *     handler: () => openNewMessage(),
 *   },
 *   {
 *     key: 'escape',
 *     description: '关闭弹窗',
 *     handler: () => closeModal(),
 *   },
 * ])
 */
export function useKeyboardShortcuts(
  shortcuts: KeyboardShortcut[],
  enabled: boolean = true
): void {
  const shortcutsRef = useRef<ShortcutMap>({})

  // 构建快捷键映射
  useEffect(() => {
    const map: ShortcutMap = {}
    shortcuts.forEach(shortcut => {
      map[shortcut.key.toLowerCase()] = shortcut
    })
    shortcutsRef.current = map
  }, [shortcuts])

  // 键盘事件处理
  const handleKeyDown = useCallback(
    (event: KeyboardEvent) => {
      if (!enabled) return

      // 检查是否在输入框中
      const target = event.target as HTMLElement
      const isInput =
        target.tagName === 'INPUT' ||
        target.tagName === 'TEXTAREA' ||
        target.tagName === 'SELECT' ||
        target.isContentEditable

      // 遍历所有快捷键
      for (const [, shortcut] of Object.entries(shortcutsRef.current)) {
        if (matchShortcut(event, shortcut.key)) {
          // 如果在输入框中，且快捷键未启用输入框模式，则跳过
          if (isInput && !shortcut.enableInInput) {
            // 特殊处理：Escape 总是在输入框中生效
            if (shortcut.key.toLowerCase() !== 'escape') {
              continue
            }
          }

          // 阻止默认行为
          if (shortcut.preventDefault !== false) {
            event.preventDefault()
          }

          // 执行回调
          shortcut.handler(event)
          break
        }
      }
    },
    [enabled]
  )

  // 注册事件监听
  useEffect(() => {
    if (!enabled) return

    window.addEventListener('keydown', handleKeyDown)
    return () => {
      window.removeEventListener('keydown', handleKeyDown)
    }
  }, [enabled, handleKeyDown])
}

/**
 * 获取快捷键的显示文本（适配 Mac/Windows）
 */
export function getShortcutDisplay(key: string): string {
  const isMac =
    typeof navigator !== 'undefined' && /Mac|iPod|iPhone|iPad/.test(navigator.platform)

  const parts = key.split('+').map(s => s.trim().toLowerCase())

  const displayMap: Record<string, string> = {
    ctrl: isMac ? '⌘' : 'Ctrl',
    shift: isMac ? '⇧' : 'Shift',
    alt: isMac ? '⌥' : 'Alt',
    meta: '⌘',
    cmd: '⌘',
    enter: '↵',
    escape: 'Esc',
    backspace: '⌫',
    delete: 'Del',
    tab: '⇥',
    space: '␣',
    arrowup: '↑',
    arrowdown: '↓',
    arrowleft: '←',
    arrowright: '→',
  }

  return parts.map(p => displayMap[p] || p.toUpperCase()).join(isMac ? '' : '+')
}

/**
 * 注册全局搜索快捷键 (Ctrl+K / Cmd+K)
 * @param onSearch 搜索回调
 * @param enabled 是否启用
 */
export function useSearchShortcut(
  onSearch: () => void,
  enabled: boolean = true
): void {
  useKeyboardShortcuts(
    [
      {
        key: 'ctrl+k',
        description: '打开搜索',
        handler: onSearch,
        enableInInput: true,
      },
    ],
    enabled
  )
}
