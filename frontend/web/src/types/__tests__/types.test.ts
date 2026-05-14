/**
 * 类型定义测试 - 验证类型结构和枚举值
 */
import { describe, it, expect } from 'vitest';
import { MessageType, MessageStatus, OnlineStatus, WSMessageType } from '../message';
import type { Message, Conversation, WSMessage, StatusUpdateData } from '../message';
import type { User, RegisterRequest, LoginRequest, LoginResponse, Device } from '../user';

describe('MessageType enum', () => {
  it('has correct values', () => {
    expect(MessageType.TEXT).toBe('text');
    expect(MessageType.IMAGE).toBe('image');
    expect(MessageType.FILE).toBe('file');
    expect(MessageType.SYSTEM).toBe('system');
  });

  it('has exactly 4 members', () => {
    const values = Object.values(MessageType);
    expect(values).toHaveLength(4);
  });
});

describe('MessageStatus enum', () => {
  it('has correct values', () => {
    expect(MessageStatus.SENDING).toBe('sending');
    expect(MessageStatus.SENT).toBe('sent');
    expect(MessageStatus.DELIVERED).toBe('delivered');
    expect(MessageStatus.READ).toBe('read');
    expect(MessageStatus.FAILED).toBe('failed');
  });

  it('has exactly 5 members', () => {
    const values = Object.values(MessageStatus);
    expect(values).toHaveLength(5);
  });
});

describe('OnlineStatus enum', () => {
  it('has correct values', () => {
    expect(OnlineStatus.OFFLINE).toBe('offline');
    expect(OnlineStatus.ONLINE).toBe('online');
    expect(OnlineStatus.AWAY).toBe('away');
    expect(OnlineStatus.BUSY).toBe('busy');
  });

  it('has exactly 4 members', () => {
    const values = Object.values(OnlineStatus);
    expect(values).toHaveLength(4);
  });
});

describe('WSMessageType enum', () => {
  it('has correct values', () => {
    expect(WSMessageType.CONNECT).toBe('connect');
    expect(WSMessageType.CONNECTED).toBe('connected');
    expect(WSMessageType.MESSAGE).toBe('message');
    expect(WSMessageType.NEW_MESSAGE).toBe('new_message');
    expect(WSMessageType.PING).toBe('ping');
    expect(WSMessageType.PONG).toBe('pong');
    expect(WSMessageType.TYPING).toBe('typing');
    expect(WSMessageType.READ).toBe('read');
    expect(WSMessageType.ERROR).toBe('error');
    expect(WSMessageType.STATUS_UPDATE).toBe('status_update');
  });

  it('has exactly 10 members', () => {
    const values = Object.values(WSMessageType);
    expect(values).toHaveLength(10);
  });
});

describe('Message interface', () => {
  it('can create a valid Message object', () => {
    const message: Message = {
      id: 'msg-1',
      conversationId: 'conv-1',
      senderId: 'user-1',
      content: 'Hello',
      type: MessageType.TEXT,
      status: MessageStatus.SENT,
      createdAt: '2026-01-01T00:00:00Z',
      updatedAt: '2026-01-01T00:00:00Z',
    };

    expect(message.id).toBe('msg-1');
    expect(message.type).toBe(MessageType.TEXT);
    expect(message.status).toBe(MessageStatus.SENT);
  });

  it('supports optional fields', () => {
    const message: Message = {
      id: 'msg-1',
      conversationId: 'conv-1',
      senderId: 'user-1',
      content: 'Hello',
      type: MessageType.TEXT,
      status: MessageStatus.READ,
      createdAt: '2026-01-01T00:00:00Z',
      updatedAt: '2026-01-01T00:00:00Z',
      readAt: '2026-01-01T00:01:00Z',
      replyTo: 'msg-0',
      metadata: { forwarded: true },
      isRecalled: false,
      recalledAt: undefined,
    };

    expect(message.readAt).toBe('2026-01-01T00:01:00Z');
    expect(message.replyTo).toBe('msg-0');
    expect(message.metadata?.forwarded).toBe(true);
  });
});

