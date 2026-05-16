import { useRef, useCallback, useState, useEffect, type ReactNode } from 'react'
import { useVirtualList } from '../hooks/useVirtualList'
import './VirtualList.css'

/**
 * 会话数据接口
 */
export interface ConversationItem {
  id: string
  name: string
  avatar?: string
  lastMessage?: {
    content: string
    createdAt: string
  }
  unreadCount: number
  type?: 'private' | 'group' | 'ai'
}

/**
 * VirtualConversationList 属性
 */
interface VirtualConversationListProps {
  /** 会话列表数据 */
  conversations: ConversationItem[]
  /** 选中的会话 ID */
  selectedId: string | null
  /** 点击会话回调 */
  onSelect: (id: string) => void
  /** 每项高度（像素） */
  itemHeight?: number
  /** 加载状态 */
  loading?: boolean
  /** 搜索过滤词 */
  searchQuery?: string
  /** 自定义渲染会话项 */
  renderItem?: (conv: ConversationItem, isSelected: boolean) => ReactNode
}

/**
 * 虚拟化会话列表组件
 * 支持大量会话的高性能滚动渲染
 */
export function VirtualConversationList({
  conversations,
  selectedId,
  onSelect,
  itemHeight = 72,
  loading = false,
  searchQuery = '',
  renderItem,
}: VirtualConversationListProps) {
  const containerRef = useRef<HTMLDivElement>(null)

  // 过滤会话
  const filteredConversations = conversations.filter(conv => {
    if (!searchQuery) return true
    const query = searchQuery.toLowerCase()
    return (
      conv.name.toLowerCase().includes(query) ||
      conv.lastMessage?.content?.toLowerCase().includes(query)
    )
  })

  const { startIndex, endIndex, totalHeight, scrollToIndex } = useVirtualList({
    itemHeight,
    itemCount: filteredConversations.length,
    containerRef,
    overscan: 5,
  })

  // 计算可见项
  const visibleItems = filteredConversations.slice(startIndex, endIndex + 1)

  // 格式化时间
  const formatTime = (dateStr?: string) => {
    if (!dateStr) return ''
    const date = new Date(dateStr)
    const now = new Date()
    const diffMs = now.getTime() - date.getTime()
    const diffMins = Math.floor(diffMs / 60000)
    const diffHours = Math.floor(diffMs / 3600000)
    const diffDays = Math.floor(diffMs / 86400000)

    if (diffMins < 1) return '刚刚'
    if (diffMins < 60) return `${diffMins}分钟前`
    if (diffHours < 24) return `${diffHours}小时前`
    if (diffDays < 7) return `${diffDays}天前`
    return date.toLocaleDateString('zh-CN', { month: 'numeric', day: 'numeric' })
  }

  // 获取会话类型图标
  const getTypeIcon = (type?: string) => {
    switch (type) {
      case 'group': return '👥'
      case 'ai': return '🤖'
      default: return '💬'
    }
  }

  if (loading) {
    return (
      <div className="virtual-list__loading">
        <div className="virtual-list__spinner" />
        <span>加载会话列表...</span>
      </div>
    )
  }

  if (filteredConversations.length === 0) {
    return (
      <div className="virtual-list__empty">
        <span className="virtual-list__empty-icon">💬</span>
        <p>{searchQuery ? '没有找到匹配的会话' : '暂无对话'}</p>
        {!searchQuery && <p className="virtual-list__empty-hint">点击"新建对话"开始聊天</p>}
      </div>
    )
  }

  return (
    <div
      ref={containerRef}
      className="virtual-list"
      role="listbox"
      aria-label="会话列表"
    >
      {/* 撑高容器 */}
      <div
        className="virtual-list__spacer"
        style={{ height: `${totalHeight}px` }}
      >
        {/* 渲染可见项 */}
        <div
          className="virtual-list__visible"
          style={{
            transform: `translateY(${startIndex * itemHeight}px)`,
          }}
        >
          {visibleItems.map((conv, idx) => {
            const isSelected = selectedId === conv.id
            const globalIndex = startIndex + idx

            if (renderItem) {
              return (
                <div key={conv.id} style={{ height: `${itemHeight}px` }}>
                  {renderItem(conv, isSelected)}
                </div>
              )
            }

            return (
              <div
                key={conv.id}
                className={`virtual-conv-item ${isSelected ? 'virtual-conv-item--selected' : ''}`}
                style={{ height: `${itemHeight}px` }}
                onClick={() => onSelect(conv.id)}
                role="option"
                aria-selected={isSelected}
                tabIndex={0}
              >
                <div className="virtual-conv-item__avatar">
                  {conv.avatar ? (
                    <img src={conv.avatar} alt={conv.name} loading="lazy" />
                  ) : (
                    <div className="virtual-conv-item__avatar-placeholder">
                      {conv.name?.charAt(0).toUpperCase() || '?'}
                    </div>
                  )}
                  {conv.type && conv.type !== 'private' && (
                    <span className="virtual-conv-item__type-badge">
                      {getTypeIcon(conv.type)}
                    </span>
                  )}
                </div>

                <div className="virtual-conv-item__content">
                  <div className="virtual-conv-item__header">
                    <span className="virtual-conv-item__name">{conv.name || '未命名对话'}</span>
                    <span className="virtual-conv-item__time">
                      {formatTime(conv.lastMessage?.createdAt)}
                    </span>
                  </div>
                  <div className="virtual-conv-item__preview">
                    {conv.lastMessage?.content || '暂无消息'}
                  </div>
                </div>

                {conv.unreadCount > 0 && (
                  <span className="virtual-conv-item__badge">
                    {conv.unreadCount > 99 ? '99+' : conv.unreadCount}
                  </span>
                )}
              </div>
            )
          })}
        </div>
      </div>
    </div>
  )
}
