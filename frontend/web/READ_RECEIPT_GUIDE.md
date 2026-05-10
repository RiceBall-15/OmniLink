# 消息已读回执功能使用指南

## 功能概述

前端消息已读回执功能提供了完整的消息状态跟踪和显示系统，包括：
- 消息发送状态显示（发送中、已发送、已送达、已读）
- 已读用户列表查看（群聊场景）
- 自动标记已读（滚动到底部时）
- WebSocket 实时状态更新

## 组件说明

### 1. MessageReadReceipt 组件

消息已读回执组件，显示消息的已读状态和相关信息。

**Props:**
- `message: Message` - 消息对象
- `isOwn: boolean` - 是否为自己的消息
- `isGroup?: boolean` - 是否为群聊（默认 false）
- `readUsers?: ReadUser[]` - 已读用户列表
- `onReadStatusClick?: () => void` - 点击已读状态回调

**使用示例:**
```tsx
<MessageReadReceipt
  message={message}
  isOwn={isOwn}
  isGroup={isGroupChat}
  readUsers={readUsers}
  onReadStatusClick={() => console.log('查看已读列表')}
/>
```

### 2. ReadStatusIndicator 组件

已读状态指示器组件，显示不同状态的图标。

**Props:**
- `status: MessageStatus` - 消息状态
- `clickable?: boolean` - 是否可点击（默认 false）
- `onClick?: () => void` - 点击回调

**状态说明:**
- `sending` - 发送中（旋转动画）
- `sent` - 已发送（灰色单钩）
- `delivered` - 已送达（灰色双钩）
- `read` - 已读（蓝色双钩）
- `failed` - 发送失败（红色错误图标）

### 3. ReadUsersModal 组件

已读用户列表弹窗，显示已读该消息的所有用户。

**Props:**
- `isOpen: boolean` - 是否显示弹窗
- `onClose: () => void` - 关闭回调
- `readUsers: ReadUser[]` - 已读用户列表
- `message: Message` - 消息对象

**ReadUser 接口:**
```typescript
interface ReadUser {
  id: string
  name: string
  avatar?: string
  readAt: string
}
```

### 4. MessageBubble 组件（已集成）

消息气泡组件已集成已读回执功能。

**新增 Props:**
- `isGroup?: boolean` - 是否为群聊（默认 false）
- `readUsers?: ReadUser[]` - 已读用户列表

**使用示例:**
```tsx
<MessageBubble
  message={message}
  isOwn={message.senderId === currentUserId}
  senderName="张三"
  senderAvatar="/avatar.jpg"
  currentUserId={currentUserId}
  isGroup={isGroupChat}
  readUsers={readUsersMap[message.id]}
  onMessageUpdate={handleMessageUpdate}
  onMessageRecall={handleMessageRecall}
  onReply={handleReply}
/>
```

## Hooks 说明

### useMessages Hook（已增强）

消息管理 Hook 已增强自动标记已读功能。

**新增返回值:**
- `autoMarkAsRead: (isAtBottom: boolean) => void` - 自动标记已读函数
- `handleReadReceipt: (wsMessage: WSMessage, currentUserId?: string) => void` - 处理 WebSocket 已读回执

**使用示例:**
```tsx
const { messages, autoMarkAsRead, handleReadReceipt } = useMessages(conversationId)

// 滚动事件处理
const handleScroll = (e: React.UIEvent<HTMLDivElement>) => {
  const target = e.target as HTMLDivElement
  const isAtBottom = target.scrollHeight - target.scrollTop - target.clientHeight < 50
  autoMarkAsRead(isAtBottom)
}

// WebSocket 消息处理
const handleWSMessage = (wsMessage: WSMessage) => {
  handleReadReceipt(wsMessage, currentUserId)
}
```

## 完整集成示例

