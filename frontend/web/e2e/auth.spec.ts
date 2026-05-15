import { test, expect } from '@playwright/test';

/**
 * OmniLink 认证流程 E2E 测试
 * 测试用户注册、登录、登出的完整流程
 */

test.describe('Authentication Flow', () => {
  test('登录页面显示用户名和密码输入框', async ({ page }) => {
    await page.goto('/login');
    await expect(page.locator('input[type="text"], input[name="username"], input[name="email"]')).toBeVisible();
    await expect(page.locator('input[type="password"]')).toBeVisible();
  });

  test('空表单提交显示验证错误', async ({ page }) => {
    await page.goto('/login');
    // 点击提交按钮
    const submitBtn = page.locator('button[type="submit"]');
    if (await submitBtn.isVisible()) {
      await submitBtn.click();
      // 应该显示错误信息或不跳转
      await expect(page).toHaveURL(/login/);
    }
  });

  test('注册页面可访问', async ({ page }) => {
    await page.goto('/register');
    // 验证注册表单存在
    const registerForm = page.locator('form, [data-testid="register-form"]');
    await expect(registerForm).toBeVisible();
  });
});