describe('Conversation interface', () => {
  it('can create a valid Conversation object', () => {
    const conversation: Conversation = {
      id: 'conv-1',
      type: 'direct',
      unreadCount: 3,
      isPinned: false,
      isMuted: false,
      createdAt: '2026-01-01T00:00:00Z',
      updatedAt: '2026-01-01T00:00:00Z',
    };

    expect(conversation.type).toBe('direct');
    expect(conversation.unreadCount).toBe(3);
  });

  it('supports group type', () => {
    const conversation: Conversation = {
      id: 'conv-2',
      type: 'group',
      name: 'Test Group',
      unreadCount: 0,
      isPinned: true,
      isMuted: false,
      createdAt: '2026-01-01T00:00:00Z',
      updatedAt: '2026-01-01T00:00:00Z',
    };

    expect(conversation.type).toBe('group');
    expect(conversation.name).toBe('Test Group');
    expect(conversation.isPinned).toBe(true);
  });

  it('supports ai type', () => {
    const conversation: Conversation = {
      id: 'conv-3',
      type: 'ai',
      name: 'AI Assistant',
      unreadCount: 0,
      isPinned: false,
      isMuted: false,
      createdAt: '2026-01-01T00:00:00Z',
      updatedAt: '2026-01-01T00:00:00Z',
    };

    expect(conversation.type).toBe('ai');
  });
});

describe('User types', () => {
  it('can create a valid User object', () => {
    const user: User = {
      id: 'user-1',
      username: 'testuser',
      email: 'test@example.com',
      createdAt: '2026-01-01T00:00:00Z',
      updatedAt: '2026-01-01T00:00:00Z',
    };

    expect(user.username).toBe('testuser');
    expect(user.email).toBe('test@example.com');
  });

  it('supports optional fields', () => {
    const user: User = {
      id: 'user-1',
      username: 'testuser',
      email: 'test@example.com',
      avatar: 'https://example.com/avatar.png',
      onlineStatus: 'online',
      createdAt: '2026-01-01T00:00:00Z',
      updatedAt: '2026-01-01T00:00:00Z',
    };

    expect(user.avatar).toBe('https://example.com/avatar.png');
    expect(user.onlineStatus).toBe('online');
  });

  it('can create RegisterRequest', () => {
    const request: RegisterRequest = {
      username: 'newuser',
      email: 'new@example.com',
      password: 'password123',
    };

    expect(request.username).toBe('newuser');
    expect(request.email).toBe('new@example.com');
  });

  it('can create LoginRequest', () => {
    const request: LoginRequest = {
      email: 'test@example.com',
      password: 'password123',
    };

    expect(request.email).toBe('test@example.com');
  });

  it('can create LoginResponse', () => {
    const response: LoginResponse = {
      token: 'jwt-token-123',
      user: {
        id: 'user-1',
        username: 'testuser',
        email: 'test@example.com',
        createdAt: '2026-01-01T00:00:00Z',
        updatedAt: '2026-01-01T00:00:00Z',
      },
    };

    expect(response.token).toBe('jwt-token-123');
    expect(response.user.username).toBe('testuser');
  });

  it('can create Device', () => {
    const device: Device = {
      id: 'device-1',
      userId: 'user-1',
      deviceType: 'web',
      deviceName: 'Chrome',
      platform: 'Windows',
      lastActiveAt: '2026-01-01T00:00:00Z',
      createdAt: '2026-01-01T00:00:00Z',
    };

    expect(device.deviceType).toBe('web');
    expect(device.platform).toBe('Windows');
  });
});

describe('WSMessage interface', () => {
  it('can create a valid WSMessage', () => {
    const wsMessage: WSMessage = {
      type: WSMessageType.MESSAGE,
      conversationId: 'conv-1',
      senderId: 'user-1',
      content: 'Hello via WS',
      timestamp: Date.now(),
    };

    expect(wsMessage.type).toBe(WSMessageType.MESSAGE);
    expect(wsMessage.content).toBe('Hello via WS');
  });

  it('supports optional fields', () => {
    const wsMessage: WSMessage = {
      type: WSMessageType.PING,
    };

    expect(wsMessage.type).toBe(WSMessageType.PING);
    expect(wsMessage.conversationId).toBeUndefined();
  });
});

describe('StatusUpdateData interface', () => {
  it('can create a valid StatusUpdateData', () => {
    const data: StatusUpdateData = {
      userId: 'user-1',
      status: OnlineStatus.ONLINE,
      timestamp: Date.now(),
    };

    expect(data.status).toBe(OnlineStatus.ONLINE);
  });
});