```tsx
import { useState, useEffect, useCallback } from 'react'
import { MessageBubble } from './components/MessageBubble'
import { useMessages } from './hooks/useMessages'
import { useWebSocket } from './hooks/useMessages'
import type { ReadUser } from './components/MessageReadReceipt'

export function ChatWindow() {
  const [currentUserId] = useState('user-123')
  const [isGroupChat] = useState(false)
  const [readUsersMap, setReadUsersMap] = useState<Record<string, ReadUser[]>>({})
  const conversationId = 'conv-456'

  const { messages, autoMarkAsRead, handleReadReceipt } = useMessages(conversationId)

  // WebSocket 连接
  const { connected } = useWebSocket('ws://localhost:3001/ws', useCallback((wsMessage) => {
    handleReadReceipt(wsMessage, currentUserId)
  }, [handleReadReceipt, currentUserId]))

  // 滚动处理
  const handleScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
    const target = e.target as HTMLDivElement
    const isAtBottom = target.scrollHeight - target.scrollTop - target.clientHeight < 50
    autoMarkAsRead(isAtBottom)
  }, [autoMarkAsRead])

  // 模拟已读用户数据（实际应从 API 获取）
  useEffect(() => {
    if (messages.length > 0) {
      // 这里应该从后端 API 获取已读用户列表
      setReadUsersMap({
        [messages[0].id]: [
          { id: 'user-1', name: '张三', readAt: new Date().toISOString() },
          { id: 'user-2', name: '李四', readAt: new Date().toISOString() },
        ]
      })
    }
  }, [messages])

  return (
    <div className="chat-window" onScroll={handleScroll}>
      <div className="messages-list">
        {messages.map((message) => (
          <MessageBubble
            key={message.id}
            message={message}
            isOwn={message.senderId === currentUserId}
            senderName={message.senderId === currentUserId ? '我' : '张三'}
            currentUserId={currentUserId}
            isGroup={isGroupChat}
            readUsers={readUsersMap[message.id] || []}
            onMessageUpdate={(messageId, updates) => {
              // 处理消息更新
            }}
            onMessageRecall={(messageId) => {
              // 处理消息撤回
            }}
          />
        ))}
      </div>
    </div>
  )
}
```

## WebSocket 消息格式

### 已读回执消息
```typescript
{
  type: 'READ',
  conversationId: string,
  messageId: string,
  senderId: string,
  timestamp: number
}
```

### 消息送达确认
```typescript
{
  type: 'MESSAGE',
  conversationId: string,
  messageId: string,
  senderId: string,
  timestamp: number
}
```

## API 端点

### 标记会话为已读
```
PUT /api/im/conversations/:conversationId/read
Authorization: Bearer {token}
Response: ApiResponse<void>
```

## 样式定制

所有组件都使用 CSS 变量，支持主题定制：

```css
:root {
  --primary-color: #667eea;
  --primary-hover: #764ba2;
  --bg-primary: #ffffff;
  --bg-secondary: #f8f9fa;
  --bg-tertiary: #e9ecef;
  --text-primary: #212529;
  --text-secondary: #6c757d;
  --text-tertiary: #adb5bd;
  --border-color: #dee2e6;
  --border-hover: #ced4da;
  --success-color: #28a745;
  --error-color: #dc3545;
  --info-color: #17a2b8;
  --radius-sm: 4px;
  --radius-md: 8px;
  --radius-lg: 12px;
  --shadow-sm: 0 1px 2px 0 rgba(0, 0, 0, 0.05);
  --shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
  --shadow-lg: 0 10px 15px -3px rgba(0, 0, 0, 0.1);
  --shadow-xl: 0 20px 25px -5px rgba(0, 0, 0, 0.1);
  --transition-fast: 0.15s ease;
  --transition-base: 0.2s ease;
}
```

## 性能优化

1. **防止重复标记**: 使用 `useRef` 跟踪上次标记的消息，避免重复 API 调用
2. **延迟标记**: 滚动到底部后延迟 300ms 再标记，避免频繁调用
3. **WebSocket 实时更新**: 通过 WebSocket 实时更新消息状态，减少轮询
4. **本地状态管理**: 已读用户列表建议在父组件中维护，避免重复请求

## 注意事项

1. 只有自己的消息显示已读回执
2. 群聊场景点击已读状态可查看已读用户列表
3. 单聊场景悬停显示已读时间
4. 自动标记已读仅在滚动到底部时触发
5. WebSocket 连接断开时会自动重连
6. 所有组件都支持响应式设计

## 故障排除

### 已读状态不更新
- 检查 WebSocket 连接状态
- 确认后端正确发送了 READ 类型的消息
- 检查 `handleReadReceipt` 是否正确调用

### 自动标记已读不工作
- 确认滚动检测逻辑正确
- 检查 `autoMarkAsRead` 是否在滚动事件中调用
- 确认会话 ID 正确

### 已读用户列表不显示
- 确认 `readUsers` prop 正确传递
- 检查数据格式是否符合 `ReadUser` 接口
- 确认群聊场景下 `isGroup` prop 为 true
