import { useState, useEffect, useRef, useCallback } from 'react'
import type { Conversation, Message, OnlineStatus, WSMessage } from '../types/message'
import { messageService } from '../services/messageService'
import { mockApi } from '../services/mockApi'

const USE_MOCK_DATA = import.meta.env.VITE_USE_MOCK_DATA === 'true'

/**
 * WebSocket 重连配置
 */
const WS_RECONNECT_CONFIG = {
  maxAttempts: 5,
  baseDelay: 1000,
  maxDelay: 30000,
  backoffFactor: 2,
}

/**
 * 获取重连延迟时间（指数退避）
 */
function getReconnectDelay(attempt: number): number {
  const delay = WS_RECONNECT_CONFIG.baseDelay * Math.pow(WS_RECONNECT_CONFIG.backoffFactor, attempt)
  return Math.min(delay, WS_RECONNECT_CONFIG.maxDelay)
}

/**
 * 类型守卫：检查对象是否为有效的 WebSocket 消息
 */
function isValidWSMessage(data: unknown): data is WSMessage {
  return (
    typeof data === 'object' &&
    data !== null &&
    'type' in data &&
    typeof data.type === 'string'
  )
}

/**
 * WebSocket 连接管理 Hook
 * @param url WebSocket 服务器地址
 * @param onMessage 消息回调函数
 * @returns 连接状态和发送消息函数
 */
export function useWebSocket(url: string, onMessage: (data: WSMessage) => void) {
  const [connected, setConnected] = useState(false)
  const [connecting, setConnecting] = useState(false)
  const [error, setError] = useState<Error | null>(null)
  const wsRef = useRef<WebSocket | null>(null)
  const reconnectAttemptsRef = useRef(0)
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null)

  const cleanup = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current)
      reconnectTimeoutRef.current = null
    }
    if (wsRef.current) {
      wsRef.current.close()
      wsRef.current = null
    }
  }, [])

  const connect = useCallback(() => {
    // 模拟模式下不连接 WebSocket
    if (USE_MOCK_DATA) {
      console.log('[WebSocket] 模拟模式：跳过连接')
      setConnected(true)
      setConnecting(false)
      return
    }

    if (connecting || wsRef.current?.readyState === WebSocket.OPEN) {
      return
    }

    setConnecting(true)
    setError(null)

    const token = localStorage.getItem('token')
    if (!token) {
      const err = new Error('未找到认证令牌')
      setError(err)
      setConnecting(false)
      return
    }

    const wsUrl = `${url}?token=${token}`
    console.log(`[WebSocket] 连接中... (${wsUrl})`)

    try {
      const ws = new WebSocket(wsUrl)
      wsRef.current = ws

      ws.onopen = () => {
        console.log('[WebSocket] 已连接')
        setConnected(true)
        setConnecting(false)
        setError(null)
        reconnectAttemptsRef.current = 0
      }

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data)
          if (isValidWSMessage(data)) {
            onMessage(data)
          } else {
            console.warn('[WebSocket] 收到无效消息格式:', data)
          }
        } catch (parseError) {
          console.error('[WebSocket] 解析消息失败:', parseError, event.data)
        }
      }

      ws.onerror = (event) => {
        console.error('[WebSocket] 连接错误:', event)
        const err = new Error('WebSocket 连接失败')
        setError(err)
        setConnected(false)
        setConnecting(false)
      }

      ws.onclose = (event) => {
        console.log(`[WebSocket] 连接关闭 (code: ${event.code}, reason: ${event.reason || 'unknown'})`)
        setConnected(false)
        setConnecting(false)

        // 自动重连（非正常关闭时）
        if (event.code !== 1000 && reconnectAttemptsRef.current < WS_RECONNECT_CONFIG.maxAttempts) {
          const delay = getReconnectDelay(reconnectAttemptsRef.current)
          console.log(`[WebSocket] 将在 ${delay}ms 后重试第 ${reconnectAttemptsRef.current + 1} 次`)

          reconnectTimeoutRef.current = setTimeout(() => {
            reconnectAttemptsRef.current++
            connect()
          }, delay)
        } else if (reconnectAttemptsRef.current >= WS_RECONNECT_CONFIG.maxAttempts) {
          console.error('[WebSocket] 已达到最大重连次数，停止重试')
          setError(new Error('WebSocket 连接失败，请刷新页面重试'))
        }
      }
    } catch (err) {
      const error = err instanceof Error ? err : new Error('创建 WebSocket 连接失败')
      console.error('[WebSocket] 创建连接失败:', error)
      setError(error)
      setConnecting(false)
    }
  }, [url, onMessage, connecting])

  const disconnect = useCallback(() => {
    cleanup()
    setConnected(false)
    setConnecting(false)
  }, [cleanup])

  useEffect(() => {
    connect()
    return cleanup
  }, [connect, cleanup])

  const sendMessage = useCallback((data: unknown) => {
    if (USE_MOCK_DATA) {
      console.log('[WebSocket] 模拟模式：跳过消息发送', data)
      return { success: true }
    }

    const ws = wsRef.current
    if (!ws || ws.readyState !== WebSocket.OPEN) {
      console.error('[WebSocket] 未连接，无法发送消息')
      return { success: false, error: '未连接' }
    }

    try {
      const message = typeof data === 'string' ? data : JSON.stringify(data)
      ws.send(message)
      return { success: true }
    } catch (err) {
      const error = err instanceof Error ? err : new Error('发送消息失败')
      console.error('[WebSocket] 发送消息失败:', error)
      return { success: false, error: error.message }
    }
  }, [])

  return {
    connected,
    connecting,
    error,
    sendMessage,
    reconnect: connect,
    disconnect,
  }
}

/**
 * 安全的 API 调用封装
 * 统一处理 Mock 和真实 API 调用
 */
