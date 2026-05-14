/**
 * 通知服务 React Hook
 */

import { useState, useEffect, useCallback } from 'react';
import { notificationService, NotificationSettings } from '../services/notificationService';

export function useNotification() {
  const [settings, setSettings] = useState<NotificationSettings>(notificationService.getSettings());
  const [permission, setPermission] = useState<NotificationPermission>('default');
  const [unreadCount, setUnreadCount] = useState(notificationService.getUnreadCount());

  // 初始化时检查权限
  useEffect(() => {
    const checkPermission = async () => {
      const perm = await notificationService.checkPermission();
      setPermission(perm);
    };
    checkPermission();
  }, []);

  // 监听未读计数变化
  useEffect(() => {
    const interval = setInterval(() => {
      const count = notificationService.getUnreadCount();
      if (count !== unreadCount) {
        setUnreadCount(count);
      }
    }, 1000);

    return () => clearInterval(interval);
  }, [unreadCount]);

  // 请求通知权限
  const requestPermission = useCallback(async () => {
    const perm = await notificationService.requestPermission();
    setPermission(perm);
    return perm;
  }, []);

  // 更新设置
  const updateSettings = useCallback((newSettings: Partial<NotificationSettings>) => {
    notificationService.updateSettings(newSettings);
    setSettings(notificationService.getSettings());
  }, []);

  // 清除未读计数
  const clearUnreadCount = useCallback(() => {
    notificationService.clearUnreadCount();
    setUnreadCount(0);
  }, []);

  // 发送测试通知
  const sendTestNotification = useCallback(async () => {
    await notificationService.sendNotification({
      title: '测试通知',
      body: '这是一条测试通知，用于验证通知功能是否正常工作。',
      tag: 'test-notification',
    });
  }, []);

  return {
    settings,
    permission,
    unreadCount,
    requestPermission,
    updateSettings,
    clearUnreadCount,
    sendTestNotification,
  };
}

export default useNotification;
