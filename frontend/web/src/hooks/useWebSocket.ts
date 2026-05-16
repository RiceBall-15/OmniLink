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
  /** 初始重连间隔（毫秒） */
  reconnectBaseInterval?: number
  /** 最大重连间隔（毫秒） */
  reconnectMaxInterval?: number
  /** 退避因子 */
  reconnectBackoffFactor?: number
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
  /** 重连状态变化回调 */
  onReconnecting?: (attempt: number, delay: number) => void
  /** 重连成功回调 */
  onReconnected?: () => void
}

/**
 * WebSocket 连接状态
 */
type ConnectionStatus = 'connecting' | 'connected' | 'disconnected' | 'error' | 'reconnecting'

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
  /** 当前重连尝试次数 */
  reconnectAttempt: number
  /** 下次重连延迟（毫秒） */
  nextReconnectDelay: number | null
  /** 是否在线（浏览器网络状态） */
  isOnline: boolean
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
 * 处理 WebSocket 连接、心跳保活、指数退避自动重连、网络状态检测等功能
 */
export function useWebSocket(config: WebSocketConfig): UseWebSocketReturn {
  const {
    url,
    autoConnect = true,
    autoReconnect = true,
    reconnectBaseInterval = 1000,   // 初始 1 秒
    reconnectMaxInterval = 30000,   // 最大 30 秒
    reconnectBackoffFactor = 2,     // 每次翻倍
    maxReconnectAttempts = 15,
    heartbeatInterval = 30000, // 30 秒
    heartbeatTimeout = 60000, // 60 秒
    onOpen,
    onClose,
    onError,
    onMessage,
    onStatusUpdate,
    onReconnecting,
    onReconnected,
  } = config

  // WebSocket 实例引用
  const wsRef = useRef<WebSocket | null>(null)

  // 连接状态
  const [status, setStatus] = useState<ConnectionStatus>('disconnected')
  // 最后消息时间
  const [lastMessageTime, setLastMessageTime] = useState<number | null>(null)
  // 当前重连尝试次数
  const [reconnectAttempt, setReconnectAttempt] = useState(0)
  // 下次重连延迟
  const [nextReconnectDelay, setNextReconnectDelay] = useState<number | null>(null)
  // 网络在线状态
  const [isOnline, setIsOnline] = useState(
    typeof navigator !== 'undefined' ? navigator.onLine : true
  )

  // 心跳定时器引用
  const heartbeatTimerRef = useRef<NodeJS.Timeout | null>(null)
  // 超时检测定时器引用
  const timeoutTimerRef = useRef<NodeJS.Timeout | null>(null)
  // 重连计数器
  const reconnectAttemptsRef = useRef(0)
  // 重连定时器引用
  const reconnectTimerRef = useRef<NodeJS.Timeout | null>(null)
  // 是否手动断开
  const manualDisconnectRef = useRef(false)
  // 页面是否可见
  const isVisibleRef = useRef(true)

  /**
   * 计算指数退避延迟
   */
  const calculateBackoffDelay = useCallback((attempt: number): number => {
    const delay = Math.min(
      reconnectBaseInterval * Math.pow(reconnectBackoffFactor, attempt),
      reconnectMaxInterval
    )
    // 添加 ±20% 随机抖动，避免重连风暴
    const jitter = delay * 0.2 * (Math.random() * 2 - 1)
    return Math.round(delay + jitter)
  }, [reconnectBaseInterval, reconnectBackoffFactor, reconnectMaxInterval])

  /**
   * 处理连接打开
   */
  const handleOpen = useCallback((event: Event) => {
    console.log('WebSocket 连接已建立')
    setStatus('connected')
    
    const wasReconnecting = reconnectAttemptsRef.current > 0
    reconnectAttemptsRef.current = 0
    setReconnectAttempt(0)
    setNextReconnectDelay(null)

    // 启动心跳定时器
    startHeartbeat()

    // 通知重连成功
    if (wasReconnecting && onReconnected) {
      onReconnected()
    }

    // 调用外部回调
    if (onOpen) {
      onOpen(event)
    }
  }, [onOpen, onReconnected])

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

    // 自动重连（仅在非手动断开且网络在线时）
    if (autoReconnect && !manualDisconnectRef.current && isOnline) {
      scheduleReconnect()
    }
  }, [autoReconnect, onClose, isOnline])

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
   * 安排重连（指数退避）
   */
  const scheduleReconnect = useCallback(() => {
    if (reconnectAttemptsRef.current >= maxReconnectAttempts) {
      console.error('达到最大重连次数，停止重连')
      setNextReconnectDelay(null)
      return
    }

    const attempt = reconnectAttemptsRef.current
    const delay = calculateBackoffDelay(attempt)
    reconnectAttemptsRef.current++
    
    setReconnectAttempt(reconnectAttemptsRef.current)
    setNextReconnectDelay(delay)
    setStatus('reconnecting')
    
    console.log(`安排重连，第 ${reconnectAttemptsRef.current} 次，${delay}ms 后重连...`)
    
    if (onReconnecting) {
      onReconnecting(reconnectAttemptsRef.current, delay)
    }

    reconnectTimerRef.current = setTimeout(() => {
      connect()
    }, delay)
  }, [maxReconnectAttempts, calculateBackoffDelay, onReconnecting])

  /**
   * 连接 WebSocket
   */
  const connect = useCallback(() => {
    if (wsRef.current) {
      wsRef.current.close()
    }

    // 如果离线，不尝试连接
    if (!isOnline) {
      console.log('网络离线，跳过 WebSocket 连接')
      return
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
  }, [url, handleOpen, handleClose, handleError, handleMessage, autoReconnect, scheduleReconnect, isOnline])

  /**
   * 断开 WebSocket 连接
   */
  const disconnect = useCallback(() => {
    manualDisconnectRef.current = true
    stopHeartbeat()

    if (reconnectTimerRef.current) {
      clearTimeout(reconnectTimerRef.current)
      reconnectTimerRef.current = null
    }

    if (wsRef.current) {
      wsRef.current.close(1000, 'User disconnected')
      wsRef.current = null
    }

    reconnectAttemptsRef.current = 0
    setReconnectAttempt(0)
    setNextReconnectDelay(null)
    setStatus('disconnected')
  }, [stopHeartbeat])

  /**
   * 手动重连
   */
  const reconnect = useCallback(() => {
    manualDisconnectRef.current = false
    disconnect()
    reconnectAttemptsRef.current = 0
    setTimeout(() => {
      connect()
    }, 100)
  }, [disconnect, connect])

  /**
   * 网络状态变化监听
   */
  useEffect(() => {
    const handleOnline = () => {
      console.log('网络恢复在线')
      setIsOnline(true)
      // 网络恢复时自动重连
      if (autoReconnect && !manualDisconnectRef.current) {
        reconnectAttemptsRef.current = 0
        setTimeout(() => connect(), 500)
      }
    }

    const handleOffline = () => {
      console.log('网络已离线')
      setIsOnline(false)
      // 停止重连尝试
      if (reconnectTimerRef.current) {
        clearTimeout(reconnectTimerRef.current)
        reconnectTimerRef.current = null
      }
    }

    window.addEventListener('online', handleOnline)
    window.addEventListener('offline', handleOffline)

    return () => {
      window.removeEventListener('online', handleOnline)
      window.removeEventListener('offline', handleOffline)
    }
  }, [autoReconnect, connect])

  /**
   * 页面可见性变化监听
   */
  useEffect(() => {
    const handleVisibilityChange = () => {
      const isVisible = document.visibilityState === 'visible'
      isVisibleRef.current = isVisible

      if (isVisible && !manualDisconnectRef.current) {
        // 页面变为可见时，检查连接状态
        const ws = wsRef.current
        if (!ws || ws.readyState === WebSocket.CLOSED) {
          console.log('页面恢复可见，WebSocket 已断开，尝试重连')
          reconnectAttemptsRef.current = 0
          setTimeout(() => connect(), 300)
        } else if (ws.readyState === WebSocket.OPEN) {
          // 发送心跳确认连接有效
          const pingMessage: WSMessage = {
            type: 'ping' as WSMessageType,
            timestamp: Date.now(),
          }
          ws.send(JSON.stringify(pingMessage))
        }
      }
    }

    document.addEventListener('visibilitychange', handleVisibilityChange)
    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange)
    }
  }, [connect])

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
    manualDisconnectRef.current = false
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
    reconnectAttempt,
    nextReconnectDelay,
    isOnline,
    send,
    connect,
    disconnect,
    reconnect,
  }
}
