import { Modal } from './Modal'
import type { ReadUser } from './MessageReadReceipt'
import type { Message } from '../types/message'
import './ReadUsersModal.css'

/**
 * 已读用户列表弹窗组件属性
 */
interface ReadUsersModalProps {
  /** 是否显示弹窗 */
  isOpen: boolean
  /** 关闭回调 */
  onClose: () => void
  /** 已读用户列表 */
  readUsers: ReadUser[]
  /** 消息对象 */
  message: Message
}

/**
 * 格式化相对时间
 */
function formatRelativeTime(dateString: string): string {
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
 * 格式化绝对时间
 */
function formatAbsoluteTime(dateString: string): string {
  const date = new Date(dateString)
  return date.toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })
}

/**
 * 已读用户列表弹窗组件
 * 显示已读该消息的所有用户及其已读时间
 */
export function ReadUsersModal({ isOpen, onClose, readUsers, message }: ReadUsersModalProps) {
  /**
   * 渲染已读用户头像
   */
  const renderAvatar = (user: ReadUser) => {
    if (user.avatar) {
      return <img src={user.avatar} alt={user.name} className="user-avatar" />
    }
    const initial = user.name.charAt(0).toUpperCase()
    return <div className="user-avatar-placeholder">{initial}</div>
  }

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="已读列表" size="md">
      <div className="read-users-modal">
        {/* 消息预览 */}
        <div className="message-preview">
          <div className="message-preview-label">消息内容</div>
          <div className="message-preview-content">{message.content}</div>
        </div>

        {/* 已读用户列表 */}
        <div className="read-users-list">
          <div className="list-header">
            <span className="read-count">已读 {readUsers.length} 人</span>
            {message.readAt && (
              <span className="last-read-time">
                最新: {formatRelativeTime(message.readAt)}
              </span>
            )}
          </div>

          {readUsers.length === 0 ? (
            <div className="empty-state">
              <div className="empty-icon">📭</div>
              <div className="empty-text">暂无已读用户</div>
            </div>
          ) : (
            <div className="users-container">
              {readUsers.map((user) => (
                <div key={user.id} className="user-item">
                  <div className="user-info">
                    <div className="user-avatar-wrapper">{renderAvatar(user)}</div>
                    <div className="user-details">
                      <div className="user-name">{user.name}</div>
                      <div className="read-time" title={formatAbsoluteTime(user.readAt)}>
                        {formatRelativeTime(user.readAt)}
                      </div>
                    </div>
                  </div>
                  <div className="read-indicator">
                    <svg viewBox="0 0 24 24" className="read-icon">
                      <path
                        d="M18 7l-1.41-1.41-6.34 6.34 1.41 1.41L18 7zm4.24-1.41L11.66 16.17 7.48 12l-1.41 1.41L11.66 19l12-12-1.42-1.41zM.41 13.41L6 19l1.41-1.41L1.83 12 .41 13.41z"
                        fill="currentColor"
                      />
                    </svg>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* 底部提示 */}
        {readUsers.length > 0 && (
          <div className="modal-footer-hint">
            💡 消息已实时同步，已读状态会自动更新
          </div>
        )}
      </div>
    </Modal>
  )
}
