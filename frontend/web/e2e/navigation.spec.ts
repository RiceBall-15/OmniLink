/**
 * E2E 测试示例 - 基础导航测试
 * 
 * 运行: npx playwright test e2e/navigation.spec.ts
 */
import { test, expect } from '@playwright/test';

test.describe('基本导航', () => {
  test('首页加载成功', async ({ page }) => {
    await page.goto('/');
    await expect(page).toHaveTitle(/OmniLink/);
  });

  test('未登录时跳转到登录页', async ({ page }) => {
    await page.goto('/');
    // 应该重定向到登录页
    await expect(page).toHaveURL(/.*auth/);
  });

  test('登录页面包含必要元素', async ({ page }) => {
    await page.goto('/auth');
    // 检查有邮箱输入框
    await expect(page.locator('input[type="email"]')).toBeVisible();
    // 检查有密码输入框
    await expect(page.locator('input[type="password"]')).toBeVisible();
    // 检查有登录按钮
    await expect(page.locator('button[type="submit"]')).toBeVisible();
  });
});

test.describe('API 健康检查', () => {
  test('后端 API 可访问', async ({ request }) => {
    const response = await request.get('http://localhost:3000/health');
    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body).toHaveProperty('status');
  });
});
