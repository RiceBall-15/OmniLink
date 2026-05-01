import request, { ApiResponse } from './api'
import type {
  AIAssistant,
  AIChatRequest,
  AIChatResponse,
  ModelInfo,
} from '../types/ai'

// AI服务
export const aiService = {
  // 获取AI助手列表
  getAssistants: async (): Promise<ApiResponse<AIAssistant[]>> => {
    return request<AIAssistant[]>('/api/ai/assistants')
  },

  // 获取AI助手详情
  getAssistant: async (assistantId: string): Promise<ApiResponse<AIAssistant>> => {
    return request<AIAssistant>(`/api/ai/assistants/${assistantId}`)
  },

  // 创建AI助手
  createAssistant: async (data: Omit<AIAssistant, 'id' | 'userId' | 'createdAt' | 'updatedAt'>): Promise<ApiResponse<AIAssistant>> => {
    return request<AIAssistant>('/api/ai/assistants', {
      method: 'POST',
      body: JSON.stringify(data),
    })
  },

  // 更新AI助手
  updateAssistant: async (assistantId: string, data: Partial<AIAssistant>): Promise<ApiResponse<AIAssistant>> => {
    return request<AIAssistant>(`/api/ai/assistants/${assistantId}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    })
  },

  // 删除AI助手
  deleteAssistant: async (assistantId: string): Promise<ApiResponse<void>> => {
    return request<void>(`/api/ai/assistants/${assistantId}`, {
      method: 'DELETE',
    })
  },

  // AI对话
  chat: async (data: AIChatRequest): Promise<ApiResponse<AIChatResponse>> => {
    return request<AIChatResponse>('/api/ai/chat', {
      method: 'POST',
      body: JSON.stringify(data),
    })
  },

  // 流式AI对话
  chatStream: async (data: AIChatRequest, onChunk: (chunk: string) => void): Promise<AIChatResponse> => {
    const token = localStorage.getItem('token')
    const response = await fetch('http://localhost:8003/api/ai/chat/stream', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
      body: JSON.stringify(data),
    })

    const reader = response.body?.getReader()
    const decoder = new TextDecoder()
    let fullContent = ''

    if (!reader) {
      throw new Error('无法读取响应流')
    }

    while (true) {
      const { done, value } = await reader.read()
      if (done) break

      const chunk = decoder.decode(value)
      fullContent += chunk
      onChunk(chunk)
    }

    // 返回完整响应
    return {
      id: crypto.randomUUID(),
      content: fullContent,
      model: data.stream ? 'stream' : 'normal',
      usage: {
        promptTokens: 0,
        completionTokens: 0,
        totalTokens: 0,
      },
      cost: 0,
      createdAt: new Date().toISOString(),
    }
  },

  // 获取可用模型列表
  getModels: async (): Promise<ApiResponse<ModelInfo[]>> => {
    return request<ModelInfo[]>('/api/ai/models')
  },
}