# 消息编辑功能使用指南

## 功能概述

消息编辑功能提供了完整的消息编辑体验，包括：
- 双击消息气泡进入编辑模式
- 右键菜单支持复制、编辑、撤回、回复操作
- 编辑时间限制（默认2分钟）
- 撤回时间限制（默认2分钟）
- 编辑历史记录（撤销/重做）
- 快捷键支持

## 组件说明

### 1. MessageEdit 组件

消息编辑组件，提供消息编辑界面。

**Props:**
```typescript
interface MessageEditProps {
  initialContent: string           // 初始消息内容
  onSave: (content: string) => Promise<void>  // 保存回调
  onCancel: () => void            // 取消回调
  isSaving?: boolean               // 是否正在保存
  editTimeLimit?: number           // 编辑时间限制（分钟），默认2分钟
  messageCreatedAt: string         // 消息创建时间
  disabled?: boolean               // 是否禁用编辑
}
```

**快捷键:**
- `Esc`: 取消编辑
- `Ctrl+Enter`: 保存
- `Ctrl+Z`: 撤销
- `Ctrl+Shift+Z` 或 `Ctrl+Y`: 重做

**使用示例:**
```typescript
import { MessageEdit } from './components/MessageEdit'

function MyComponent() {
  const [isEditing, setIsEditing] = useState(false)

  const handleSave = async (content: string) => {
    await messageService.editMessage(messageId, content)
    setIsEditing(false)
  }

  if (isEditing) {
    return (
      <MessageEdit
        initialContent={message.content}
        onSave={handleSave}
        onCancel={() => setIsEditing(false)}
        messageCreatedAt={message.createdAt}
      />
    )
  }

  return <div>{message.content}</div>
}
```

### 2. MessageContextMenu 组件

右键菜单组件，提供消息操作菜单。

**Props:**
```typescript
interface MessageContextMenuProps {
  message: Message                    // 消息对象
  visible: boolean                    // 是否显示菜单
  position: { x: number; y: number }  // 菜单位置
  onClose: () => void                 // 关闭菜单回调
  onCopy: () => void                  // 复制回调
  onEdit: () => void                  // 编辑回调
  onRecall: () => void                // 撤回回调
  onReply: () => void                 // 回复回调
  currentUserId: string               // 当前用户ID
  senderId: string                    // 消息发送者ID
  canEdit?: boolean                   // 编辑是否可用
  canRecall?: boolean                 // 撤回是否可用
}
```

**使用示例:**
```typescript
import { MessageContextMenu } from './components/MessageContextMenu'

function MyComponent() {
  const [showMenu, setShowMenu] = useState(false)
  const [menuPosition, setMenuPosition] = useState({ x: 0, y: 0 })

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault()
    setMenuPosition({ x: e.clientX, y: e.clientY })
    setShowMenu(true)
  }

  return (
    <>
      <div onContextMenu={handleContextMenu}>消息内容</div>
      <MessageContextMenu
        visible={showMenu}
        position={menuPosition}
        onClose={() => setShowMenu(false)}
        onCopy={handleCopy}
        onEdit={handleEdit}
        onRecall={handleRecall}
        onReply={handleReply}
        currentUserId={userId}
        senderId={message.senderId}
        canEdit={canEdit()}
        canRecall={canRecall()}
      />
    </>
  )
}
```

### 3. MessageBubble 组件（已集成编辑功能）

消息气泡组件，已集成完整的编辑功能。

**Props:**
```typescript
interface MessageBubbleProps {
  message: Message                   // 消息对象
  isOwn: boolean                     // 是否为自己的消息
  senderName?: string                // 发送者显示名称
  senderAvatar?: string              // 发送者头像
  currentUserId: string              // 当前用户ID
  editTimeLimit?: number             // 编辑时间限制（分钟），默认2
  recallTimeLimit?: number           // 撤回时间限制（分钟），默认2
  onMessageUpdate?: (messageId: string, updates: Partial<Message>) => void
  onMessageRecall?: (messageId: string) => void
  onReply?: (messageId: string) => void
}
```

**使用示例:**
```typescript
import { MessageBubble } from './components/MessageBubble'

function ChatMessage({ message, userId }: { message: Message, userId: string }) {
  const handleUpdate = (messageId: string, updates: Partial<Message>) => {
    // 更新本地消息状态
  }

  const handleRecall = (messageId: string) => {
    // 处理消息撤回
  }

  const handleReply = (messageId: string) => {
    // 处理消息回复
  }

  return (
    <MessageBubble
      message={message}
      isOwn={message.senderId === userId}
      currentUserId={userId}
      senderName="张三"
      onMessageUpdate={handleUpdate}
      onMessageRecall={handleRecall}
      onReply={handleReply}
    />
  )
}
```

### 4. useMessages Hook（已更新）

消息管理 Hook，已添加编辑、撤回等功能。

