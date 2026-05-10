import { useState, useEffect, useCallback, useRef } from 'react'
import type { WSMessage, StatusUpdateData } from '../types/message'
import { WSMessageType } from '../types/message'

/**
 * WebSocket 配置
 */
interface WebSocketConfig {
  /** WebSocket 服务器地址 */
  url: string
  /** 是否自动连接 */
  autoConnect?: boolean
  /** 是否自动重连 */
  autoReconnect?: boolean
  /** 重连间隔（毫秒） */
  reconnectInterval?: number
  /** 最大重连次数 */
  maxReconnectAttempts?: number
  /** 心跳间隔（毫秒） */
  heartbeatInterval?: number
  /** 心跳超时（毫秒） */
  heartbeatTimeout?: number
  /** 连接建立回调 */
  onOpen?: (event: Event) => void
  /** 连接关闭回调 */
  onClose?: (event: CloseEvent) => void
  /** 连接错误回调 */
  onError?: (event: Event) => void
  /** 消息接收回调 */
  onMessage?: (message: WSMessage) => void
  /** 在线状态更新回调 */
  onStatusUpdate?: (data: StatusUpdateData) => void
}

/**
 * WebSocket 连接状态
 */
type ConnectionStatus = 'connecting' | 'connected' | 'disconnected' | 'error'

/**
 * WebSocket Hook 返回值
 */
interface UseWebSocketReturn {
  /** 连接状态 */
  status: ConnectionStatus
  /** 最后消息时间 */
  lastMessageTime: number | null
  /** 是否已连接 */
  isConnected: boolean
  /** 发送消息 */
  send: (message: WSMessage) => boolean
  /** 连接 */
  connect: () => void
  /** 断开连接 */
  disconnect: () => void
  /** 手动重连 */
  reconnect: () => void
}

/**
 * WebSocket 管理 Hook
 * 处理 WebSocket 连接、心跳保活、自动重连等功能
 */
