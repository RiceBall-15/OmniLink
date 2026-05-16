/**
 * 通知服务 React Hook
 */

import { useState, useEffect, useCallback } from 'react';
import { notificationService, NotificationSettings, NotificationItem } from '../services/notificationService';

export function useNotification() {
  const [settings, setSettings] = useState<NotificationSettings>(notificationService.getSettings());
  const [permission, setPermission] = useState<NotificationPermission>('default');
  const [unreadCount, setUnreadCount] = useState(notificationService.getUnreadCount());
  const [notifications, setNotifications] = useState<NotificationItem[]>(notificationService.getNotifications());

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

  // 添加通知
  const addNotification = useCallback((notification: Omit<NotificationItem, 'id' | 'timestamp' | 'read'>) => {
    const item = notificationService.addNotification(notification);
    setNotifications(notificationService.getNotifications());
    setUnreadCount(notificationService.getUnreadCount());
    return item;
  }, []);

  // 标记通知为已读
  const markAsRead = useCallback((notificationId: string) => {
    notificationService.markAsRead(notificationId);
    setNotifications(notificationService.getNotifications());
    setUnreadCount(notificationService.getUnreadCount());
  }, []);

  // 全部标为已读
  const markAllAsRead = useCallback(() => {
    notificationService.markAllAsRead();
    setNotifications(notificationService.getNotifications());
    setUnreadCount(0);
  }, []);

  // 删除通知
  const clearNotification = useCallback((notificationId: string) => {
    notificationService.removeNotification(notificationId);
    setNotifications(notificationService.getNotifications());
    setUnreadCount(notificationService.getUnreadCount());
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
    notifications,
    requestPermission,
    updateSettings,
    clearUnreadCount,
    sendTestNotification,
    addNotification,
    markAsRead,
    markAllAsRead,
    clearNotification,
  };
}

export default useNotification;
