import { useRef, useEffect, useState, useCallback, type UIEvent } from 'react'
import type { Message } from '../types/message'
import { MessageBubble } from './MessageBubble'
import './MessageList.css'

/**
 * MessageList 属性
 */
interface MessageListProps {
  /** 消息列表 */
  messages: Message[]
  /** 当前用户 ID */
  currentUserId: string
  /** 是否正在加载更多 */
  loadingMore?: boolean
  /** 是否还有更多历史消息 */
  hasMore?: boolean
  /** 加载更多回调 */
  onLoadMore?: () => void
  /** 新消息通知（用于自动滚动） */
  newMessageCount?: number
}

/**
 * 日期分隔符
 */
function DateSeparator({ date }: { date: string }) {
  const d = new Date(date)
  const now = new Date()
  const diffDays = Math.floor((now.getTime() - d.getTime()) / 86400000)

  let label: string
  if (diffDays === 0) label = '今天'
  else if (diffDays === 1) label = '昨天'
  else if (diffDays < 7) label = `${diffDays}天前`
  else label = d.toLocaleDateString('zh-CN', { month: 'long', day: 'numeric' })

  return (
    <div className="message-list__date-separator">
      <span>{label}</span>
    </div>
  )
}

/**
 * 加载更多指示器
 */
function LoadMoreIndicator({ loading }: { loading: boolean }) {
  return (
    <div className="message-list__load-more">
      {loading ? (
        <div className="message-list__load-spinner">
          <div className="message-list__spinner" />
          <span>加载历史消息...</span>
        </div>
      ) : (
        <span className="message-list__load-hint">上拉加载更多</span>
      )}
    </div>
  )
}

/**
 * 消息列表组件
 * 支持无限滚动加载、日期分隔、自动滚动到底部
 */
export function MessageList({
  messages,
  currentUserId,
  loadingMore = false,
  hasMore = true,
  onLoadMore,
  newMessageCount = 0,
}: MessageListProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const bottomRef = useRef<HTMLDivElement>(null)
  const [isAtBottom, setIsAtBottom] = useState(true)
  const [showScrollButton, setShowScrollButton] = useState(false)
  const prevMessageCountRef = useRef(messages.length)
  const isLoadingMoreRef = useRef(false)

  // 检测是否在底部
  const checkIfAtBottom = useCallback(() => {
    const el = containerRef.current
    if (!el) return true
    const threshold = 100
    return el.scrollHeight - el.scrollTop - el.clientHeight < threshold
  }, [])

  // 滚动到底部
  const scrollToBottom = useCallback((smooth = true) => {
    bottomRef.current?.scrollIntoView({
      behavior: smooth ? 'smooth' : 'instant',
    })
  }, [])

  // 滚动事件处理
  const handleScroll = useCallback(
    (e: UIEvent<HTMLDivElement>) => {
      const el = e.currentTarget
      const atBottom = checkIfAtBottom()
      setIsAtBottom(atBottom)
      setShowScrollButton(!atBottom && messages.length > 0)

      // 检测是否需要加载更多（滚动到顶部附近）
      if (el.scrollTop < 100 && hasMore && !loadingMore && onLoadMore && !isLoadingMoreRef.current) {
        isLoadingMoreRef.current = true
        // 保存当前滚动位置
        const prevScrollHeight = el.scrollHeight
        const prevScrollTop = el.scrollTop

        onLoadMore()

        // 加载完成后恢复滚动位置
        requestAnimationFrame(() => {
          const newScrollHeight = el.scrollHeight
          el.scrollTop = prevScrollTop + (newScrollHeight - prevScrollHeight)
          isLoadingMoreRef.current = false
        })
      }
    },
    [checkIfAtBottom, hasMore, loadingMore, onLoadMore, messages.length]
  )

  // 新消息自动滚动
  useEffect(() => {
    if (messages.length > prevMessageCountRef.current && isAtBottom) {
      scrollToBottom()
    }
    prevMessageCountRef.current = messages.length
  }, [messages.length, isAtBottom, scrollToBottom])

  // 首次加载滚动到底部
  useEffect(() => {
    if (messages.length > 0 && prevMessageCountRef.current === 0) {
      scrollToBottom(false)
    }
  }, [messages.length, scrollToBottom])

  // 按日期分组消息
  const groupedMessages: Array<{ date: string; messages: Message[] }> = []
  let currentDate = ''

  for (const msg of messages) {
    const msgDate = new Date(msg.createdAt).toDateString()
    if (msgDate !== currentDate) {
      currentDate = msgDate
      groupedMessages.push({ date: msg.createdAt, messages: [msg] })
    } else {
      groupedMessages[groupedMessages.length - 1].messages.push(msg)
    }
  }

  return (
    <div className="message-list-wrapper">
      <div
        ref={containerRef}
        className="message-list"
        onScroll={handleScroll}
      >
        {/* 加载更多指示器 */}
        {hasMore && <LoadMoreIndicator loading={loadingMore} />}

        {/* 无更多消息提示 */}
        {!hasMore && messages.length > 0 && (
          <div className="message-list__no-more">
            <span>— 没有更早的消息了 —</span>
          </div>
        )}

        {/* 消息列表 */}
        {messages.length === 0 ? (
          <div className="message-list__empty">
            <div className="message-list__empty-icon">💬</div>
            <p>开始对话吧！</p>
            <p className="message-list__empty-hint">发送第一条消息</p>
          </div>
        ) : (
          groupedMessages.map(group => (
            <div key={group.date}>
              <DateSeparator date={group.date} />
              {group.messages.map(message => (
                <MessageBubble
                  key={message.id}
                  message={message}
                  isOwn={message.senderId === currentUserId}
                  currentUserId={currentUserId}
                />
              ))}
            </div>
          ))
        )}

        {/* 锚点元素 */}
        <div ref={bottomRef} className="message-list__bottom-anchor" />
      </div>

      {/* 回到底部按钮 */}
      {showScrollButton && (
        <button
          className="message-list__scroll-button"
          onClick={() => scrollToBottom()}
          aria-label="回到底部"
        >
          <span>↓</span>
          {newMessageCount > 0 && (
            <span className="message-list__new-badge">{newMessageCount}</span>
          )}
        </button>
      )}
    </div>
  )
}
