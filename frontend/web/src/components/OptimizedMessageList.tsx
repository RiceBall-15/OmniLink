/**
 * 优化的消息列表组件
 * 使用虚拟滚动提高长列表性能
 */

import React, { useRef, useEffect, useCallback } from 'react';
import type { Message } from '../types/message';
import { MessageBubble } from './MessageBubble';
import { VirtualScroll } from './VirtualScroll';

interface OptimizedMessageListProps {
  messages: Message[];
  currentUserId: string;
  containerHeight?: number;
  itemHeight?: number;
  onLoadMore?: () => void;
  loading?: boolean;
}

export function OptimizedMessageList({
  messages,
  currentUserId,
  containerHeight = 600,
  itemHeight = 80,
  onLoadMore,
  loading = false,
}: OptimizedMessageListProps) {
  const listRef = useRef<HTMLDivElement>(null);
  const shouldAutoScroll = useRef(true);

  // 检查是否应该自动滚动到底部
  const checkAutoScroll = useCallback(() => {
    if (!listRef.current) return;

    const { scrollTop, scrollHeight, clientHeight } = listRef.current;
    const isNearBottom = scrollHeight - scrollTop - clientHeight < 100;
    shouldAutoScroll.current = isNearBottom;
  }, []);

  // 滚动到底部
  const scrollToBottom = useCallback(() => {
    if (!listRef.current || !shouldAutoScroll.current) return;

    listRef.current.scrollTop = listRef.current.scrollHeight;
  }, []);

  // 新消息到达时自动滚动
  useEffect(() => {
    scrollToBottom();
  }, [messages.length, scrollToBottom]);

  // 渲染单个消息
  const renderMessage = useCallback(
    (message: Message, index: number) => {
      const isOwn = message.senderId === currentUserId;
      const showAvatar =
        index === 0 ||
        messages[index - 1]?.senderId !== message.senderId;

      return (
        <div className="message-item" style={{ padding: '4px 16px' }}>
          <MessageBubble
            message={message}
            isOwn={isOwn}
            currentUserId={currentUserId}
            showAvatar={showAvatar}
          />
        </div>
      );
    },
    [currentUserId, messages]
  );

  // 处理滚动事件
  const handleScroll = useCallback(
    (e: React.UIEvent<HTMLDivElement>) => {
      checkAutoScroll();

      // 检查是否需要加载更多
      if (!onLoadMore || loading) return;

      const { scrollTop } = e.currentTarget;
      if (scrollTop < 100) {
        onLoadMore();
      }
    },
    [checkAutoScroll, onLoadMore, loading]
  );

  if (messages.length === 0) {
    return (
      <div className="empty-messages">
        <div className="empty-icon">💬</div>
        <p>开始对话吧！</p>
        <p className="empty-hint">发送第一条消息</p>
      </div>
    );
  }

  return (
    <div
      ref={listRef}
      className="optimized-message-list"
      onScroll={handleScroll}
      style={{ height: containerHeight, overflow: 'auto' }}
    >
      {/* 加载更多指示器 */}
      {loading && (
        <div className="loading-indicator">
          <div className="spinner"></div>
          <span>加载中...</span>
        </div>
      )}

      {/* 虚拟滚动列表 */}
      <VirtualScroll
        items={messages}
        itemHeight={itemHeight}
        containerHeight={containerHeight}
        renderItem={renderMessage}
        overscan={10}
        onEndReached={onLoadMore}
        endReachedThreshold={200}
      />
    </div>
  );
}

export default OptimizedMessageList;
