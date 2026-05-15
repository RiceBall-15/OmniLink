import { test, expect } from '@playwright/test';

/**
 * OmniLink 冒烟测试
 * 验证核心页面和功能的基本可用性
 */

test.describe('OmniLink Smoke Tests', () => {
  test('首页加载成功', async ({ page }) => {
    await page.goto('/');
    // 验证页面标题包含 OmniLink
    await expect(page).toHaveTitle(/OmniLink/i);
  });

  test('登录页面可访问', async ({ page }) => {
    await page.goto('/login');
    // 验证登录表单存在
    const loginForm = page.locator('form, [data-testid="login-form"]');
    await expect(loginForm).toBeVisible();
  });

  test('未登录时重定向到登录页', async ({ page }) => {
    await page.goto('/chat');
    // 应该被重定向到登录页
    await expect(page).toHaveURL(/login/);
  });

  test('页面无控制台错误', async ({ page }) => {
    const errors: string[] = [];
    page.on('console', (msg) => {
      if (msg.type() === 'error') {
        errors.push(msg.text());
      }
    });
    await page.goto('/');
    // 过滤已知的无害错误
    const criticalErrors = errors.filter(
      (e) => !e.includes('favicon') && !e.includes('WebSocket')
    );
    expect(criticalErrors).toHaveLength(0);
  });
});
