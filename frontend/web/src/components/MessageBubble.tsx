import { useState, useRef } from 'react'
import { MessageEdit } from './MessageEdit'
import { MessageContextMenu } from './MessageContextMenu'
import { RecallConfirmDialog } from './RecallConfirmDialog'
import { MessageReadReceipt, type ReadUser } from './MessageReadReceipt'
import { ReadStatusIndicator } from './ReadStatusIndicator'
import './MessageBubble.css'
import './MessageEdit.css'
import './MessageContextMenu.css'
import './RecallConfirmDialog.css'
import type { Message } from '../types/message'
import { messageService } from '../services/messageService'

/**
 * 消息气泡组件属性
 */
interface MessageBubbleProps {
  /** 消息对象 */
  message: Message
  /** 是否为自己的消息 */
  isOwn: boolean
  /** 发送者显示名称 */
  senderName?: string
  /** 发送者头像 */
  senderAvatar?: string
  /** 当前用户ID */
  currentUserId: string
  /** 编辑时间限制（分钟） */
  editTimeLimit?: number
  /** 撤回时间限制（分钟） */
  recallTimeLimit?: number
  /** 消息更新回调 */
  onMessageUpdate?: (messageId: string, updates: Partial<Message>) => void
  /** 消息撤回回调 */
  onMessageRecall?: (messageId: string) => void
  /** 回复消息回调 */
  onReply?: (messageId: string) => void
  /** 是否为群聊 */
  isGroup?: boolean
  /** 已读用户列表 */
  readUsers?: ReadUser[]
}

/**
 * 消息气泡组件
 * 显示消息内容，支持：
 * - 双击进入编辑模式（仅自己的消息）
 * - 右键菜单（复制、编辑、撤回、回复）
 * - 编辑时间限制
 * - 消息状态显示
 */
