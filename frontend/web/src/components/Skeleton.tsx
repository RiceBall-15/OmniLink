import type { CSSProperties } from 'react'
import './Skeleton.css'

/**
 * 骨架屏组件属性
 */
interface SkeletonProps {
  /** 宽度 */
  width?: string | number
  /** 高度 */
  height?: string | number
  /** 形状：圆形或矩形 */
  variant?: 'text' | 'circular' | 'rectangular' | 'rounded'
  /** 是否显示动画 */
  animation?: 'pulse' | 'wave' | 'none'
  /** 自定义样式 */
  style?: CSSProperties
  /** 自定义类名 */
  className?: string
}

/**
 * 通用骨架屏组件
 * 用于内容加载时显示占位动画
 */
export function Skeleton({
  width,
  height,
  variant = 'text',
  animation = 'wave',
  style,
  className = '',
}: SkeletonProps) {
  const inlineStyle: CSSProperties = {
    width: width ?? (variant === 'circular' ? 40 : '100%'),
    height: height ?? (variant === 'text' ? 16 : variant === 'circular' ? 40 : 100),
    ...style,
  }

  return (
    <div
      className={`skeleton skeleton--${variant} skeleton--${animation} ${className}`}
      style={inlineStyle}
      aria-hidden="true"
    />
  )
}

/**
 * 消息列表骨架屏
 */
export function MessageListSkeleton({ count = 5 }: { count?: number }) {
  return (
    <div className="skeleton-message-list">
      {Array.from({ length: count }).map((_, i) => (
        <div
          key={i}
          className={`skeleton-message ${i % 2 === 0 ? 'skeleton-message--left' : 'skeleton-message--right'}`}
        >
          {i % 2 === 0 && <Skeleton variant="circular" width={36} height={36} />}
          <div className="skeleton-message__content">
            {i % 2 === 0 && <Skeleton width={60} height={12} style={{ marginBottom: 6 }} />}
            <Skeleton
              variant="rounded"
              width={150 + Math.random() * 100}
              height={36 + Math.random() * 20}
            />
          </div>
        </div>
      ))}
    </div>
  )
}

/**
 * 联系人列表骨架屏
 */
export function ContactListSkeleton({ count = 8 }: { count?: number }) {
  return (
    <div className="skeleton-contact-list">
      {Array.from({ length: count }).map((_, i) => (
        <div key={i} className="skeleton-contact">
          <Skeleton variant="circular" width={44} height={44} />
          <div className="skeleton-contact__info">
            <Skeleton width={80 + Math.random() * 40} height={14} />
            <Skeleton width={120 + Math.random() * 60} height={12} style={{ marginTop: 6 }} />
          </div>
          <Skeleton width={32} height={12} />
        </div>
      ))}
    </div>
  )
}

/**
 * 聊天页面骨架屏
 */
export function ChatPageSkeleton() {
  return (
    <div className="skeleton-chat-page">
      <div className="skeleton-chat-sidebar">
        <div className="skeleton-chat-sidebar__header">
          <Skeleton variant="rounded" width="100%" height={36} />
        </div>
        <ContactListSkeleton count={6} />
      </div>
      <div className="skeleton-chat-main">
        <div className="skeleton-chat-main__header">
          <Skeleton variant="circular" width={36} height={36} />
          <Skeleton width={100} height={16} />
        </div>
        <MessageListSkeleton count={6} />
        <div className="skeleton-chat-main__input">
          <Skeleton variant="rounded" width="100%" height={44} />
        </div>
      </div>
    </div>
  )
}

/**
 * 设置页面骨架屏
 */
export function SettingsPageSkeleton() {
  return (
    <div className="skeleton-settings">
      <Skeleton variant="rounded" width={140} height={28} style={{ marginBottom: 24 }} />
      {Array.from({ length: 4 }).map((_, i) => (
        <div key={i} className="skeleton-settings__item">
          <Skeleton width={80} height={14} />
          <Skeleton variant="rounded" width="100%" height={40} />
        </div>
      ))}
    </div>
  )
}
