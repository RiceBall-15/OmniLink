// 消息类型
export enum MessageType {
  TEXT = 'text',
  IMAGE = 'image',
  FILE = 'file',
  SYSTEM = 'system',
}

// 消息状态
export enum MessageStatus {
  SENDING = 'sending',
  SENT = 'sent',
  DELIVERED = 'delivered',
  READ = 'read',
  FAILED = 'failed',
}

// 消息实体
export interface Message {
  id: string
  conversationId: string
  senderId: string
  content: string
  type: MessageType
  status: MessageStatus
  createdAt: string
  updatedAt: string
  readAt?: string
  replyTo?: string
  metadata?: Record<string, any>
}

// 会话实体
export interface Conversation {
  id: string
  type: 'direct' | 'group' | 'ai'
  name?: string
  avatar?: string
  lastMessage?: Message
  unreadCount: number
  isPinned: boolean
  isMuted: boolean
  createdAt: string
  updatedAt: string
}

// 在线状态
export enum OnlineStatus {
  OFFLINE = 'offline',
  ONLINE = 'online',
  AWAY = 'away',
  BUSY = 'busy',
}
