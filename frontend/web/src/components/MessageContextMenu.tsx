import { useEffect, useRef, useState } from 'react'
import type { Message } from '../types/message'

/**
 * 消息上下文菜单项类型
 */
export type ContextMenuItem = 'copy' | 'edit' | 'recall' | 'reply'

/**
 * 消息上下文菜单属性
 */
interface MessageContextMenuProps {
  /** 消息对象 */
  message: Message
  /** 是否显示菜单 */
  visible: boolean
  /** 菜单位置 */
  position: { x: number; y: number }
  /** 关闭菜单回调 */
  onClose: () => void
  /** 复制回调 */
  onCopy: () => void
  /** 编辑回调 */
  onEdit: () => void
  /** 撤回回调 */
  onRecall: () => void
  /** 回复回调 */
  onReply: () => void
  /** 当前用户ID（用于权限判断） */
  currentUserId: string
  /** 消息发送者ID */
  senderId: string
  /** 编辑是否可用（时间限制等） */
  canEdit?: boolean
  /** 撤回是否可用（时间限制等） */
  canRecall?: boolean
}

/**
 * 消息右键菜单组件
 * 提供消息操作功能：
 * - 复制：复制消息内容
 * - 编辑：编辑消息（仅自己的消息且在编辑时间内）
 * - 撤回：撤回消息（仅自己的消息且在撤回时间内）
 * - 回复：回复消息
 */
export function MessageContextMenu({
  visible,
  position,
  onClose,
  onCopy,
  onEdit,
  onRecall,
  onReply,
  currentUserId,
  senderId,
  canEdit = false,
  canRecall = false,
}: MessageContextMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null)

  /**
   * 计算菜单显示位置
   * 防止菜单超出视口
   */
  const [menuPosition, setMenuPosition] = useState({ x: 0, y: 0 })

  useEffect(() => {
    if (!visible || !menuRef.current) return

    const menu = menuRef.current
    const rect = menu.getBoundingClientRect()
    const viewportWidth = window.innerWidth
    const viewportHeight = window.innerHeight

    let x = position.x
    let y = position.y

    // 防止超出右边界
    if (x + rect.width > viewportWidth - 10) {
      x = position.x - rect.width
    }

    // 防止超出底部边界
    if (y + rect.height > viewportHeight - 10) {
      y = position.y - rect.height
    }

    // 防止超出左边界
    if (x < 10) {
      x = 10
    }

    // 防止超出顶部边界
    if (y < 10) {
      y = 10
    }

    setMenuPosition({ x, y })
  }, [visible, position])

  /**
   * 点击外部关闭菜单
   */
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        onClose()
      }
    }

    const handleScroll = () => {
      onClose()
    }

    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onClose()
      }
    }

    if (visible) {
      document.addEventListener('mousedown', handleClickOutside)
      window.addEventListener('scroll', handleScroll, true)
      document.addEventListener('keydown', handleEscape)

      return () => {
        document.removeEventListener('mousedown', handleClickOutside)
        window.removeEventListener('scroll', handleScroll, true)
        document.removeEventListener('keydown', handleEscape)
      }
    }
  }, [visible, onClose])

  /**
   * 判断是否为自己的消息
   */
  const isOwnMessage = currentUserId === senderId

  /**
   * 处理菜单项点击
   */
  const handleItemClick = (action: ContextMenuItem) => {
    switch (action) {
      case 'copy':
        onCopy()
        break
      case 'edit':
        if (canEdit) {
          onEdit()
        }
        break
      case 'recall':
        if (canRecall) {
          onRecall()
        }
        break
      case 'reply':
        onReply()
        break
    }
    onClose()
  }

  /**
   * 复制消息内容到剪贴板
   */
  const handleCopy = () => {
    handleItemClick('copy')
  }

  if (!visible) return null

  return (
    <div
      ref={menuRef}
      className="message-context-menu"
      style={{
        position: 'fixed',
        left: `${menuPosition.x}px`,
        top: `${menuPosition.y}px`,
        zIndex: 1000,
      }}
      role="menu"
      aria-label="消息操作菜单"
    >
      <div className="message-context-menu-item" onClick={handleCopy} role="menuitem" tabIndex={0}>
        <span className="message-context-menu-icon">📋</span>
        <span className="message-context-menu-text">复制</span>
      </div>

      {/* 编辑：仅自己的消息且在编辑时间内 */}
      {isOwnMessage && canEdit && (
        <div
          className="message-context-menu-item"
          onClick={() => handleItemClick('edit')}
          role="menuitem"
          tabIndex={0}
        >
          <span className="message-context-menu-icon">✏️</span>
          <span className="message-context-menu-text">编辑</span>
        </div>
      )}

      {/* 撤回：仅自己的消息且在撤回时间内 */}
      {isOwnMessage && canRecall && (
        <>
          <div className="message-context-menu-divider" />
          <div
            className="message-context-menu-item message-context-menu-item-danger"
            onClick={() => handleItemClick('recall')}
            role="menuitem"
            tabIndex={0}
          >
            <span className="message-context-menu-icon">↩️</span>
            <span className="message-context-menu-text">撤回</span>
          </div>
        </>
      )}

      {/* 回复：所有消息都可以回复 */}
      <div className="message-context-menu-divider" />
      <div
        className="message-context-menu-item"
        onClick={() => handleItemClick('reply')}
        role="menuitem"
        tabIndex={0}
      >
        <span className="message-context-menu-icon">↩️</span>
        <span className="message-context-menu-text">回复</span>
      </div>
    </div>
  )
}
