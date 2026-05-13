# OmniLink 移动端开发方案

**文档版本：** v1.0  
**更新日期：** 2026-05-13  
**技术栈：** React Native + Expo

---

## 📱 技术选型

### 为什么选择 React Native + Expo？

| 方案 | 优势 | 劣势 | 推荐度 |
|------|------|------|--------|
| **React Native + Expo** | ✅ 与现有React代码复用高<br>✅ Expo简化配置<br>✅ 热更新支持<br>✅ 生态成熟 | 性能略低于原生 | ⭐⭐⭐⭐⭐ |
| Flutter | 性能好、UI一致 | 需要学习Dart | ⭐⭐⭐⭐ |
| 原生开发 | 性能最佳 | 开发成本高 | ⭐⭐⭐ |

**最终选择：React Native + Expo**
- 理由：现有Web端使用React/TypeScript，代码复用率可达60%+
- Expo简化了原生配置、推送通知、OTA更新等复杂功能

---

## 🎨 UI设计参考

### 设计风格选择

基于 **popular-web-designs** 技能的54个真实设计系统，推荐以下风格：

#### 推荐风格：**Linear 风格**
- **特点：** 极简暗黑模式、精准紫色点缀、工具感强
- **适合：** 效率工具、专业应用
- **优势：** 高端感、辨识度高、符合IM工具定位

#### 备选风格：
1. **Discord 风格** - 社交感强、圆角设计、深色背景
2. **Telegram 风格** - 简洁清爽、蓝色主题、功能导向
3. **Slack 风格** - 专业商务、多彩标识、清晰层级

### 设计Token（Linear风格）

```css
/* 主色调 */
--color-bg-primary: #0a0a0b;
--color-bg-secondary: #131316;
--color-bg-tertiary: #1c1c21;
--color-bg-hover: #242428;

/* 文字颜色 */
--color-text-primary: #f5f5f7;
--color-text-secondary: #8b8b8b;
--color-text-muted: #5c5c5c;

/* 强调色 */
--color-accent: #8b5cf6;  /* 紫色 */
--color-accent-hover: #7c3aed;

/* 状态色 */
--color-success: #22c55e;
--color-warning: #f59e0b;
--color-error: #ef4444;

/* 字体 */
--font-sans: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
--font-mono: 'JetBrains Mono', 'Fira Code', monospace;

/* 圆角 */
--radius-sm: 6px;
--radius-md: 8px;
--radius-lg: 12px;
--radius-full: 9999px;

/* 阴影 */
--shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.3);
--shadow-md: 0 4px 6px rgba(0, 0, 0, 0.4);
--shadow-lg: 0 10px 15px rgba(0, 0, 0, 0.5);
```

---

## 🏗️ 项目结构

```
mobile/
├── app/                        # Expo Router 页面
│   ├── (tabs)/                # 底部标签页
│   │   ├── _layout.tsx       # 标签页布局
│   │   ├── index.tsx         # 消息列表（主页）
│   │   ├── contacts.tsx      # 通讯录
│   │   ├── discover.tsx      # 发现
│   │   └── profile.tsx       # 我的
│   ├── chat/                  # 聊天页面
│   │   └── [id].tsx          # 聊天详情
│   ├── login.tsx             # 登录
│   └── register.tsx          # 注册
├── src/
│   ├── components/            # 可复用组件
│   │   ├── ui/               # 基础UI组件
│   │   │   ├── Button.tsx
│   │   │   ├── Input.tsx
│   │   │   ├── Avatar.tsx
│   │   │   └── Badge.tsx
│   │   ├── chat/             # 聊天相关组件
│   │   │   ├── MessageBubble.tsx
│   │   │   ├── MessageList.tsx
│   │   │   ├── ChatInput.tsx
│   │   │   └── TypingIndicator.tsx
│   │   └── common/           # 通用组件
│   │       ├── Header.tsx
│   │       ├── TabBar.tsx
│   │       └── SearchBar.tsx
│   ├── hooks/                # 自定义Hooks
│   │   ├── useAuth.ts
│   │   ├── useChat.ts
│   │   ├── useWebSocket.ts
│   │   └── useNotifications.ts
│   ├── services/             # API服务
│   │   ├── api.ts
│   │   ├── auth.ts
│   │   ├── chat.ts
│   │   └── websocket.ts
│   ├── stores/               # 状态管理（Zustand）
│   │   ├── authStore.ts
│   │   ├── chatStore.ts
│   │   └── uiStore.ts
│   ├── types/                # TypeScript类型
│   │   ├── user.ts
│   │   ├── message.ts
│   │   └── conversation.ts
│   ├── utils/                # 工具函数
│   │   ├── format.ts
│   │   ├── storage.ts
│   │   └── validation.ts
│   ├── constants/            # 常量
│   │   ├── colors.ts
│   │   ├── config.ts
│   │   └── dimensions.ts
│   └── theme/                # 主题配置
│       ├── index.ts
│       └── tokens.ts
├── assets/                   # 静态资源
│   ├── fonts/
│   ├── images/
│   └── icons/
├── app.json                  # Expo配置
├── package.json
├── tsconfig.json
└── babel.config.js
```

