import { useState, useEffect, useRef } from 'react'
import { useAuth } from '../hooks/useAuth'
import { useConversations, useMessages, useWebSocket } from '../hooks/useMessages'
import { MessageList } from '../components/MessageList'
import { AuthPage } from './AuthPage'
import './ChatPage.css'

export function ChatPage() {
  const { user, logout } = useAuth()
  const { conversations, loading: conversationsLoading } = useConversations()
  const [selectedConversation, setSelectedConversation] = useState<string | null>(null)
  const [sidebarOpen, setSidebarOpen] = useState(true)
  const [inputMessage, setInputMessage] = useState('')
  const messagesEndRef = useRef<HTMLDivElement>(null)

  // WebSocket连接
  const { connected, sendMessage: sendWsMessage } = useWebSocket(
    import.meta.env.VITE_WS_URL || 'ws://localhost:8001',
    (data) => {
      console.log('收到WebSocket消息:', data)
      // 处理新消息
      if (data.type === 'new_message') {
        // 刷新消息列表
      }
    }
  )

  // 消息管理
  const { messages, loading: messagesLoading, sendMessage: sendApiMessage } = useMessages(
    selectedConversation || ''
  )

  // 自动滚动到底部
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages])

  if (!user) {
    return <AuthPage />
  }

  const handleSendMessage = async () => {
    if (!inputMessage.trim() || !selectedConversation) return

    try {
      await sendApiMessage(inputMessage)
      setInputMessage('')

      // 通过WebSocket发送消息
      sendWsMessage({
        type: 'message',
        conversationId: selectedConversation,
        content: inputMessage,
      })
    } catch (error) {
      console.error('发送消息失败:', error)
    }
  }

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSendMessage()
    }
  }

  const selectedConv = conversations.find((c) => c.id === selectedConversation)

  return (
    <div className="chat-container">
      {/* 侧边栏 */}
      <div className={`sidebar ${sidebarOpen ? 'open' : 'closed'}`}>
        <div className="sidebar-header">
          <h2 className="app-title">💬 OmniLink</h2>
          <button
            className="new-chat-button"
            onClick={() => console.log('新建对话')}
          >
            + 新建对话
          </button>
        </div>

        <div className="user-info">
          <div className="user-avatar">{user.username.charAt(0).toUpperCase()}</div>
          <div className="user-details">
            <div className="user-name">{user.username}</div>
            <div className="user-email">{user.email}</div>
            <div className={`connection-status ${connected ? 'online' : 'offline'}`}>
              {connected ? '● 已连接' : '○ 未连接'}
            </div>
          </div>
          <button className="logout-button" onClick={logout} title="退出登录">
            🚪
          </button>
        </div>

        <div className="conversations-list">
          {conversationsLoading ? (
            <div className="loading">加载中...</div>
          ) : conversations.length === 0 ? (
            <div className="empty-state">
              <p>暂无对话</p>
              <p className="empty-hint">点击"新建对话"开始聊天</p>
            </div>
          ) : (
            conversations.map((conv) => (
              <div
                key={conv.id}
                className={`conversation-item ${selectedConversation === conv.id ? 'active' : ''}`}
                onClick={() => setSelectedConversation(conv.id)}
              >
                <div className="conversation-avatar">
                  {conv.avatar ? (
                    <img src={conv.avatar} alt={conv.name} />
                  ) : (
                    <div className="avatar-placeholder">
                      {conv.name?.charAt(0).toUpperCase() || 'AI'}
                    </div>
                  )}
                </div>
                <div className="conversation-info">
                  <div className="conversation-name">{conv.name || '未命名对话'}</div>
                  <div className="conversation-preview">
                    {conv.lastMessage?.content || '暂无消息'}
                  </div>
                </div>
                {conv.unreadCount > 0 && (
                  <div className="unread-badge">{conv.unreadCount}</div>
                )}
              </div>
            ))
          )}
        </div>
      </div>

      {/* 主聊天区域 */}
      <div className="chat-main">
        {selectedConversation ? (
          <div className="chat-content">
            <div className="chat-header">
              <h3>{selectedConv?.name || '对话'}</h3>
            </div>

            <div className="chat-messages">
              {messagesLoading ? (
                <div className="loading">加载消息中...</div>
              ) : (
                <>
                  <MessageList messages={messages} currentUserId={user.id} />
                  <div ref={messagesEndRef} />
                </>
              )}
            </div>

            <div className="chat-input">
              <textarea
                value={inputMessage}
                onChange={(e) => setInputMessage(e.target.value)}
                onKeyDown={handleKeyPress}
                placeholder="输入消息... (Enter发送)"
                rows={1}
                className="message-textarea"
              />
              <button
                className="send-button"
                onClick={handleSendMessage}
                disabled={!inputMessage.trim()}
              >
                发送
              </button>
            </div>
          </div>
        ) : (
          <div className="empty-chat">
            <div className="empty-chat-content">
              <div className="empty-icon">💬</div>
              <h2>欢迎来到 OmniLink</h2>
              <p>选择一个对话开始聊天，或创建新的对话</p>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
