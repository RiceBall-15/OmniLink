import request, { ApiResponse } from './api'
import type {
  Message,
  Conversation,
  MessageType,
  MessageStatus,
  OnlineStatus,
} from '../types/message'

// 消息服务
export const messageService = {
  // 获取会话列表
  getConversations: async (): Promise<ApiResponse<Conversation[]>> => {
    return request<Conversation[]>('/api/im/conversations')
  },

  // 获取会话消息
  getMessages: async (conversationId: string, page = 1, limit = 50): Promise<ApiResponse<Message[]>> => {
    return request<Message[]>(
      `/api/im/conversations/${conversationId}/messages?page=${page}&limit=${limit}`
    )
  },

  // 发送消息
  sendMessage: async (conversationId: string, content: string, type: MessageType = MessageType.TEXT): Promise<ApiResponse<Message>> => {
    return request<Message>(`/api/im/conversations/${conversationId}/messages`, {
      method: 'POST',
      body: JSON.stringify({ content, type }),
    })
  },

  // 标记消息为已读
  markAsRead: async (conversationId: string): Promise<ApiResponse<void>> => {
    return request<void>(`/api/im/conversations/${conversationId}/read`, {
      method: 'PUT',
    })
  },

  // 撤回消息
  recallMessage: async (messageId: string): Promise<ApiResponse<void>> => {
    return request<void>(`/api/im/messages/${messageId}/recall`, {
      method: 'PUT',
    })
  },

  // 编辑消息
  editMessage: async (messageId: string, content: string): Promise<ApiResponse<Message>> => {
    return request<Message>(`/api/im/messages/${messageId}`, {
      method: 'PUT',
      body: JSON.stringify({ content }),
    })
  },

  // 创建会话
  createConversation: async (data: {
    type: 'direct' | 'group' | 'ai'
    name?: string
    participantIds?: string[]
  }): Promise<ApiResponse<Conversation>> => {
    return request<Conversation>('/api/im/conversations', {
      method: 'POST',
      body: JSON.stringify(data),
    })
  },

  // 更新在线状态
  updateOnlineStatus: async (status: OnlineStatus): Promise<ApiResponse<void>> => {
    return request<void>('/api/im/status', {
      method: 'PUT',
      body: JSON.stringify({ status }),
    })
  },
}