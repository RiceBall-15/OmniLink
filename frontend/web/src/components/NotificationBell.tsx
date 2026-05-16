/**
 * NotificationBell 组件
 * 通知铃铛图标，显示未读数量，点击展开通知面板
 */

import React, { useState, useRef, useEffect, useCallback } from 'react'
import { useNotification } from '../hooks/useNotification'
import { useClickOutside } from '../hooks/useClickOutside'
import './NotificationBell.css'

interface NotificationBellProps {
  /** 自定义类名 */
  className?: string
  /** 最大显示通知数量 */
  maxItems?: number
  /** 点击通知时的回调 */
  onNotificationClick?: (notificationId: string) => void
  /** 查看所有通知时的回调 */
  onViewAll?: () => void
}

export const NotificationBell: React.FC<NotificationBellProps> = ({
  className = '',
  maxItems = 5,
  onNotificationClick,
  onViewAll,
}) => {
  const [isOpen, setIsOpen] = useState(false)
  const panelRef = useRef<HTMLDivElement>(null)
  const bellRef = useRef<HTMLButtonElement>(null)

  const {
    notifications,
    unreadCount,
    markAsRead,
    markAllAsRead,
    clearNotification,
  } = useNotification()

  // 点击外部关闭面板
  useClickOutside(panelRef, () => {
    if (isOpen) {
      setIsOpen(false)
    }
  }, [bellRef])

  // ESC 键关闭面板
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && isOpen) {
        setIsOpen(false)
      }
    }

    if (isOpen) {
      window.addEventListener('keydown', handleKeyDown)
      return () => window.removeEventListener('keydown', handleKeyDown)
    }
  }, [isOpen])

  // 切换面板
  const togglePanel = useCallback(() => {
    setIsOpen(prev => !prev)
  }, [])

  // 点击通知
  const handleNotificationClick = useCallback(
    (notificationId: string) => {
      markAsRead(notificationId)
      onNotificationClick?.(notificationId)
    },
    [markAsRead, onNotificationClick]
  )

  // 删除通知
  const handleDeleteNotification = useCallback(
    (e: React.MouseEvent, notificationId: string) => {
      e.stopPropagation()
      clearNotification(notificationId)
    },
    [clearNotification]
  )

  // 全部标为已读
  const handleMarkAllAsRead = useCallback(() => {
    markAllAsRead()
  }, [markAllAsRead])

  // 获取通知类型图标
  const getNotificationIcon = (type: string): string => {
    switch (type) {
      case 'message':
        return '💬'
      case 'mention':
        return '@'
      case 'system':
        return '🔔'
      case 'friend_request':
        return '👤'
      case 'group_invite':
        return '👥'
      default:
        return '📌'
    }
  }

  // 格式化时间
  const formatTime = (timestamp: number): string => {
    const now = Date.now()
    const diff = now - timestamp

    if (diff < 60000) return '刚刚'
    if (diff < 3600000) return `${Math.floor(diff / 60000)}分钟前`
    if (diff < 86400000) return `${Math.floor(diff / 3600000)}小时前`
    return `${Math.floor(diff / 86400000)}天前`
  }

  // 显示的通知列表
  const displayNotifications = notifications.slice(0, maxItems)
  const hasMore = notifications.length > maxItems

  return (
    <div className={`notification-bell ${className}`}>
      {/* 铃铛按钮 */}
      <button
        ref={bellRef}
        className="notification-bell__trigger"
        onClick={togglePanel}
        aria-label={`通知${unreadCount > 0 ? ` (${unreadCount}条未读)` : ''}`}
        aria-expanded={isOpen}
        aria-haspopup="true"
      >
        <svg
          className="notification-bell__icon"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9" />
          <path d="M13.73 21a2 2 0 0 1-3.46 0" />
        </svg>

        {/* 未读数量徽章 */}
        {unreadCount > 0 && (
          <span className="notification-bell__badge">
            {unreadCount > 99 ? '99+' : unreadCount}
          </span>
        )}
      </button>

      {/* 通知面板 */}
      {isOpen && (
        <div ref={panelRef} className="notification-bell__panel" role="dialog" aria-label="通知列表">
          {/* 面板头部 */}
          <div className="notification-bell__header">
            <h3 className="notification-bell__title">通知</h3>
            {unreadCount > 0 && (
              <button
                className="notification-bell__mark-all"
                onClick={handleMarkAllAsRead}
              >
                全部已读
              </button>
            )}
          </div>

          {/* 通知列表 */}
          <div className="notification-bell__list">
            {displayNotifications.length === 0 ? (
              <div className="notification-bell__empty">
                <span className="notification-bell__empty-icon">🔔</span>
                <span className="notification-bell__empty-text">暂无通知</span>
              </div>
            ) : (
              displayNotifications.map(notification => (
                <div
                  key={notification.id}
                  className={`notification-bell__item ${
                    !notification.read ? 'notification-bell__item--unread' : ''
                  }`}
                  onClick={() => handleNotificationClick(notification.id)}
                >
                  <div className="notification-bell__item-icon">
                    {getNotificationIcon(notification.type)}
                  </div>

                  <div className="notification-bell__item-content">
                    <div className="notification-bell__item-title">
                      {notification.title}
                    </div>
                    <div className="notification-bell__item-message">
                      {notification.message}
                    </div>
                    <div className="notification-bell__item-time">
                      {formatTime(notification.timestamp)}
                    </div>
                  </div>

                  <button
                    className="notification-bell__item-delete"
                    onClick={(e) => handleDeleteNotification(e, notification.id)}
                    aria-label="删除通知"
                  >
                    ×
                  </button>
                </div>
              ))
            )}
          </div>

          {/* 面板底部 */}
          {hasMore && (
            <div className="notification-bell__footer">
              <button
                className="notification-bell__view-all"
                onClick={() => {
                  setIsOpen(false)
                  onViewAll?.()
                }}
              >
                查看全部通知 ({notifications.length})
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  )
}

export default NotificationBell
