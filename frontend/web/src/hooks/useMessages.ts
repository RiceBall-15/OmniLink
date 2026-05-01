import { useState, useEffect, useRef } from 'react'
import type { Conversation, Message, OnlineStatus } from '../types/message'
import { messageService } from '../services/messageService'
import { mockApi } from '../services/mockApi'

const USE_MOCK_DATA = import.meta.env.VITE_USE_MOCK_DATA === 'true'

// WebSocket连接管理
export function useWebSocket(url: string, onMessage: (data: any) => void) {
  const [connected, setConnected] = useState(false)
  const wsRef = useRef<WebSocket | null>(null)

  useEffect(() => {
    // 模拟模式下不连接WebSocket
    if (USE_MOCK_DATA) {
      console.log('模拟模式：跳过WebSocket连接')
      setConnected(true)
      return
    }

    const token = localStorage.getItem('token')
    const wsUrl = `${url}?token=${token}`

    wsRef.current = new WebSocket(wsUrl)

    wsRef.current.onopen = () => {
      console.log('WebSocket connected')
      setConnected(true)
    }

    wsRef.current.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data)
        onMessage(data)
      } catch (error) {
        console.error('Failed to parse WebSocket message:', error)
      }
    }

    wsRef.current.onerror = (error) => {
      console.error('WebSocket error:', error)
      setConnected(false)
    }

    wsRef.current.onclose = () => {
      console.log('WebSocket closed')
      setConnected(false)
      // 自动重连
      setTimeout(() => {
        if (wsRef.current?.readyState === WebSocket.CLOSED) {
          setConnected(false)
        }
      }, 3000)
    }

    return () => {
      if (wsRef.current) {
        wsRef.current.close()
      }
    }
  }, [url, onMessage])

  const sendMessage = (data: any) => {
    if (USE_MOCK_DATA) {
      console.log('模拟模式：跳过WebSocket消息发送')
      return
    }

    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(data))
    }
  }

  return { connected, sendMessage }
}

// 会话管理Hook
export function useConversations() {
  const [conversations, setConversations] = useState<Conversation[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    loadConversations()
  }, [])

  const loadConversations = async () => {
    if (USE_MOCK_DATA) {
      const response = await mockApi.getConversations()
      if (response.success && response.data) {
        setConversations(response.data)
      }
    } else {
      const response = await messageService.getConversations()
      if (response.success && response.data) {
        setConversations(response.data)
      }
    }
    setLoading(false)
  }

  const createConversation = async (data: {
    type: 'direct' | 'group' | 'ai'
    name?: string
    participantIds?: string[]
  }) => {
    const response = await messageService.createConversation(data)
    if (response.success && response.data) {
      setConversations([response.data, ...conversations])
      return response.data
    }
    return null
  }

  return { conversations, loading, loadConversations, createConversation }
}

// 消息管理Hook
export function useMessages(conversationId: string) {
  const [messages, setMessages] = useState<Message[]>([])
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    if (conversationId) {
      loadMessages(conversationId)
    }
  }, [conversationId])

  const loadMessages = async (convId: string) => {
    setLoading(true)
    if (USE_MOCK_DATA) {
      const response = await mockApi.getMessages(convId)
      if (response.success && response.data) {
        setMessages(response.data)
      }
    } else {
      const response = await messageService.getMessages(convId)
      if (response.success && response.data) {
        setMessages(response.data)
      }
    }
    setLoading(false)
  }

  const sendMessage = async (content: string) => {
    if (USE_MOCK_DATA) {
      const response = await mockApi.sendMessage(conversationId, content)
      if (response.success && response.data) {
        setMessages([...messages, response.data])
        return response.data
      }
      return null
    }

    const response = await messageService.sendMessage(conversationId, content)
    if (response.success && response.data) {
      setMessages([...messages, response.data])
      return response.data
    }
    return null
  }

  const markAsRead = async () => {
    await messageService.markAsRead(conversationId)
  }

  return { messages, loading, loadMessages, sendMessage, markAsRead }
}

// 在线状态Hook
export function useOnlineStatus() {
  const [status, setStatus] = useState<OnlineStatus>('online')

  const updateStatus = async (newStatus: OnlineStatus) => {
    setStatus(newStatus)
    await messageService.updateOnlineStatus(newStatus)
  }

  return { status, updateStatus }
}
