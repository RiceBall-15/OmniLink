# OmniLink 后端测试覆盖率配置

## 使用 cargo-tarpaulin

### 安装

```bash
cargo install cargo-tarpaulin
```

### 运行覆盖率测试

```bash
# 基本用法
cargo tarpaulin --out Html --output-dir coverage/

# 只测试特定 crate
cargo tarpaulin -p im-api --out Html --output-dir coverage/im-api/

# 排除测试文件和生成的代码
cargo tarpaulin --exclude-files '*/tests/*' --exclude-files '*/target/*' --out Html

# 生成多种格式
cargo tarpaulin --out Html --out Lcov --out Json --output-dir coverage/
```

### 配置文件

创建 `tarpaulin.toml`：

```toml
[all]
exclude-files = ["*/tests/*", "*/target/*", "*/build.rs"]
timeout = 300
fail-under = 50

[im-api]
packages = ["im-api"]

[im-gateway]
packages = ["im-gateway"]

[common]
packages = ["common"]
```

### CI 集成

在 GitHub Actions 中添加覆盖率 job：

```yaml
  coverage:
    name: Test Coverage
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install cargo-tarpaulin
        run: cargo install cargo-tarpaulin
      - name: Run coverage
        run: cargo tarpaulin --out Lcov --output-dir coverage/
      - name: Upload coverage
        uses: codecov/codecov-action@v4
        with:
          files: coverage/lcov.info
```

## 前端测试覆盖率

### 使用 Vitest

在 `frontend/web/vitest.config.ts` 中添加：

```typescript
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      reportsDirectory: './coverage',
      exclude: [
        'node_modules/',
        'src/test/',
        '**/*.d.ts',
        '**/*.test.{ts,tsx}',
        '**/*.spec.{ts,tsx}',
      ],
    },
  },
});
```

### 运行

```bash
cd frontend/web
npm run test:coverage
```

## 覆盖率目标

| 模块 | 最低覆盖率 | 目标覆盖率 |
|------|-----------|-----------|
| common | 60% | 80% |
| im-api handlers | 50% | 70% |
| im-api middleware | 50% | 70% |
| im-gateway | 40% | 60% |
| 前端组件 | 40% | 60% |
