import { apiRequest } from './api'

// ============================================================
// 类型定义
// ============================================================

export interface AIAssistant {
  id: string
  name: string
  description: string
  system_prompt: string
  model: string
  temperature: number
  max_tokens: number
  avatar?: string
  created_at: string
  updated_at: string
}

export interface CreateAssistantRequest {
  name: string
  description?: string
  system_prompt: string
  model?: string
  temperature?: number
  max_tokens?: number
  avatar?: string
}

export interface UpdateAssistantRequest {
  name?: string
  description?: string
  system_prompt?: string
  model?: string
  temperature?: number
  max_tokens?: number
  avatar?: string
}

export interface ChatMessage {
  role: 'system' | 'user' | 'assistant'
  content: string
  timestamp?: string
}

export interface ChatRequest {
  messages: ChatMessage[]
  model?: string
  assistant_id?: string
  temperature?: number
  max_tokens?: number
  stream?: boolean
}

export interface ChatResponse {
  message: ChatMessage
  usage: TokenUsage
}

export interface TokenUsage {
  prompt_tokens: number
  completion_tokens: number
  total_tokens: number
}

export interface ConversationMessage {
  id: string
  conversation_id: string
  role: 'system' | 'user' | 'assistant'
  content: string
  tokens_used: number
  created_at: string
}

export interface ModelInfo {
  id: string
  name: string
  provider: string
  max_tokens: number
  description?: string
}

export interface UsageStats {
  total_prompt_tokens: number
  total_completion_tokens: number
  total_tokens: number
  by_model: Record<string, TokenUsage>
  by_date: Record<string, TokenUsage>
}

export interface ApiKeyInfo {
  id: string
  provider: string
  key_preview: string
  is_active: boolean
  last_used_at?: string
  created_at: string
}

// ============================================================
// AI 服务
// ============================================================

export const aiService = {
  // === 聊天 ===

  /** 发送聊天消息（非流式） */
  async chat(request: ChatRequest): Promise<ChatResponse> {
    return apiRequest<ChatResponse>('/ai/chat', {
      method: 'POST',
      body: JSON.stringify({ ...request, stream: false }),
    })
  },

  /** 发送聊天消息（流式，返回 EventSource） */
  createChatStream(request: ChatRequest): EventSource {
    const token = localStorage.getItem('token')
    const baseUrl = import.meta.env.VITE_API_BASE_URL || 'http://localhost:8002'

    // 构建 URL，将参数放在 query string 中以便 EventSource 使用
    const params = new URLSearchParams()
    if (request.model) params.set('model', request.model)
    if (request.assistant_id) params.set('assistant_id', request.assistant_id)
    if (request.temperature !== undefined) params.set('temperature', String(request.temperature))
    if (request.max_tokens !== undefined) params.set('max_tokens', String(request.max_tokens))
    if (token) params.set('token', token)

    const url = `${baseUrl}/ai/chat/stream?${params.toString()}`
    return new EventSource(url)
  },

  // === 助手管理 ===

  /** 获取助手列表 */
  async listAssistants(): Promise<AIAssistant[]> {
    return apiRequest<AIAssistant[]>('/ai/assistants')
  },

  /** 创建助手 */
  async createAssistant(data: CreateAssistantRequest): Promise<AIAssistant> {
    return apiRequest<AIAssistant>('/ai/assistants', {
      method: 'POST',
      body: JSON.stringify(data),
    })
  },

  /** 获取单个助手 */
  async getAssistant(id: string): Promise<AIAssistant> {
    return apiRequest<AIAssistant>(`/ai/assistants/${id}`)
  },

  /** 更新助手 */
  async updateAssistant(id: string, data: UpdateAssistantRequest): Promise<AIAssistant> {
    return apiRequest<AIAssistant>(`/ai/assistants/${id}`, {
      method: 'POST',
      body: JSON.stringify(data),
    })
  },

  /** 删除助手 */
  async deleteAssistant(id: string): Promise<void> {
    return apiRequest<void>(`/ai/assistants/${id}`, {
      method: 'DELETE',
    })
  },

  // === 模型 ===

  /** 获取可用模型列表 */
  async listModels(): Promise<ModelInfo[]> {
    return apiRequest<ModelInfo[]>('/ai/models')
  },

  // === 对话历史 ===

  /** 获取对话历史 */
  async getConversationHistory(conversationId: string): Promise<ConversationMessage[]> {
    return apiRequest<ConversationMessage[]>(`/ai/conversations/${conversationId}/messages`)
  },

  /** 清空对话历史 */
  async clearConversation(conversationId: string): Promise<void> {
    return apiRequest<void>(`/ai/conversations/${conversationId}`, {
      method: 'DELETE',
    })
  },

  // === 使用统计 ===

  /** 获取 Token 使用统计 */
  async getUsageStats(): Promise<UsageStats> {
    return apiRequest<UsageStats>('/ai/usage')
  },

  // === API 密钥管理 ===

  /** 获取 API 密钥列表 */
  async listApiKeys(): Promise<ApiKeyInfo[]> {
    return apiRequest<ApiKeyInfo[]>('/ai/keys')
  },

  /** 轮换 API 密钥 */
  async rotateApiKey(keyId: string): Promise<ApiKeyInfo> {
    return apiRequest<ApiKeyInfo>('/ai/keys/rotate', {
      method: 'POST',
      body: JSON.stringify({ key_id: keyId }),
    })
  },

  /** 回滚 API 密钥 */
  async rollbackApiKey(keyId: string): Promise<ApiKeyInfo> {
    return apiRequest<ApiKeyInfo>('/ai/keys/rollback', {
      method: 'POST',
      body: JSON.stringify({ key_id: keyId }),
    })
  },

  /** 切换 API 密钥状态 */
  async toggleApiKey(keyId: string): Promise<ApiKeyInfo> {
    return apiRequest<ApiKeyInfo>('/ai/keys/toggle', {
      method: 'POST',
      body: JSON.stringify({ key_id: keyId }),
    })
  },
}
