# OmniLink Web 前端

## 项目概述

OmniLink Web前端是基于React + TypeScript + Vite构建的现代化Web应用，提供完整的IM + AI对话体验。

## 技术栈

- **框架**: React 18 + TypeScript
- **构建工具**: Vite 5
- **样式**: CSS (可升级到Tailwind CSS)
- **状态管理**: React Hooks (可升级到Zustand)
- **路由**: (待添加React Router)
- **HTTP客户端**: Fetch API (可升级到Axios)

## 项目结构

```
web/
├── src/
│   ├── components/      # 可复用组件
│   ├── pages/          # 页面组件
│   │   ├── AuthPage.tsx       # 登录/注册页面
│   │   ├── ChatPage.tsx       # 聊天主页面
│   │   └── *.css              # 页面样式
│   ├── services/       # API服务层
│   │   ├── api.ts              # API基础配置
│   │   ├── userService.ts      # 用户服务
│   │   ├── messageService.ts   # 消息服务
│   │   └── aiService.ts        # AI服务
│   ├── hooks/          # 自定义Hooks
│   │   ├── useAuth.ts          # 认证Hook
│   │   └── useMessages.ts      # 消息/会话Hook
│   ├── types/          # TypeScript类型定义
│   │   ├── user.ts             # 用户类型
│   │   ├── message.ts          # 消息类型
│   │   └── ai.ts               # AI类型
│   ├── utils/          # 工具函数
│   ├── App.tsx         # 根组件
│   ├── main.tsx        # 入口文件
│   └── index.css       # 全局样式
├── public/             # 静态资源
├── index.html          # HTML模板
├── vite.config.ts      # Vite配置
├── tsconfig.json       # TypeScript配置
└── package.json        # 依赖配置
```

## 快速开始

### 安装依赖

```bash
cd /root/omnilink/frontend/web
npm install
# 或使用pnpm
pnpm install
```

### 启动开发服务器

```bash
npm run dev
# 或
pnpm dev
```

应用将在 http://localhost:3000 启动

### 构建生产版本

```bash
npm run build
# 或
pnpm build
```

## 功能特性

### ✅ 已实现

- [x] React + TypeScript基础架构
- [x] Vite构建配置
- [x] 类型定义（用户/消息/AI）
- [x] API服务层
- [x] 自定义React Hooks
- [x] 登录/注册页面
- [x] 聊天主界面布局
- [x] 会话列表展示
- [x] 用户信息展示
- [x] 响应式设计
- [x] WebSocket连接管理

### 🚧 待开发

- [ ] React Router路由配置
- [ ] 完整的消息收发功能
- [ ] AI对话集成
- [ ] 文件上传/下载
- [ ] 消息撤回/编辑
- [ ] 在线状态显示
- [ ] 已读回执
- [ ] 推送通知集成
- [ ] 设置页面
- [ ] 个人资料编辑
- [ ] 搜索功能
- [ ] 深色模式
- [ ] 国际化支持

## API集成

前端通过以下端口连接后端服务：

| 服务 | 端口 | 说明 |
|------|------|------|
| im-api | 8002 | REST API |
| im-gateway | 8001 | WebSocket |
| ai-service | 8003 | AI服务 |

所有API请求会自动添加JWT Token进行认证。

## 开发规范

### 代码风格
- 使用TypeScript严格模式
- 组件使用函数式组件
- 使用React Hooks管理状态
- 使用语义化HTML标签

### 命名规范
- 组件: PascalCase (如 `ChatPage.tsx`)
- 工具函数: camelCase (如 `formatDate.ts`)
- 类型: PascalCase接口 (如 `interface User`)
- 样式: kebab-case (如 `chat-page.css`)

### 提交规范
- feat: 新功能
- fix: 修复bug
- docs: 文档更新
- style: 代码格式调整
- refactor: 重构
- test: 测试相关
- chore: 构建/工具相关

## 性能优化

- [ ] 代码分割
- [ ] 图片懒加载
- [ ] 虚拟列表（长消息列表）
- [ ] 防抖/节流
- [ ] 缓存策略
- [ ] Service Worker (PWA)

## 浏览器支持

- Chrome (最新版)
- Firefox (最新版)
- Safari (最新版)
- Edge (最新版)

## 后续优化建议

1. **UI组件库**: 集成shadcn/ui或Ant Design
2. **状态管理**: 引入Zustand或Jotai
3. **表单验证**: 集成React Hook Form + Zod
4. **请求库**: 升级到Axios或TanStack Query
5. **CSS框架**: 升级到Tailwind CSS
6. **国际化**: 集成i18next
7. **测试**: 添加Vitest + Testing Library
8. **E2E测试**: 添加Playwright

## 相关文档

- [OmniLink项目总览](../README.md)
- [后端API文档](../docs/api.md)
- [开发计划](../PROJECT_PLAN.md)
- [进度跟踪](../PROGRESS.md)

## 贡献指南

1. Fork项目
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'feat: 添加某功能'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启Pull Request

## 许可证

MIT License

---

**状态**: 🚧 开发中 (基础架构已完成，核心功能开发进行中)