---

## 📦 依赖包

### 核心依赖

```json
{
  "dependencies": {
    "expo": "~52.0.0",
    "expo-router": "~4.0.0",
    "react": "18.2.0",
    "react-native": "0.76.0",
    "expo-status-bar": "~2.0.0",
    "expo-constants": "~16.0.0",
    "expo-linking": "~7.0.0",
    "expo-secure-store": "~13.0.0",
    "expo-notifications": "~0.28.0",
    "expo-image-picker": "~15.0.0",
    "expo-camera": "~15.0.0",
    "expo-av": "~14.0.0"
  }
}
```

### UI和动画

```json
{
  "dependencies": {
    "@expo/vector-icons": "^14.0.0",
    "react-native-reanimated": "~3.16.0",
    "react-native-gesture-handler": "~2.20.0",
    "react-native-safe-area-context": "4.12.0",
    "react-native-screens": "~4.0.0",
    "@gorhom/bottom-sheet": "^5.0.0",
    "react-native-svg": "15.8.0",
    "moti": "^0.28.0"
  }
}
```

### 状态管理和网络

```json
{
  "dependencies": {
    "zustand": "^4.5.0",
    "axios": "^1.7.0",
    "@tanstack/react-query": "^5.0.0",
    "react-native-mmkv": "^3.0.0"
  }
}
```

---

## 🚀 开发步骤

### Step 1: 环境准备

在**本地电脑**执行（服务器资源不足）：

```bash
# 1. 安装 Expo CLI
npm install -g expo-cli

# 2. 创建项目
cd /root/omnilink
npx create-expo-app@latest mobile --template tabs

# 3. 进入项目目录
cd mobile

# 4. 安装依赖
npm install
```

### Step 2: 配置项目

**app.json 配置：**

```json
{
  "expo": {
    "name": "OmniLink",
    "slug": "omnilink",
    "version": "1.0.0",
    "orientation": "portrait",
    "icon": "./assets/icon.png",
    "userInterfaceStyle": "dark",
    "splash": {
      "image": "./assets/splash.png",
      "resizeMode": "contain",
      "backgroundColor": "#0a0a0b"
    },
    "assetBundlePatterns": ["**/*"],
    "ios": {
      "supportsTablet": true,
      "bundleIdentifier": "com.omnilink.app"
    },
    "android": {
      "adaptiveIcon": {
        "foregroundImage": "./assets/adaptive-icon.png",
        "backgroundColor": "#0a0a0b"
      },
      "package": "com.omnilink.app"
    },
    "plugins": [
      "expo-router",
      "expo-secure-store",
      "expo-notifications"
    ]
  }
}
```

### Step 3: 实现核心功能

#### 3.1 认证系统

```typescript
// src/hooks/useAuth.ts
import { create } from 'zustand'
import * as SecureStore from 'expo-secure-store'
import { api } from '../services/api'

interface AuthState {
  user: User | null
  token: string | null
  isAuthenticated: boolean
  login: (email: string, password: string) => Promise<void>
  logout: () => Promise<void>
  checkAuth: () => Promise<void>
}

export const useAuth = create<AuthState>((set) => ({
  user: null,
  token: null,
  isAuthenticated: false,

  login: async (email, password) => {
    const response = await api.post('/auth/login', { email, password })
    const { token, user } = response.data
    
    await SecureStore.setItemAsync('token', token)
    set({ user, token, isAuthenticated: true })
  },

  logout: async () => {
    await SecureStore.deleteItemAsync('token')
    set({ user: null, token: null, isAuthenticated: false })
  },

  checkAuth: async () => {
    const token = await SecureStore.getItemAsync('token')
    if (!token) return

    try {
      const response = await api.get('/user/me', {
        headers: { Authorization: `Bearer ${token}` }
      })
      set({ user: response.data, token, isAuthenticated: true })
    } catch {
      await SecureStore.deleteItemAsync('token')
    }
  }
}))
```

#### 3.2 WebSocket连接