async function safeApiCall<T>(
  mockCall: () => Promise<{ success: boolean; data?: T; error?: { message: string } }>,
  apiCall: () => Promise<{ success: boolean; data?: T; error?: { message: string } }>
): Promise<{ success: boolean; data?: T; error?: string }> {
  try {
    const response = USE_MOCK_DATA ? await mockCall() : await apiCall()
    return {
      success: response.success,
      data: response.data,
      error: response.error?.message,
    }
  } catch (error) {
    const err = error instanceof Error ? error : new Error('未知错误')
    console.error('[API] 调用失败:', err)
    return {
      success: false,
      error: err.message || '请求失败',
    }
  }
}

/**
 * 会话管理 Hook
 * @returns 会话列表和操作函数
 */
export function useConversations() {
  const [conversations, setConversations] = useState<Conversation[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const loadConversations = useCallback(async () => {
    setLoading(true)
    setError(null)

    const { success, data, error: err } = await safeApiCall(
      () => mockApi.getConversations(),
      () => messageService.getConversations()
    )

    if (success && data) {
      setConversations(data)
    } else {
      setError(err || '加载会话列表失败')
    }

    setLoading(false)
  }, [])

  const createConversation = useCallback(
    async (data: {
      type: 'direct' | 'group' | 'ai'
      name?: string
      participantIds?: string[]
    }) => {
      const { success, data: newConv, error: err } = await safeApiCall(
        async () => {
          // Mock 模式下的创建逻辑
          const response = await mockApi.getConversations()
          if (response.success && response.data) {
            const conv: Conversation = {
              id: `conv-${Date.now()}`,
              type: data.type,
              name: data.name || `新对话`,
              avatar: undefined,
              lastMessage: undefined,
              unreadCount: 0,
              isPinned: false,
              isMuted: false,
              createdAt: new Date().toISOString(),
              updatedAt: new Date().toISOString(),
            }
            return { success: true, data: conv }
          }
          return { success: false, error: { message: '创建失败' } }
        },
        () => messageService.createConversation(data)
      )

      if (success && newConv) {
        setConversations((prev) => [newConv, ...prev])
        return newConv
      }

      setError(err || '创建会话失败')
      return null
    },
    []
  )

  const updateConversation = useCallback(
    (conversationId: string, updates: Partial<Conversation>) => {
      setConversations((prev) =>
        prev.map((conv) => (conv.id === conversationId ? { ...conv, ...updates } : conv))
      )
    },
    []
  )

  const deleteConversation = useCallback(async (conversationId: string) => {
    const { success, error: err } = await safeApiCall(
      async () => ({ success: true }),
      () => messageService.deleteConversation(conversationId)
    )

    if (success) {
      setConversations((prev) => prev.filter((conv) => conv.id !== conversationId))
      return true
    }

    setError(err || '删除会话失败')
    return false
  }, [])

  useEffect(() => {
    loadConversations()
  }, [loadConversations])

  return {
    conversations,
    loading,
    error,
    loadConversations,
    createConversation,
    updateConversation,
    deleteConversation,
  }
}

/**
 * 消息管理 Hook
 * @param conversationId 会话 ID
 * @returns 消息列表和操作函数
 */
export function useMessages(conversationId: string) {
  const [messages, setMessages] = useState<Message[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const loadMessages = useCallback(async (convId: string) => {
    if (!convId) return

    setLoading(true)
    setError(null)

    const { success, data, error: err } = await safeApiCall(
      () => mockApi.getMessages(convId),
      () => messageService.getMessages(convId)
    )

    if (success && data) {
      setMessages(data)
    } else {
      setError(err || '加载消息失败')
    }

    setLoading(false)
  }, [])

  const sendMessage = useCallback(
    async (content: string) => {
      if (!conversationId) {
        setError('请先选择会话')
        return null
      }

      const { success, data: newMessage, error: err } = await safeApiCall(
        () => mockApi.sendMessage(conversationId, content),
        () => messageService.sendMessage(conversationId, content)
      )

      if (success && newMessage) {
        setMessages((prev) => [...prev, newMessage])
        return newMessage
      }

      setError(err || '发送消息失败')
      return null
    },
    [conversationId]
  )

  const addMessage = useCallback((message: Message) => {
    setMessages((prev) => {
      // 避免重复添加
      if (prev.some((m) => m.id === message.id)) {
        return prev
      }
      return [...prev, message]
    })
  }, [])

  const updateMessage = useCallback(
    (messageId: string, updates: Partial<Message>) => {
      setMessages((prev) =>
        prev.map((msg) => (msg.id === messageId ? { ...msg, ...updates } : msg))
      )
    },
    []
  )

  const markAsRead = useCallback(async () => {
    const { success, error: err } = await safeApiCall(
      async () => ({ success: true }),
      () => messageService.markAsRead(conversationId)
    )

    if (!success) {
      setError(err || '标记已读失败')
    }

    return success
  }, [conversationId])

  useEffect(() => {
    loadMessages(conversationId)
  }, [conversationId, loadMessages])

  return {
    messages,
    loading,
    error,
    loadMessages,
    sendMessage,
    addMessage,
    updateMessage,
    markAsRead,
  }
}

/**
 * 在线状态管理 Hook
 * @returns 在线状态和更新函数
 */
export function useOnlineStatus() {
  const [status, setStatus] = useState<OnlineStatus>('online')

  const updateStatus = useCallback(async (newStatus: OnlineStatus) => {
    setStatus(newStatus)

    const { success } = await safeApiCall(
      async () => ({ success: true }),
      () => messageService.updateOnlineStatus(newStatus)
    )

    if (!success) {
      console.error('[OnlineStatus] 更新状态失败')
    }

    return success
  }, [])

  return { status, updateStatus }
}
