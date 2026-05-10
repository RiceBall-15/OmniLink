import { useState } from 'react'
import { ReadStatusIndicator } from './ReadStatusIndicator'
import { ReadUsersModal } from './ReadUsersModal'
import type { Message } from '../types/message'
import './MessageReadReceipt.css'

/**
 * 已读用户信息接口
 */
export interface ReadUser {
  /** 用户ID */
  id: string
  /** 用户昵称 */
  name: string
  /** 用户头像 */
  avatar?: string
  /** 已读时间 */
  readAt: string
}

/**
 * 消息已读回执组件属性
 */
interface MessageReadReceiptProps {
  /** 消息对象 */
  message: Message
  /** 是否为自己的消息 */
  isOwn: boolean
  /** 是否为群聊 */
  isGroup?: boolean
  /** 已读用户列表 */
  readUsers?: ReadUser[]
  /** 点击已读状态时的回调（群聊场景） */
  onReadStatusClick?: () => void
}

/**
 * 消息已读回执组件
 * 显示消息的已读状态，支持悬停显示详细信息和点击查看已读用户列表
 */
export function MessageReadReceipt({
  message,
  isOwn,
  isGroup = false,
  readUsers = [],
  onReadStatusClick,
}: MessageReadReceiptProps) {
  const [showReadModal, setShowReadModal] = useState(false)
  const [showTooltip, setShowTooltip] = useState(false)

  /**
   * 处理已读状态点击
   */
  const handleReadStatusClick = () => {
    if (isGroup && readUsers.length > 0) {
      // 群聊场景，显示已读用户列表
      setShowReadModal(true)
      onReadStatusClick?.()
    }
  }

  /**
   * 处理鼠标进入
   */
  const handleMouseEnter = () => {
    // 仅在已读状态时显示提示
    if (message.status === 'read') {
      setShowTooltip(true)
    }
  }

  /**
   * 处理鼠标离开
   */
  const handleMouseLeave = () => {
    setShowTooltip(false)
  }

  /**
   * 格式化已读时间
   */
  const formatReadTime = (dateString?: string) => {
    if (!dateString) return ''

    const date = new Date(dateString)
    const now = new Date()
    const diffMs = now.getTime() - date.getTime()
    const diffMins = Math.floor(diffMs / 60000)

    if (diffMins < 1) {
      return '刚刚'
    } else if (diffMins < 60) {
      return `${diffMins}分钟前已读`
    } else if (diffMins < 1440) {
      const hours = Math.floor(diffMins / 60)
      return `${hours}小时前已读`
    } else {
      return date.toLocaleDateString('zh-CN', {
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
      })
    }
  }

  // 非自己的消息不显示已读回执
  if (!isOwn) {
    return null
  }

  return (
    <>
      <div
        className="message-read-receipt"
        onMouseEnter={handleMouseEnter}
        onMouseLeave={handleMouseLeave}
      >
        {/* 已读状态指示器 */}
        <ReadStatusIndicator
          status={message.status}
          onClick={handleReadStatusClick}
        />

        {/* 悬停提示 */}
        {showTooltip && message.status === 'read' && (
          <div className="read-receipt-tooltip">
            {isGroup ? (
              <>
                <div className="tooltip-header">
                  <span className="read-count">{readUsers.length}人已读</span>
                  {message.readAt && (
                    <span className="read-time">{formatReadTime(message.readAt)}</span>
                  )}
                </div>
                {/* 显示最多3个已读用户 */}
                {readUsers.length > 0 && (
                  <div className="tooltip-users">
                    {readUsers.slice(0, 3).map((user) => (
                      <div key={user.id} className="tooltip-user">
                        {user.avatar ? (
                          <img src={user.avatar} alt={user.name} className="user-avatar" />
                        ) : (
                          <div className="user-avatar-placeholder">{user.name.charAt(0)}</div>
                        )}
                        <span className="user-name">{user.name}</span>
                      </div>
                    ))}
                    {readUsers.length > 3 && (
                      <span className="more-users">等{readUsers.length}人</span>
                    )}
                  </div>
                )}
                {readUsers.length > 0 && (
                  <div className="tooltip-footer">点击查看详情</div>
                )}
              </>
            ) : (
              <div className="tooltip-single">
                {message.readAt && formatReadTime(message.readAt)}
              </div>
            )}
          </div>
        )}
      </div>

      {/* 已读用户列表弹窗 */}
      <ReadUsersModal
        isOpen={showReadModal}
        onClose={() => setShowReadModal(false)}
        readUsers={readUsers}
        message={message}
      />
    </>
  )
}
