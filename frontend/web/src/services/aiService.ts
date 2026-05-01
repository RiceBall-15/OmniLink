import { api } from './api'
import type { AIAssistant, AIConversation, AIMessage, AIResponse } from '../types/ai'

export const aiService = {
  // 获取AI助手列表
  getAssistants: async () => {
    return api.get<AIAssistant[]>('/ai/assistants')
  },

  // 创建AI助手
  createAssistant: async (data: {
    name: string
    modelProvider: string
    modelName: string
    systemPrompt: string
    temperature: number
    maxTokens: number
  }) => {
    return api.post<AIAssistant>('/ai/assistants', data)
  },

  // 更新AI助手
  updateAssistant: async (id: string, data: Partial<AIAssistant>) => {
    return api.put<AIAssistant>(`/ai/assistants/${id}`, data)
  },

  // 删除AI助手
  deleteAssistant: async (id: string) => {
    return api.delete(`/ai/assistants/${id}`)
  },

  // 获取AI对话列表
  getConversations: async () => {
    return api.get<AIConversation[]>('/ai/conversations')
  },

  // 创建AI对话
  createConversation: async (data: {
    assistantId: string
    title?: string
  }) => {
    return api.post<AIConversation>('/ai/conversations', data)
  },

  // 获取AI对话消息
  getMessages: async (conversationId: string) => {
    return api.get<AIMessage[]>(`/ai/conversations/${conversationId}/messages`)
  },

  // 发送AI消息
  sendMessage: async (conversationId: string, content: string) => {
    return api.post<AIResponse>(`/ai/conversations/${conversationId}/messages`, {
      content,
    })
  },

  // 流式AI对话
  streamMessage: async (conversationId: string, content: string, onChunk: (chunk: string) => void) => {
    const token = localStorage.getItem('token')
    const response = await fetch(`${import.meta.env.VITE_API_BASE_URL}/ai/conversations/${conversationId}/stream`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
      body: JSON.stringify({ content }),
    })

    if (!response.ok) {
      throw new Error('AI对话失败')
    }

    const reader = response.body?.getReader()
    if (!reader) {
      throw new Error('无法创建流式读取器')
    }

    const decoder = new TextDecoder()
    let fullText = ''

    while (true) {
      const { done, value } = await reader.read()
      if (done) break

      const chunk = decoder.decode(value)
      fullText += chunk
      onChunk(chunk)
    }

    return fullText
  },

  // 生成代码补全
  completeCode: async (code: string, language: string) => {
    return api.post<{ completion: string }>('/ai/code/completion', {
      code,
      language,
    })
  },

  // 文档分析
  analyzeDocument: async (content: string, type: 'text' | 'code' | 'markdown') => {
    return api.post<{ summary: string; topics: string[] }>('/ai/document/analyze', {
      content,
      type,
    })
  },

  // 翻译
  translate: async (text: string, from: string, to: string) => {
    return api.post<{ translatedText: string }>('/ai/translate', {
      text,
      from,
      to,
    })
  },

  // 摘要生成
  summarize: async (text: string, maxLength?: number) => {
    return api.post<{ summary: string }>('/ai/summarize', {
      text,
      maxLength,
    })
  },
}