**返回值:**
```typescript
{
  messages: Message[]
  loading: boolean
  error: string | null
  loadMessages: (conversationId: string) => Promise<void>
  sendMessage: (content: string) => Promise<Message | null>
  addMessage: (message: Message) => void
  updateMessage: (messageId: string, updates: Partial<Message>) => void
  editMessage: (messageId: string, content: string) => Promise<Message | null>  // 新增
  recallMessage: (messageId: string) => Promise<boolean>  // 新增
  markAsRead: () => Promise<boolean>
  canEditMessage: (message: Message, currentUserId: string, editTimeLimit?: number) => boolean  // 新增
  canRecallMessage: (message: Message, currentUserId: string, recallTimeLimit?: number) => boolean  // 新增
}
```

**使用示例:**
```typescript
import { useMessages } from './hooks/useMessages'

function ChatView({ conversationId, userId }: { conversationId: string, userId: string }) {
  const {
    messages,
    loading,
    editMessage,
    recallMessage,
    canEditMessage,
    canRecallMessage,
  } = useMessages(conversationId)

  const handleMessageUpdate = async (messageId: string, updates: Partial<Message>) => {
    // 本地更新消息状态
  }

  const handleRecall = async (messageId: string) => {
    await recallMessage(messageId)
  }

  if (loading) return <div>加载中...</div>

  return (
    <div>
      {messages.map((message) => (
        <MessageBubble
          key={message.id}
          message={message}
          isOwn={message.senderId === userId}
          currentUserId={userId}
          onMessageUpdate={handleMessageUpdate}
          onMessageRecall={handleRecall}
        />
      ))}
    </div>
  )
}
```

## API 说明

### 消息服务 (messageService)

已实现的 API 方法：

```typescript
// 编辑消息
editMessage: async (messageId: string, content: string): Promise<ApiResponse<Message>>

// 撤回消息
recallMessage: async (messageId: string): Promise<ApiResponse<void>>
```

**API 端点:**
- 编辑消息: `PUT /api/im/messages/:messageId`
  - 请求体: `{ content: string }`
  - 响应: `ApiResponse<Message>`

- 撤回消息: `PUT /api/im/messages/:messageId/recall`
  - 响应: `ApiResponse<void>`

## 功能特性

### 1. 编辑时间限制
- 默认2分钟内可以编辑消息
- 超时后编辑功能自动禁用
- 可以通过 `editTimeLimit` prop 自定义时间限制

### 2. 编辑历史记录
- 支持撤销（Ctrl+Z）和重做（Ctrl+Shift+Z / Ctrl+Y）
- 记录编辑历史，方便回退操作

### 3. 快捷键支持
- `Esc`: 取消编辑
- `Ctrl+Enter`: 保存编辑
- `Ctrl+Z`: 撤销
- `Ctrl+Shift+Z` / `Ctrl+Y`: 重做

### 4. 响应式设计
- 自适应移动端和桌面端
- 菜单位置自动调整，防止超出视口

### 5. 权限控制
- 只能编辑自己的消息
- 只能撤回自己的消息
- 编辑和撤回有时间限制

### 6. 用户反馈
- 加载状态显示
- 编辑成功/失败提示（需要集成 Toast 组件）
- 超时提示
- 编辑标识显示

## 样式定制

所有组件都支持通过 CSS 自定义样式：

```css
/* 编辑组件样式 */
.message-edit-container { ... }
.message-edit-textarea { ... }
.message-edit-btn { ... }
.message-edit-btn-save { ... }
.message-edit-btn-cancel { ... }

/* 右键菜单样式 */
.message-context-menu { ... }
.message-context-menu-item { ... }
.message-context-menu-item-danger { ... }

/* 消息气泡样式 */
.message-bubble { ... }
.message-text { ... }
.message-edited { ... }
```

## 注意事项

1. **服务层**: `messageService.editMessage` 已实现，确保后端 API 正确配置
2. **时间限制**: 编辑和撤回时间限制基于客户端时间，请确保服务器时间同步
3. **错误处理**: 建议集成 Toast 组件显示错误提示
4. **性能优化**: 编辑历史记录可以使用防抖优化（当前实现为每次更改都记录）
5. **Mock 模式**: 在 Mock 模式下，编辑操作会在内存中模拟，不会真正发送请求

## 文件清单

```
components/
  MessageEdit.tsx          # 消息编辑组件
  MessageEdit.css          # 消息编辑样式
  MessageContextMenu.tsx   # 右键菜单组件
  MessageContextMenu.css   # 右键菜单样式
  MessageBubble.tsx        # 消息气泡组件（已更新）
  MessageBubble.css        # 消息气泡样式（已更新）

hooks/
  useMessages.ts           # 消息管理 Hook（已更新）

services/
  messageService.ts        # 消息服务（已更新）
```

## 测试建议

1. 测试双击消息进入编辑模式
2. 测试右键菜单各项功能
3. 测试快捷键（Esc、Ctrl+Enter、Ctrl+Z、Ctrl+Shift+Z）
4. 测试编辑时间限制
5. 测试保存和取消功能
6. 测试编辑历史记录（撤销/重做）
7. 测试权限控制（只能编辑/撤回自己的消息）
8. 测试响应式设计（移动端和桌面端）
9. 测试菜单位置自动调整
10. 测试错误处理和网络异常情况