export function useWebSocket(config: WebSocketConfig): UseWebSocketReturn {
  const {
    url,
    autoConnect = true,
    autoReconnect = true,
    reconnectInterval = 3000,
    maxReconnectAttempts = 10,
    heartbeatInterval = 30000, // 30 秒
    heartbeatTimeout = 60000, // 60 秒
    onOpen,
    onClose,
    onError,
    onMessage,
    onStatusUpdate,
  } = config

  // WebSocket 实例引用
  const wsRef = useRef<WebSocket | null>(null)

  // 连接状态
  const [status, setStatus] = useState<ConnectionStatus>('disconnected')
  // 最后消息时间
  const [lastMessageTime, setLastMessageTime] = useState<number | null>(null)

  // 心跳定时器引用
  const heartbeatTimerRef = useRef<NodeJS.Timeout | null>(null)
  // 超时检测定时器引用
  const timeoutTimerRef = useRef<NodeJS.Timeout | null>(null)
  // 重连计数器
  const reconnectAttemptsRef = useRef(0)
  // 重连定时器引用
  const reconnectTimerRef = useRef<NodeJS.Timeout | null>(null)

  /**
   * 处理连接打开
   */
  const handleOpen = useCallback((event: Event) => {
    console.log('WebSocket 连接已建立')
    setStatus('connected')
    reconnectAttemptsRef.current = 0

    // 启动心跳定时器
    startHeartbeat()

    // 调用外部回调
    if (onOpen) {
      onOpen(event)
    }
  }, [onOpen])

  /**
   * 处理连接关闭
   */
  const handleClose = useCallback((event: CloseEvent) => {
    console.log('WebSocket 连接已关闭:', event.code, event.reason)
    setStatus('disconnected')

    // 清理心跳定时器
    stopHeartbeat()

    // 调用外部回调
    if (onClose) {
      onClose(event)
    }

    // 自动重连
    if (autoReconnect && !event.wasClean) {
      scheduleReconnect()
    }
  }, [autoReconnect, onClose])

  /**
   * 处理连接错误
   */
  const handleError = useCallback((event: Event) => {
    console.error('WebSocket 连接错误:', event)
    setStatus('error')

    // 调用外部回调
    if (onError) {
      onError(event)
    }
  }, [onError])

  /**
   * 处理接收消息
   */
  const handleMessage = useCallback((event: MessageEvent) => {
    try {
      const message: WSMessage = JSON.parse(event.data)
      setLastMessageTime(Date.now())

      console.log('收到 WebSocket 消息:', message)

      // 更新最后响应时间，重置超时检测
      if (timeoutTimerRef.current) {
        clearTimeout(timeoutTimerRef.current)
      }
      timeoutTimerRef.current = setTimeout(() => {
        console.warn('WebSocket 响应超时')
        reconnect()
      }, heartbeatTimeout)

      // 处理 PONG 响应
      if (message.type === WSMessageType.PONG) {
        console.log('收到 PONG 响应')
        return
      }

      // 处理状态更新消息
      if (message.type === WSMessageType.STATUS_UPDATE) {
        if (onStatusUpdate && message.data) {
          const statusData = message.data as StatusUpdateData
          onStatusUpdate(statusData)
        }
      }

      // 调用外部回调
      if (onMessage) {
        onMessage(message)
      }
    } catch (error) {
      console.error('解析 WebSocket 消息失败:', error)
    }
  }, [onMessage, onStatusUpdate, heartbeatTimeout])

  /**
   * 启动心跳
   */
  const startHeartbeat = useCallback(() => {
    if (heartbeatTimerRef.current) {
      clearInterval(heartbeatTimerRef.current)
    }

    heartbeatTimerRef.current = setInterval(() => {
      if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
        // 发送 PING 消息
        const pingMessage: WSMessage = {
          type: 'ping' as WSMessageType,
          timestamp: Date.now(),
        }
        wsRef.current.send(JSON.stringify(pingMessage))
        console.log('发送 PING 消息')

        // 设置超时检测
        if (timeoutTimerRef.current) {
          clearTimeout(timeoutTimerRef.current)
        }
        timeoutTimerRef.current = setTimeout(() => {
          console.warn('心跳超时，尝试重连')
          reconnect()
        }, heartbeatTimeout)
      }
    }, heartbeatInterval)
  }, [heartbeatInterval, heartbeatTimeout])

  /**
   * 停止心跳
   */
  const stopHeartbeat = useCallback(() => {
    if (heartbeatTimerRef.current) {
      clearInterval(heartbeatTimerRef.current)
      heartbeatTimerRef.current = null
    }
    if (timeoutTimerRef.current) {
      clearTimeout(timeoutTimerRef.current)
      timeoutTimerRef.current = null
    }
  }, [])

  /**
   * 安排重连
   */
  const scheduleReconnect = useCallback(() => {
    if (reconnectAttemptsRef.current >= maxReconnectAttempts) {
      console.error('达到最大重连次数，停止重连')
      return
    }

    reconnectAttemptsRef.current++
    console.log(`安排重连，第 ${reconnectAttemptsRef.current} 次...`)

    reconnectTimerRef.current = setTimeout(() => {
      connect()
    }, reconnectInterval)
  }, [maxReconnectAttempts, reconnectInterval])

  /**
   * 连接 WebSocket
   */
  const connect = useCallback(() => {
    if (wsRef.current) {
      wsRef.current.close()
    }

    setStatus('connecting')
    console.log('连接 WebSocket...', url)

    try {
      const ws = new WebSocket(url)
      wsRef.current = ws

      ws.onopen = handleOpen
      ws.onclose = handleClose
      ws.onerror = handleError
      ws.onmessage = handleMessage
    } catch (error) {
      console.error('创建 WebSocket 连接失败:', error)
      setStatus('error')
      if (autoReconnect) {
        scheduleReconnect()
      }
    }
  }, [url, handleOpen, handleClose, handleError, handleMessage, autoReconnect, scheduleReconnect])

  /**
   * 断开 WebSocket 连接
   */
  const disconnect = useCallback(() => {
    stopHeartbeat()

    if (reconnectTimerRef.current) {
      clearTimeout(reconnectTimerRef.current)
      reconnectTimerRef.current = null
    }

    if (wsRef.current) {
      wsRef.current.close(1000, 'User disconnected')
      wsRef.current = null
    }

    setStatus('disconnected')
  }, [stopHeartbeat])

  /**
   * 手动重连
   */
  const reconnect = useCallback(() => {
    disconnect()
    reconnectAttemptsRef.current = 0
    setTimeout(() => {
      connect()
    }, 100)
  }, [disconnect, connect])

  /**
   * 发送消息
   */
  const send = useCallback((message: WSMessage): boolean => {
    if (!wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) {
      console.error('WebSocket 未连接，无法发送消息')
      return false
    }

    try {
      wsRef.current.send(JSON.stringify(message))
      return true
    } catch (error) {
      console.error('发送 WebSocket 消息失败:', error)
      return false
    }
  }, [])

  /**
   * 自动连接
   */
  useEffect(() => {
    if (autoConnect) {
      connect()
    }

    // 清理函数
    return () => {
      disconnect()
    }
  }, [autoConnect, connect, disconnect])

  return {
    status,
    lastMessageTime,
    isConnected: status === 'connected',
    send,
    connect,
    disconnect,
    reconnect,
  }
}
