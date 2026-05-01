import { useState, useEffect, useRef, useCallback } from 'react'
import { useAuth } from '../hooks/useAuth'
import { useConversations, useMessages, useWebSocket, WSMessageType } from '../hooks/useMessages'
import { MessageList } from '../components/MessageList'
import { AIChat } from '../components/AIChat'
import { MessageSearch } from '../components/MessageSearch'
import { FileUploader, FileList } from '../components/FileUploader'
import { AuthPage } from './AuthPage'
import { useToast } from '../components/Toast'
import { MessageStatus } from '../types/message'
import type { WSMessage } from '../types/message'
import './ChatPage.css'

/**
 * 聊天页面主组件
 * 负责管理聊天界面、侧边栏、消息发送等核心功能
 */
export function ChatPage() {
  const { user, logout } = useAuth()
  const { showSuccess, showError, showInfo } = useToast()
  const { conversations, loading: conversationsLoading, createConversation, updateConversation } =
    useConversations()

  // UI 状态
  const [selectedConversation, setSelectedConversation] = useState<string | null>(null)
  const [sidebarOpen, setSidebarOpen] = useState(true)
  const [inputMessage, setInputMessage] = useState('')
  const [selectedAssistant, setSelectedAssistant] = useState<string>('default')
  const [searchOpen, setSearchOpen] = useState(false)
  const [fileUploadOpen, setFileUploadOpen] = useState(false)
  const [isTyping, setIsTyping] = useState(false)
  const typingTimeoutRef = useRef<NodeJS.Timeout | null>(null)

  // 文件上传状态
  const [uploadedFiles, setUploadedFiles] = useState<
    Array<{ file: File; progress?: number; status?: 'uploading' | 'done' | 'error' }>
  >([])

  // Refs
  const messagesEndRef = useRef<HTMLDivElement>(null)

  // WebSocket 连接
  const { connected, sendMessage: sendWsMessage, error: wsError } = useWebSocket(
    import.meta.env.VITE_WS_URL || 'ws://localhost:8001',
    useCallback(
      (data: WSMessage) => {
        console.log('[ChatPage] 收到 WebSocket 消息:', data)

        switch (data.type) {
          case WSMessageType.CONNECTED:
            showSuccess('WebSocket 已连接')
            break

          case WSMessageType.NEW_MESSAGE:
            if (data.conversationId) {
              updateConversation(data.conversationId, {
                lastMessage: {
                  id: data.messageId || '',
                  conversationId: data.conversationId,
                  senderId: data.senderId || '',
                  content: data.content || '',
                  type: 'text' as any,
                  status: MessageStatus.DELIVERED,
                  createdAt: new Date().toISOString(),
                  updatedAt: new Date().toISOString(),
                },
                unreadCount: (conversations.find((c) => c.id === data.conversationId)?.unreadCount ||
                  0) + 1,
              })
              showSuccess('收到新消息')
            }
            break

          case WSMessageType.TYPING:
            if (data.senderId && data.conversationId === selectedConversation) {
              setIsTyping(true)
              if (typingTimeoutRef.current) {
                clearTimeout(typingTimeoutRef.current)
              }
              typingTimeoutRef.current = setTimeout(() => {
                setIsTyping(false)
              }, 3000)
            }
            break

          case WSMessageType.ERROR:
            showError(data.content || '发生错误')
            break

          default:
            console.log('[ChatPage] 未处理的消息类型:', data.type)
        }
      },
      [selectedConversation, conversations, updateConversation, showSuccess, showError]
    )
  )

  // 消息管理
  const { messages, loading: messagesLoading, sendMessage: sendApiMessage, addMessage } =
    useMessages(selectedConversation || '')

  // 自动滚动到底部
  useEffect(() => {
    if (messagesEndRef.current) {
      messagesEndRef.current.scrollIntoView({ behavior: 'smooth' })
    }
  }, [messages, isTyping])

  // WebSocket 错误提示
  useEffect(() => {
    if (wsError) {
      showError(`WebSocket 错误: ${wsError.message}`)
    }
  }, [wsError, showError])

  // 清理定时器
  useEffect(() => {
    return () => {
      if (typingTimeoutRef.current) {
        clearTimeout(typingTimeoutRef.current)
      }
    }
  }, [])

  // 未登录时显示登录页
  if (!user) {
    return <AuthPage />
  }

  /**
   * 发送消息
   */
  const handleSendMessage = useCallback(async () => {
    const trimmedMessage = inputMessage.trim()
    if (!trimmedMessage || !selectedConversation) return

    setInputMessage('')

    // 乐观更新 UI：立即显示消息
    const tempMessageId = `temp-${Date.now()}`
    addMessage({
      id: tempMessageId,
      conversationId: selectedConversation,
      senderId: user.id,
      content: trimmedMessage,
      type: 'text' as any,
      status: MessageStatus.SENDING,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    })

    try {
      const result = await sendApiMessage(trimmedMessage)

      if (result) {
        // 发送 WebSocket 通知
        sendWsMessage({
          type: WSMessageType.MESSAGE,
          conversationId: selectedConversation,
          content: trimmedMessage,
        })

        showSuccess('消息发送成功')
      } else {
        throw new Error('发送失败')
      }
    } catch (error) {
      console.error('[ChatPage] 发送消息失败:', error)
      showError('发送消息失败，请稍后重试')

      // 更新消息状态为失败
      // 注意：这里需要实现 updateMessage 功能
    }
  }, [inputMessage, selectedConversation, user.id, sendApiMessage, addMessage, sendWsMessage, showSuccess, showError])

  /**
   * 处理键盘事件
   */
  const handleKeyPress = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSendMessage()
    }
  }

  /**
   * 处理输入变化（发送正在输入状态）
   */
  const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setInputMessage(e.target.value)

    if (selectedConversation && connected) {
      sendWsMessage({
        type: WSMessageType.TYPING,
        conversationId: selectedConversation,
      })
    }
  }

  /**
   * 处理文件上传
   */
  const handleFileUpload = useCallback(
    async (files: File[]) => {
      if (!selectedConversation) {
        showError('请先选择对话')
        return
      }

      for (const file of files) {
        // 验证文件大小（10MB）
        const maxSize = 10 * 1024 * 1024
        if (file.size > maxSize) {
          showError(`文件 ${file.name} 超过 10MB 限制`)
          continue
        }

        const fileWithProgress = { file, progress: 0, status: 'uploading' as const }
        setUploadedFiles((prev) => [...prev, fileWithProgress])

        // 模拟文件上传进度
        for (let progress = 0; progress <= 100; progress += 10) {
          await new Promise((resolve) => setTimeout(resolve, 100))
          setUploadedFiles((prev) =>
            prev.map((f) => (f.file === file ? { ...f, progress, status: 'uploading' as const } : f))
          )
        }

        // 标记上传完成
        setUploadedFiles((prev) =>
          prev.map((f) => (f.file === file ? { ...f, progress: 100, status: 'done' as const } : f))
        )
      }

      setFileUploadOpen(false)
      showSuccess(`成功上传 ${files.length} 个文件`)
    },
    [selectedConversation, sendWsMessage, connected, showError, showSuccess]
  )

  /**
   * 移除已上传文件
   */
  const handleRemoveFile = useCallback((index: number) => {
    setUploadedFiles((prev) => prev.filter((_, i) => i !== index))
  }, [])

  /**
   * 创建新对话
   */
  const handleCreateConversation = useCallback(async () => {
    const newConversation = await createConversation({
      type: 'ai',
      name: `AI 对话 ${conversations.length + 1}`,
    })

    if (newConversation) {
      setSelectedConversation(newConversation.id)
      showSuccess('对话创建成功')
    }
  }, [conversations.length, createConversation, showSuccess])

  /**
   * 处理搜索消息选中
   */
  const handleMessageSelect = useCallback(
    (messageId: string) => {
      showInfo(`跳转到消息: ${messageId}`)
      // 实现滚动到指定消息的逻辑
    },
    [showInfo]
  )

  const selectedConv = conversations.find((c) => c.id === selectedConversation)

  return (
    <div className="chat-container">
      {/* 侧边栏 */}
      <div className={`sidebar ${sidebarOpen ? 'open' : 'closed'}`}>
        <div className="sidebar-header">
          <h2 className="app-title">💬 OmniLink</h2>
          <button
            className="new-chat-button"
            onClick={handleCreateConversation}
            title="新建对话"
            disabled={conversationsLoading}
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
          <div className="user-actions">
            <button
              className="settings-button"
              onClick={() => (window.location.href = '/settings')}
              title="设置"
              aria-label="设置"
            >
              ⚙️
            </button>
            <button
              className="logout-button"
              onClick={logout}
              title="退出登录"
              aria-label="退出登录"
            >
              🚪
            </button>
          </div>
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
                role="button"
                tabIndex={0}
                aria-selected={selectedConversation === conv.id}
              >
                <div className="conversation-avatar">
                  {conv.avatar ? (
                    <img src={conv.avatar} alt={conv.name} loading="lazy" />
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
                  <div className="unread-badge" aria-label={`${conv.unreadCount} 条未读消息`}>
                    {conv.unreadCount}
                  </div>
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
              <button
                className="sidebar-toggle"
                onClick={() => setSidebarOpen(!sidebarOpen)}
                aria-label={sidebarOpen ? '隐藏侧边栏' : '显示侧边栏'}
              >
                {sidebarOpen ? '◀' : '▶'}
              </button>
              <h3>{selectedConv?.name || '对话'}</h3>
              <div className="chat-header-actions">
                <button
                  className="header-action"
                  onClick={() => setSearchOpen(!searchOpen)}
                  title="搜索消息"
                  aria-label="搜索消息"
                  aria-expanded={searchOpen}
                >
                  🔍
                </button>
                <button
                  className="header-action"
                  onClick={() => setFileUploadOpen(!fileUploadOpen)}
                  title="上传文件"
                  aria-label="上传文件"
                  aria-expanded={fileUploadOpen}
                >
                  📎
                </button>
              </div>
            </div>

            {/* 搜索面板 */}
            {searchOpen && (
              <div className="search-panel">
                <MessageSearch
                  conversationId={selectedConversation}
                  onMessageSelect={handleMessageSelect}
                />
              </div>
            )}

            {/* 文件上传面板 */}
            {fileUploadOpen && (
              <div className="file-upload-panel">
                <FileUploader
                  onUpload={handleFileUpload}
                  maxSize={10}
                  multiple
                  accept="image/*,.pdf,.doc,.docx,.txt"
                />
                {uploadedFiles.length > 0 && (
                  <FileList files={uploadedFiles} onRemove={handleRemoveFile} />
                )}
              </div>
            )}

            {/* 消息区域 */}
            <div className="chat-messages" role="log" aria-live="polite">
              {messagesLoading ? (
                <div className="loading" aria-busy="true">
                  加载消息中...
                </div>
              ) : (
                <>
                  {selectedConv?.type === 'ai' ? (
                    <AIChat
                      conversationId={selectedConversation}
                      assistantId={selectedAssistant}
                    />
                  ) : (
                    <>
                      <MessageList messages={messages} currentUserId={user.id} />
                      {isTyping && (
                        <div className="typing-indicator" aria-label="对方正在输入">
                          <span></span>
                          <span></span>
                          <span></span>
                        </div>
                      )}
                      <div ref={messagesEndRef} />
                    </>
                  )}
                </>
              )}
            </div>

            {/* 输入区域 */}
            {selectedConv?.type !== 'ai' && (
              <div className="chat-input">
                <textarea
                  value={inputMessage}
                  onChange={handleInputChange}
                  onKeyDown={handleKeyPress}
                  placeholder="输入消息... (Enter 发送，Shift+Enter 换行)"
                  rows={1}
                  className="message-textarea"
                  aria-label="消息输入框"
                />
                <button
                  className="send-button"
                  onClick={handleSendMessage}
                  disabled={!inputMessage.trim()}
                  aria-label="发送消息"
                >
                  发送
                </button>
              </div>
            )}
          </div>
        ) : (
          <div className="empty-chat">
            <div className="empty-chat-content">
              <div className="empty-icon" role="img" aria-label="聊天图标">
                💬
              </div>
              <h2>欢迎来到 OmniLink</h2>
              <p>选择一个对话开始聊天，或创建新的对话</p>
              <div className="empty-chat-actions">
                <button
                  className="primary-button"
                  onClick={handleCreateConversation}
                  aria-label="创建新对话"
                >
                  + 创建新对话
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
