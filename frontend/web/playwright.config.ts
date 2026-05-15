import { defineConfig, devices } from '@playwright/test';

/**
 * OmniLink E2E 测试配置
 * 
 * @see https://playwright.dev/docs/test-configuration
 */
export default defineConfig({
  testDir: './e2e',
  /* 每个测试最长时间 */
  timeout: 30 * 1000,
  /* 测试间超时 */
  expect: {
    timeout: 10000,
  },
  /* 并行运行测试 */
  fullyParallel: false,
  /* CI 环境下失败不重试 */
  retries: process.env.CI ? 2 : 0,
  /* 并行 worker 数量 */
  workers: 1,
  /* 报告器 */
  reporter: [
    ['html', { outputFolder: 'playwright-report' }],
    ['list'],
  ],
  /* 全局测试设置 */
  use: {
    /* 基础 URL */
    baseURL: process.env.PLAYWRIGHT_BASE_URL || 'http://localhost:5173',
    /* 失败时截图 */
    screenshot: 'only-on-failure',
    /* 失败时录制 trace */
    trace: 'on-first-retry',
    /* 视频录制 */
    video: 'on-first-retry',
  },
  /* 浏览器配置 */
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
    // 可以添加更多浏览器
    // {
    //   name: 'firefox',
    //   use: { ...devices['Desktop Firefox'] },
    // },
    // {
    //   name: 'webkit',
    //   use: { ...devices['Desktop Safari'] },
    // },
  ],
  /* 本地开发服务器配置 */
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:5173',
    reuseExistingServer: !process.env.CI,
    timeout: 120 * 1000,
  },
});
