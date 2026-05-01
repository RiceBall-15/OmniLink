// AI助手类型
export interface AIAssistant {
  id: string
  userId: string
  name: string
  avatar?: string
  modelProvider: 'openai' | 'anthropic' | 'google'
  modelName: string
  systemPrompt: string
  temperature: number
  maxTokens: number
  isPublic: boolean
  createdAt: string
  updatedAt: string
}

// AI对话请求
export interface AIChatRequest {
  assistantId: string
  message: string
  conversationId?: string
  stream?: boolean
}

// AI对话响应
export interface AIChatResponse {
  id: string
  content: string
  model: string
  usage: {
    promptTokens: number
    completionTokens: number
    totalTokens: number
  }
  cost: number
  createdAt: string
}

// 模型信息
export interface ModelInfo {
  id: string
  name: string
  provider: string
  contextWindow: number
  inputPrice: number // per 1K tokens
  outputPrice: number // per 1K tokens
  description: string
}