export function MessageBubble({
  message,
  isOwn,
  senderName,
  senderAvatar,
  currentUserId,
  editTimeLimit = 2,
  recallTimeLimit = 2,
  onMessageUpdate,
  onMessageRecall,
  onReply,
  isGroup = false,
  readUsers = [],
}: MessageBubbleProps) {
  const [isEditing, setIsEditing] = useState(false)
  const [isSaving, setIsSaving] = useState(false)
  const [showContextMenu, setShowContextMenu] = useState(false)
  const [contextMenuPosition, setContextMenuPosition] = useState({ x: 0, y: 0 })
  const [showRecallDialog, setShowRecallDialog] = useState(false)

  const bubbleRef = useRef<HTMLDivElement>(null)

  /**
   * 检查是否可以编辑（自己的消息且在编辑时间内）
   */
  const canEdit = () => {
    if (!isOwn) return false

    const now = Date.now()
    const createdAt = new Date(message.createdAt).getTime()
    const timeDiffMinutes = (now - createdAt) / (1000 * 60)

    return timeDiffMinutes < editTimeLimit
  }

  /**
   * 检查是否可以撤回（自己的消息且在撤回时间内）
   */
  const canRecall = () => {
    if (!isOwn) return false

    const now = Date.now()
    const createdAt = new Date(message.createdAt).getTime()
    const timeDiffMinutes = (now - createdAt) / (1000 * 60)

    return timeDiffMinutes < recallTimeLimit
  }

  /**
   * 处理双击事件 - 进入编辑模式
   */
  const handleDoubleClick = () => {
    if (canEdit()) {
      setIsEditing(true)
      setShowContextMenu(false)
    }
  }

  /**
   * 处理右键菜单
   */
  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault()
    if (canEdit() || canRecall() || onReply) {
      setContextMenuPosition({ x: e.clientX, y: e.clientY })
      setShowContextMenu(true)
    }
  }

  /**
   * 关闭上下文菜单
   */
  const handleCloseContextMenu = () => {
    setShowContextMenu(false)
  }

  /**
   * 复制消息内容
   */
  const handleCopy = () => {
    navigator.clipboard.writeText(message.content)
    // 可以添加 toast 提示
  }

  /**
   * 进入编辑模式
   */
  const handleEdit = () => {
    if (canEdit()) {
      setIsEditing(true)
    }
  }

  /**
   * 撤回消息 - 显示确认对话框
   */
  const handleRecall = () => {
    if (!canRecall()) return
    setShowRecallDialog(true)
    setShowContextMenu(false)
  }

  /**
   * 确认撤回消息
   */
  const handleConfirmRecall = async () => {
    if (!canRecall()) return

    setIsSaving(true)
    try {
      await messageService.recallMessage(message.id)
      // 更新本地消息状态
      onMessageRecall?.(message.id)
      setShowRecallDialog(false)
    } catch (error) {
      console.error('撤回消息失败:', error)
      // 可以添加错误提示
    } finally {
      setIsSaving(false)
    }
  }

  /**
   * 取消撤回消息
   */
  const handleCancelRecall = () => {
    setShowRecallDialog(false)
  }

  /**
   * 回复消息
   */
  const handleReply = () => {
    onReply?.(message.id)
  }

  /**
   * 保存编辑
   */
  const handleSaveEdit = async (content: string) => {
    setIsSaving(true)
    try {
      const response = await messageService.editMessage(message.id, content)
      if (response.success && response.data) {
        // 更新本地消息状态
        onMessageUpdate?.(message.id, {
          content: response.data.content,
          updatedAt: response.data.updatedAt,
        })
        setIsEditing(false)
      }
    } catch (error) {
      console.error('编辑消息失败:', error)
      // 可以添加错误提示
    } finally {
      setIsSaving(false)
    }
  }

  /**
   * 取消编辑
   */
  const handleCancelEdit = () => {
    setIsEditing(false)
  }

  /**
   * 格式化消息时间
   */
  const formatTime = (dateString: string) => {
    const date = new Date(dateString)
    return date.toLocaleTimeString('zh-CN', {
      hour: '2-digit',
      minute: '2-digit',
    })
  }

  /**
   * 格式化撤回时间
   */
  const formatRecallTime = (dateString: string) => {
    const date = new Date(dateString)
    const now = new Date()
    const diffMs = now.getTime() - date.getTime()
    const diffMins = Math.floor(diffMs / 60000)

    if (diffMins < 1) {
      return '刚刚'
    } else if (diffMins < 60) {
      return `${diffMins}分钟前`
    } else if (diffMins < 1440) {
      const hours = Math.floor(diffMins / 60)
      return `${hours}小时前`
    } else {
      return date.toLocaleDateString('zh-CN', {
        month: '2-digit',
        day: '2-digit',
      })
    }
  }

  /**
   * 渲染发送者头像
   */
  const renderAvatar = () => {
    if (senderAvatar) {
      return (
        <div className="message-avatar">
          <img src={senderAvatar} alt={senderName || 'Avatar'} />
        </div>
      )
    }
    const initial = senderName ? senderName.charAt(0).toUpperCase() : '?'
    return <div className="message-avatar">{initial}</div>
  }

  return (
    <>
      <div
        ref={bubbleRef}
        className={`message-bubble ${isOwn ? 'own' : 'other'} ${message.isRecalled ? 'recalled' : ''}`}
        onDoubleClick={handleDoubleClick}
        onContextMenu={handleContextMenu}
        role="article"
        aria-label={`来自${isOwn ? '我' : senderName || '对方'}的消息`}
        style={{
          cursor: canEdit() || canRecall() ? 'pointer' : 'default',
        }}
      >
        {/* 非自己的消息显示头像 */}
        {!isOwn && renderAvatar()}

        <div className="message-content">
          {/* 非自己的消息显示名称 */}
          {!isOwn && senderName && (
            <div className="message-header">
              <span className="message-name">{senderName}</span>
            </div>
          )}

          {/* 消息内容或编辑组件 */}
          {message.isRecalled ? (
            // 撤回消息显示
            <div className="message-text message-recalled">
              <div className="message-recalled-content">
                <span className="message-recalled-icon">↩️</span>
                <span className="message-recalled-text">此消息已撤回</span>
              </div>
              {message.recalledAt && (
                <div className="message-recalled-time">
                  {formatRecallTime(message.recalledAt)}
                </div>
              )}
            </div>
          ) : isEditing ? (
            <MessageEdit
              initialContent={message.content}
              onSave={handleSaveEdit}
              onCancel={handleCancelEdit}
              isSaving={isSaving}
              editTimeLimit={editTimeLimit}
              messageCreatedAt={message.createdAt}
              disabled={!canEdit()}
            />
          ) : (
            <div className="message-text">
              {message.content}
            </div>
          )}

          {/* 消息元信息 */}
          {!isEditing && !message.isRecalled && (
            <div className="message-footer">
              <span className="message-time">{formatTime(message.createdAt)}</span>
              {/* 消息状态指示器 - 仅自己的消息显示 */}
              {isOwn && (
                <ReadStatusIndicator status={message.status} />
              )}
              {/* 已读回执 - 仅自己的消息显示 */}
              {isOwn && (
                <MessageReadReceipt
                  message={message}
                  isOwn={isOwn}
                  isGroup={isGroup}
                  readUsers={readUsers}
                />
              )}
              {/* 编辑标识 */}
              {message.updatedAt !== message.createdAt && (
                <span className="message-edited">(已编辑)</span>
              )}
            </div>
          )}
        </div>

        {/* 自己的消息显示头像 */}
        {isOwn && renderAvatar()}
      </div>

      {/* 右键菜单 */}
      {!message.isRecalled && (
        <MessageContextMenu
          visible={showContextMenu}
          position={contextMenuPosition}
          onClose={handleCloseContextMenu}
          onCopy={handleCopy}
          onEdit={handleEdit}
          onRecall={handleRecall}
          onReply={handleReply}
          message={message}
          currentUserId={currentUserId}
          senderId={message.senderId}
          canEdit={canEdit()}
          canRecall={canRecall()}
        />
      )}

      {/* 撤回确认对话框 */}
      <RecallConfirmDialog
        visible={showRecallDialog}
        onClose={handleCancelRecall}
        onConfirm={handleConfirmRecall}
        isRecalling={isSaving}
        recallTimeLimit={recallTimeLimit}
      />
    </>
  )
}