```typescript
// src/services/websocket.ts
import { useEffect, useRef, useCallback } from 'react'
import { useAuth } from '../hooks/useAuth'
import { useChatStore } from '../stores/chatStore'

const WS_URL = 'wss://api.omnilink.com/ws'

export function useWebSocket() {
  const ws = useRef<WebSocket | null>(null)
  const { token } = useAuth()
  const { addMessage, updateOnlineStatus } = useChatStore()

  const connect = useCallback(() => {
    if (!token) return

    ws.current = new WebSocket(`${WS_URL}?token=${token}`)

    ws.current.onopen = () => {
      console.log('WebSocket connected')
      // 发送心跳
      setInterval(() => {
        ws.current?.send(JSON.stringify({ type: 'ping' }))
      }, 30000)
    }

    ws.current.onmessage = (event) => {
      const data = JSON.parse(event.data)
      
      switch (data.type) {
        case 'new_message':
          addMessage(data.conversation_id, data.message)
          break
        case 'online_status':
          updateOnlineStatus(data.user_id, data.status)
          break
        case 'typing':
          // 处理输入状态
          break
      }
    }

    ws.current.onclose = () => {
      console.log('WebSocket closed, reconnecting...')
      setTimeout(connect, 3000)
    }

    ws.current.onerror = (error) => {
      console.error('WebSocket error:', error)
    }
  }, [token])

  useEffect(() => {
    connect()
    return () => ws.current?.close()
  }, [connect])

  const sendMessage = useCallback((conversationId: string, content: string) => {
    ws.current?.send(JSON.stringify({
      type: 'message',
      conversation_id: conversationId,
      content
    }))
  }, [])

  return { sendMessage }
}
```

#### 3.3 消息列表组件

```typescript
// src/components/chat/MessageBubble.tsx
import React from 'react'
import { View, Text, StyleSheet } from 'react-native'
import { colors } from '../../constants/colors'

interface MessageBubbleProps {
  message: Message
  isOwn: boolean
}

export function MessageBubble({ message, isOwn }: MessageBubbleProps) {
  return (
    <View style={[styles.container, isOwn ? styles.own : styles.other]}>
      <View style={[styles.bubble, isOwn ? styles.ownBubble : styles.otherBubble]}>
        <Text style={styles.content}>{message.content}</Text>
        <Text style={styles.time}>
          {new Date(message.createdAt).toLocaleTimeString('zh-CN', {
            hour: '2-digit',
            minute: '2-digit'
          })}
        </Text>
      </View>
    </View>
  )
}

const styles = StyleSheet.create({
  container: {
    paddingHorizontal: 16,
    marginVertical: 4,
  },
  own: {
    alignItems: 'flex-end',
  },
  other: {
    alignItems: 'flex-start',
  },
  bubble: {
    maxWidth: '80%',
    paddingHorizontal: 12,
    paddingVertical: 8,
    borderRadius: 16,
  },
  ownBubble: {
    backgroundColor: colors.accent,
    borderBottomRightRadius: 4,
  },
  otherBubble: {
    backgroundColor: colors.bgTertiary,
    borderBottomLeftRadius: 4,
  },
  content: {
    color: colors.textPrimary,
    fontSize: 16,
    lineHeight: 22,
  },
  time: {
    color: colors.textMuted,
    fontSize: 11,
    marginTop: 4,
    alignSelf: 'flex-end',
  },
})
```

---

## 🎯 开发优先级

### P0 - 核心功能（第1周）
- [x] 项目初始化和配置
- [ ] 登录/注册页面
- [ ] 消息列表页面
- [ ] 聊天详情页面
- [ ] WebSocket连接

### P1 - 基础功能（第2周）
- [ ] 通讯录页面
- [ ] 消息发送（文字/图片）
- [ ] 推送通知
- [ ] 在线状态显示

### P2 - 增强功能（第3周）
- [ ] 语音消息
- [ ] 文件发送
- [ ] 消息搜索
- [ ] 群聊功能

### P3 - 高级功能（第4周）
- [ ] 朋友圈/动态
- [ ] 表情包系统
- [ ] 主题切换
- [ ] 消息定时发送

---

## 📝 开发命令

```bash
# 启动开发服务器
cd mobile
npx expo start

# 启动iOS模拟器
npx expo start --ios

# 启动Android模拟器
npx expo start --android

# 构建生产版本
npx eas build --platform ios
npx eas build --platform android

# OTA热更新
npx expo publish
```

---

## 🔗 相关资源

- **Expo文档：** https://docs.expo.dev
- **React Native文档：** https://reactnative.dev
- **Expo Router：** https://expo.github.io/router
- **Zustand状态管理：** https://github.com/pmndrs/zustand
- **React Query：** https://tanstack.com/query

---

## ⚠️ 注意事项

1. **服务器资源限制** - 移动端开发在本地电脑进行，服务器不适合
2. **代码复用** - 优先复用Web端的类型定义、API服务、业务逻辑
3. **测试设备** - 准备iOS和Android真机进行测试
4. **推送证书** - 需要配置APNs（iOS）和FCM（Android）推送证书
5. **应用商店** - 需要Apple Developer（$99/年）和Google Play（$25）账号

---

*文档生成时间：2026-05-13*
