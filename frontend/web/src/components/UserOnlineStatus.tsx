import type { OnlineStatus } from '../types/message'
import { OnlineStatusIndicator } from './OnlineStatusIndicator'
import './UserOnlineStatus.css'

/**
 * 用户在线状态组件属性
 */
interface UserOnlineStatusProps {
  /** 用户ID */
  userId: string
  /** 用户头像 */
  avatar?: string
  /** 用户昵称 */
  nickname: string
  /** 在线状态 */
  status: OnlineStatus
  /** 头像大小（像素） */
  avatarSize?: number
  /** 是否可点击 */
  clickable?: boolean
  /** 点击回调 */
  onClick?: (userId: string) => void
  /** 是否显示状态文字 */
  showStatusLabel?: boolean
  /** 是否紧凑模式（只显示头像和状态点） */
  compact?: boolean
}

/**
 * 用户在线状态组件
 * 显示用户头像 + 在线状态点 + 用户昵称
 */
export function UserOnlineStatus({
  userId,
  avatar,
  nickname,
  status,
  avatarSize = 40,
  clickable = false,
  onClick,
  showStatusLabel = false,
  compact = false,
}: UserOnlineStatusProps) {
  const handleClick = () => {
    if (clickable && onClick) {
      onClick(userId)
    }
  }

  return (
    <div
      className={`user-online-status ${clickable ? 'clickable' : ''} ${compact ? 'compact' : ''}`}
      onClick={handleClick}
    >
      <div
        className="user-avatar-wrapper"
        style={{ width: `${avatarSize}px`, height: `${avatarSize}px` }}
      >
        {avatar ? (
          <img
            src={avatar}
            alt={nickname}
            className="user-avatar"
            style={{ width: `${avatarSize}px`, height: `${avatarSize}px` }}
          />
        ) : (
          <div
            className="user-avatar-placeholder"
            style={{ width: `${avatarSize}px`, height: `${avatarSize}px` }}
          >
            {nickname.charAt(0).toUpperCase()}
          </div>
        )}
        <div className="user-status-indicator">
          <OnlineStatusIndicator status={status} size={8} />
        </div>
      </div>

      {!compact && (
        <div className="user-info">
          <div className="user-nickname">{nickname}</div>
          {showStatusLabel && (
            <div className="user-status-text">
              <OnlineStatusIndicator status={status} showLabel={true} size={6} />
            </div>
          )}
        </div>
      )}
    </div>
  )
}
