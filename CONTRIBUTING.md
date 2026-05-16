# 贡献指南

感谢您对 OmniLink 项目的关注！我们欢迎各种形式的贡献。

## 📋 目录

- [行为准则](#行为准则)
- [如何贡献](#如何贡献)
- [开发流程](#开发流程)
- [代码规范](#代码规范)
- [提交规范](#提交规范)
- [Pull Request 流程](#pull-request-流程)
- [问题报告](#问题报告)

## 行为准则

请尊重所有项目参与者，保持友善和专业的交流环境。

## 如何贡献

### 报告 Bug

1. 检查 [Issues](https://github.com/your-org/omnilink/issues) 确认问题未被报告
2. 使用 Bug 报告模板创建新 Issue
3. 提供详细的复现步骤和环境信息

### 提交功能建议

1. 在 Discussions 中创建功能建议帖
2. 描述使用场景和预期行为
3. 等待社区讨论和维护者反馈

### 提交代码

1. Fork 本仓库
2. 创建功能分支
3. 编写代码和测试
4. 提交 Pull Request

## 开发流程

### 环境搭建

```bash
# 1. 克隆仓库
git clone https://github.com/your-org/omnilink.git
cd omnilink

# 2. 安装 Rust 工具链
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable

# 3. 安装 Node.js 依赖
cd frontend/web
npm install
cd ../..

# 4. 配置环境变量
cp .env.example .env
# 编辑 .env 配置数据库等

# 5. 启动开发环境
docker-compose up -d  # 启动数据库
cargo run --bin im-api  # 启动后端
cd frontend/web && npm run dev  # 启动前端
```

### 分支策略

- `master`: 稳定版本分支
- `develop`: 开发分支
- `feature/*`: 功能分支
- `fix/*`: 修复分支
- `release/*`: 发布分支

### 开发新功能

```bash
# 1. 从 develop 创建功能分支
git checkout develop
git pull origin develop
git checkout -b feature/your-feature

# 2. 开发功能
# ... 编写代码 ...

# 3. 运行测试
cargo test
cd frontend/web && npm test

# 4. 提交代码
git add -A
git commit -m "feat(scope): add your feature"

# 5. 推送并创建 PR
git push origin feature/your-feature
```

## 代码规范

### Rust 代码规范

- 使用 `rustfmt` 格式化代码
- 使用 `clippy` 检查代码质量
- 遵循 [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

```bash
# 格式化代码
cargo fmt

# 检查代码质量
cargo clippy -- -D warnings
```

### TypeScript/React 代码规范

- 使用 ESLint + Prettier 格式化代码
- 遵循 React Hooks 最佳实践
- 使用 TypeScript 严格模式

```bash
cd frontend/web

# 检查代码质量
npm run lint

# 格式化代码
npm run format

# 类型检查
npm run type-check
```

### 通用规范

- 变量和函数命名清晰明了
- 避免魔法数字，使用常量
- 编写有意义的注释
- 保持函数简短（单一职责）

## 提交规范

使用 [Conventional Commits](https://www.conventionalcommits.org/) 格式：

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Type 类型

| 类型 | 说明 | 示例 |
|------|------|------|
| `feat` | 新功能 | `feat(auth): add JWT refresh token` |
| `fix` | Bug 修复 | `fix(api): handle null response` |
| `docs` | 文档更新 | `docs(readme): update setup guide` |
| `style` | 代码格式 | `style: fix indentation` |
| `refactor` | 代码重构 | `refactor(services): extract common logic` |
| `test` | 测试相关 | `test(user): add registration tests` |
| `chore` | 构建/工具 | `chore(deps): update dependencies` |
| `perf` | 性能优化 | `perf(query): add database index` |
| `ci` | CI/CD | `ci: add GitHub Actions workflow` |

### Scope 范围

- `api`: API 网关
- `user`: 用户服务
- `msg`: 消息服务
- `ai`: AI 服务
- `file`: 文件服务
- `frontend`: 前端
- `common`: 共享库
- `docs`: 文档
- `ci`: CI/CD

### 示例

```bash
# 简单提交
git commit -m "feat: add user profile page"

# 带范围的提交
git commit -m "feat(auth): implement refresh token rotation"

# 带详细说明的提交
git commit -m "fix(api): handle concurrent websocket connections

- Add connection pool management
- Implement graceful disconnect
- Add reconnection backoff strategy

Fixes #123"
```

## Pull Request 流程

### PR 标题

使用与 commit 相同的格式：

```
feat(scope): description
```

### PR 描述

使用 PR 模板，包含：

1. **变更说明**: 描述此 PR 做了什么
2. **相关 Issue**: 关联的 Issue 编号
3. **测试说明**: 如何测试此变更
4. **截图/录屏**: UI 变更需要提供视觉证据
5. **检查清单**: 完成自查

### 代码审查

- 至少需要 1 位维护者批准
- 所有自动化检查必须通过
- 解决所有审查意见
- 保持 PR 大小合理（< 500 行）

### 合并策略

- 使用 Squash and Merge
- 确保 commit message 符合规范
- 删除合并后的功能分支

## 问题报告

### Bug 报告模板

```markdown
## 描述
简要描述 bug

## 复现步骤
1. 访问 '...'
2. 点击 '...'
3. 滚动到 '...'
4. 看到错误

## 预期行为
描述预期的行为

## 实际行为
描述实际的行为

## 环境信息
- OS: [e.g., Windows 10]
- Browser: [e.g., Chrome 120]
- Node: [e.g., 18.17.0]
- Rust: [e.g., 1.70.0]

## 日志/截图
添加相关日志或截图
```

### 功能建议模板

```markdown
## 问题描述
描述要解决的问题

## 建议方案
描述建议的解决方案

## 替代方案
描述考虑过的替代方案

## 附图
添加相关设计图或示意图
```

## 开发者资源

- [API 文档](docs/api/)
- [架构设计](docs/architecture/)
- [数据库 Schema](docs/database/)

## 获取帮助

- 💬 [GitHub Discussions](https://github.com/your-org/omnilink/discussions)
- 📧 [邮件列表](mailto:dev@omnilink.dev)
- 🐛 [Issue Tracker](https://github.com/your-org/omnilink/issues)

---

感谢您的贡献！🎉
