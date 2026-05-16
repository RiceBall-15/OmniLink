/**
 * KeyboardShortcutsPanel 组件
 * 键盘快捷键帮助面板
 */

import React, { useEffect } from 'react'
import { getShortcutDisplay, type KeyboardShortcut } from '../hooks/useKeyboardShortcuts'
import './KeyboardShortcutsPanel.css'

interface KeyboardShortcutsPanelProps {
  /** 快捷键列表 */
  shortcuts: KeyboardShortcut[]
  /** 是否显示 */
  isOpen: boolean
  /** 关闭回调 */
  onClose: () => void
}

export const KeyboardShortcutsPanel: React.FC<KeyboardShortcutsPanelProps> = ({
  shortcuts,
  isOpen,
  onClose,
}) => {
  // ESC 键关闭
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && isOpen) {
        onClose()
      }
    }

    if (isOpen) {
      window.addEventListener('keydown', handleKeyDown)
      return () => window.removeEventListener('keydown', handleKeyDown)
    }
  }, [isOpen, onClose])

  if (!isOpen) return null

  return (
    <div className="shortcuts-panel-overlay" onClick={onClose}>
      <div
        className="shortcuts-panel"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-label="键盘快捷键"
      >
        <div className="shortcuts-panel__header">
          <h3 className="shortcuts-panel__title">⌨️ 键盘快捷键</h3>
          <button
            className="shortcuts-panel__close"
            onClick={onClose}
            aria-label="关闭"
          >
            ✕
          </button>
        </div>

        <div className="shortcuts-panel__content">
          <div className="shortcuts-panel__list">
            {shortcuts.map((shortcut, index) => (
              <div key={index} className="shortcuts-panel__item">
                <span className="shortcuts-panel__description">
                  {shortcut.description}
                </span>
                <kbd className="shortcuts-panel__key">
                  {getShortcutDisplay(shortcut.key)}
                </kbd>
              </div>
            ))}
          </div>
        </div>

        <div className="shortcuts-panel__footer">
          <span className="shortcuts-panel__hint">
            按 <kbd>?</kbd> 显示此帮助
          </span>
        </div>
      </div>
    </div>
  )
}

/**
 * 快捷键帮助触发 hook
 */
export function useShortcutsHelp(
  shortcuts: KeyboardShortcut[]
): {
  isOpen: boolean
  open: () => void
  close: () => void
  toggle: () => void
} {
  const [isOpen, setIsOpen] = React.useState(false)

  const open = React.useCallback(() => setIsOpen(true), [])
  const close = React.useCallback(() => setIsOpen(false), [])
  const toggle = React.useCallback(() => setIsOpen(prev => !prev), [])

  // 注册 ? 键打开帮助
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (
        event.key === '?' &&
        !event.ctrlKey &&
        !event.metaKey &&
        !event.altKey
      ) {
        const target = event.target as HTMLElement
        const isInput =
          target.tagName === 'INPUT' ||
          target.tagName === 'TEXTAREA' ||
          target.isContentEditable

        if (!isInput) {
          event.preventDefault()
          toggle()
        }
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [toggle])

  return { isOpen, open, close, toggle }
}

export default KeyboardShortcutsPanel
