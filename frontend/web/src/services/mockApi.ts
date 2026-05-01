import type { User, Conversation, Message, AIAssistant } from '../types'

// 模拟数据（用于开发测试）
export const mockUser: User = {
  id: '1',
  username: 'DemoUser',
  email: 'demo@example.com',
  avatar: undefined,
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
}

export const mockConversations: Conversation[] = [
  {
    id: 'conv-1',
    type: 'ai',
    name: 'AI助手',
    avatar: undefined,
    lastMessage: {
      id: 'msg-1',
      conversationId: 'conv-1',
      senderId: 'ai-1',
      content: '你好！我是AI助手，有什么可以帮助你的吗？',
      type: 'text',
      status: 'delivered',
      createdAt: new Date(Date.now() - 1000 * 60 * 5).toISOString(),
      updatedAt: new Date(Date.now() - 1000 * 60 * 5).toISOString(),
    },
    unreadCount: 0,
    isPinned: false,
    isMuted: false,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  },
  {
    id: 'conv-2',
    type: 'ai',
    name: '代码助手',
    avatar: undefined,
    lastMessage: {
      id: 'msg-2',
      conversationId: 'conv-2',
      senderId: 'ai-2',
      content: '让我帮你分析这段代码...',
      type: 'text',
      status: 'delivered',
      createdAt: new Date(Date.now() - 1000 * 60 * 60).toISOString(),
      updatedAt: new Date(Date.now() - 1000 * 60 * 60).toISOString(),
    },
    unreadCount: 2,
    isPinned: false,
    isMuted: false,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  },
]

export const mockMessages: Message[] = [
  {
    id: 'msg-1',
    conversationId: 'conv-1',
    senderId: '1',
    content: '你好！',
    type: 'text',
    status: 'delivered',
    createdAt: new Date(Date.now() - 1000 * 60 * 10).toISOString(),
    updatedAt: new Date(Date.now() - 1000 * 60 * 10).toISOString(),
  },
  {
    id: 'msg-2',
    conversationId: 'conv-1',
    senderId: 'ai-1',
    content: '你好！我是AI助手，有什么可以帮助你的吗？',
    type: 'text',
    status: 'delivered',
    createdAt: new Date(Date.now() - 1000 * 60 * 5).toISOString(),
    updatedAt: new Date(Date.now() - 1000 * 60 * 5).toISOString(),
  },
  {
    id: 'msg-3',
    conversationId: 'conv-1',
    senderId: '1',
    content: '请帮我解释一下React Hooks的使用',
    type: 'text',
    status: 'delivered',
    createdAt: new Date(Date.now() - 1000 * 60 * 2).toISOString(),
    updatedAt: new Date(Date.now() - 1000 * 60 * 2).toISOString(),
  },
]

export const mockAssistants: AIAssistant[] = [
  {
    id: 'ai-1',
    userId: '1',
    name: '通用助手',
    avatar: undefined,
    modelProvider: 'openai',
    modelName: 'gpt-4',
    systemPrompt: '你是一个友好的AI助手，帮助用户解决问题。',
    temperature: 0.7,
    maxTokens: 2048,
    isPublic: true,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  },
  {
    id: 'ai-2',
    userId: '1',
    name: '代码助手',
    avatar: undefined,
    modelProvider: 'openai',
    modelName: 'gpt-4',
    systemPrompt: '你是一个专业的编程助手，帮助用户解决编程问题。',
    temperature: 0.3,
    maxTokens: 4096,
    isPublic: true,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  },
]

// 模拟API服务
export const mockApi = {
  // 模拟延迟
  delay: (ms: number = 500) => new Promise((resolve) => setTimeout(resolve, ms)),

  // 模拟登录
  login: async (email: string, password: string) => {
    await mockApi.delay()
    return {
      success: true,
      data: {
        token: 'mock-token-' + Date.now(),
        user: mockUser,
      },
    }
  },

  // 模拟注册
  register: async (username: string, email: string, password: string) => {
    await mockApi.delay()
    return {
      success: true,
      data: mockUser,
    }
  },

  // 模拟获取会话列表
  getConversations: async () => {
    await mockApi.delay()
    return {
      success: true,
      data: mockConversations,
    }
  },

  // 模拟获取消息列表
  getMessages: async (conversationId: string) => {
    await mockApi.delay()
    return {
      success: true,
      data: mockMessages.filter((m) => m.conversationId === conversationId),
    }
  },

  // 模拟发送消息
  sendMessage: async (conversationId: string, content: string) => {
    await mockApi.delay()
    const newMessage: Message = {
      id: 'msg-' + Date.now(),
      conversationId,
      senderId: mockUser.id,
      content,
      type: 'text',
      status: 'sent',
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    }
    return {
      success: true,
      data: newMessage,
    }
  },
}
