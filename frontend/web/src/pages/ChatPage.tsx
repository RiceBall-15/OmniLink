import { useState, useEffect } from 'react'
import { useAuth } from '../hooks/useAuth'
import { useConversations } from '../hooks/useMessages'
import { AuthPage } from './AuthPage'
import './ChatPage.css'

export function ChatPage() {
  const { user, logout } = useAuth()
  const { conversations, loading: conversationsLoading } = useConversations()
  const [selectedConversation, setSelectedConversation] = useState<string | null>(null)
  const [sidebarOpen, setSidebarOpen] = useState(true)

  if (!user) {
    return <AuthPage />
  }

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
              <h3>
                {conversations.find((c) => c.id === selectedConversation)?.name}
              </h3>
            </div>
            <div className="chat-messages">
              {/* 消息列表将在这里渲染 */}
              <div className="welcome-message">
                <p>👋 开始对话吧！</p>
                <p className="welcome-hint">这里将显示聊天消息</p>
              </div>
            </div>
            <div className="chat-input">
              <input
                type="text"
                placeholder="输入消息..."
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    console.log('发送消息')
                  }
                }}
              />
              <button className="send-button">发送</button>
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
