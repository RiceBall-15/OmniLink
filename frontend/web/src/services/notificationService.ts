/**
 * 浏览器通知服务
 * 处理消息提醒、通知权限、声音设置
 */

export interface NotificationItem {
  id: string;
  type: 'message' | 'mention' | 'system' | 'friend_request' | 'group_invite';
  title: string;
  message: string;
  timestamp: number;
  read: boolean;
  data?: any;
}

export interface NotificationOptions {
  title: string;
  body: string;
  icon?: string;
  badge?: string;
  tag?: string;
  data?: any;
  silent?: boolean;
  onClick?: () => void;
}

export interface NotificationSettings {
  enabled: boolean;
  sound: boolean;
  soundVolume: number;
  showBadge: boolean;
  showMessagePreview: boolean;
}

class NotificationService {
  private permission: NotificationPermission = 'default';
  private settings: NotificationSettings = {
    enabled: true,
    sound: true,
    soundVolume: 0.5,
    showBadge: true,
    showMessagePreview: true,
  };
  private unreadCount = 0;
  private audio: HTMLAudioElement | null = null;
  private notifications: NotificationItem[] = [];
  private maxNotifications = 50;

  constructor() {
    this.loadSettings();
    this.initAudio();
    this.loadNotifications();
  }

  /**
   * 初始化音频
   */
  private initAudio() {
    try {
      this.audio = new Audio('/sounds/notification.mp3');
      this.audio.volume = this.settings.soundVolume;
    } catch (error) {
      console.warn('Failed to initialize notification audio:', error);
    }
  }

  /**
   * 加载设置
   */
  private loadSettings() {
    try {
      const saved = localStorage.getItem('notification_settings');
      if (saved) {
        this.settings = { ...this.settings, ...JSON.parse(saved) };
      }
    } catch (error) {
      console.warn('Failed to load notification settings:', error);
    }
  }

  /**
   * 保存设置
   */
  /**
   * 加载通知列表
   */
  private loadNotifications() {
    try {
      const saved = localStorage.getItem('notification_list');
      if (saved) {
        this.notifications = JSON.parse(saved);
        this.unreadCount = this.notifications.filter(n => !n.read).length;
      }
    } catch (error) {
      console.warn('Failed to load notifications:', error);
    }
  }

  /**
   * 保存通知列表
   */
  private saveNotifications() {
    try {
      localStorage.setItem('notification_list', JSON.stringify(this.notifications));
    } catch (error) {
      console.warn('Failed to save notifications:', error);
    }
  }

  /**
   * 添加通知到列表
   */
  addNotification(notification: Omit<NotificationItem, 'id' | 'timestamp' | 'read'>): NotificationItem {
    const item: NotificationItem = {
      ...notification,
      id: `notification_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      timestamp: Date.now(),
      read: false,
    };

    this.notifications.unshift(item);

    // 限制最大数量
    if (this.notifications.length > this.maxNotifications) {
      this.notifications = this.notifications.slice(0, this.maxNotifications);
    }

    this.unreadCount = this.notifications.filter(n => !n.read).length;
    this.updateBadge();
    this.saveNotifications();

    return item;
  }

  /**
   * 标记通知为已读
   */
  markAsRead(notificationId: string): void {
    const notification = this.notifications.find(n => n.id === notificationId);
    if (notification) {
      notification.read = true;
      this.unreadCount = this.notifications.filter(n => !n.read).length;
      this.updateBadge();
      this.saveNotifications();
    }
  }

  /**
   * 全部标为已读
   */
  markAllAsRead(): void {
    this.notifications.forEach(n => {
      n.read = true;
    });
    this.unreadCount = 0;
    this.updateBadge();
    this.saveNotifications();
  }

  /**
   * 删除通知
   */
  removeNotification(notificationId: string): void {
    this.notifications = this.notifications.filter(n => n.id !== notificationId);
    this.unreadCount = this.notifications.filter(n => !n.read).length;
    this.updateBadge();
    this.saveNotifications();
  }

  /**
   * 获取通知列表
   */
  getNotifications(): NotificationItem[] {
    return [...this.notifications];
  }

  private saveSettings() {
    try {
      localStorage.setItem('notification_settings', JSON.stringify(this.settings));
    } catch (error) {
      console.warn('Failed to save notification settings:', error);
    }
  }

  /**
   * 请求通知权限
   */
  async requestPermission(): Promise<NotificationPermission> {
    if (!('Notification' in window)) {
      console.warn('Browser does not support notifications');
      return 'denied';
    }

    if (this.permission === 'granted') {
      return 'granted';
    }

    try {
      this.permission = await Notification.requestPermission();
      return this.permission;
    } catch (error) {
      console.error('Failed to request notification permission:', error);
      return 'denied';
    }
  }

  /**
   * 检查通知权限
   */
  async checkPermission(): Promise<NotificationPermission> {
    if (!('Notification' in window)) {
      return 'denied';
    }

    this.permission = Notification.permission;
    return this.permission;
  }

  /**
   * 发送通知
   */
  async sendNotification(options: NotificationOptions): Promise<void> {
    // 检查是否启用通知
    if (!this.settings.enabled) {
      return;
    }

    // 检查权限
    if (this.permission !== 'granted') {
      const permission = await this.requestPermission();
      if (permission !== 'granted') {
        return;
      }
    }

    // 检查页面是否在前台
    if (document.visibilityState === 'visible') {
      // 页面在前台，可以选择不发送通知
      return;
    }

    try {
      const notification = new Notification(options.title, {
        body: this.settings.showMessagePreview ? options.body : '新消息',
        icon: options.icon || '/icons/notification-icon.png',
        badge: options.badge || '/icons/badge-icon.png',
        tag: options.tag || 'omnilink-message',
        data: options.data,
        silent: !this.settings.sound || options.silent,
      });

      // 点击事件
      notification.onclick = () => {
        window.focus();
        notification.close();
        if (options.onClick) {
          options.onClick();
        }
      };

      // 自动关闭
      setTimeout(() => {
        notification.close();
      }, 5000);

      // 更新未读计数
      this.incrementUnreadCount();

      // 播放声音
      if (this.settings.sound && !options.silent) {
        this.playSound();
      }
    } catch (error) {
      console.error('Failed to send notification:', error);
    }
  }

  /**
   * 播放通知声音
   */
  playSound() {
    if (this.audio && this.settings.sound) {
      this.audio.currentTime = 0;
      this.audio.play().catch(error => {
        console.warn('Failed to play notification sound:', error);
      });
    }
  }

  /**
   * 更新未读计数
   */
  incrementUnreadCount() {
    this.unreadCount++;
    this.updateBadge();
  }

  /**
   * 清除未读计数
   */
  clearUnreadCount() {
    this.unreadCount = 0;
    this.updateBadge();
  }

  /**
   * 更新角标
   */
  private updateBadge() {
    if (!this.settings.showBadge) {
      return;
    }

    // 更新页面标题
    if (this.unreadCount > 0) {
      document.title = `(${this.unreadCount}) OmniLink`;
    } else {
      document.title = 'OmniLink';
    }

    // 尝试使用 Navigator.setAppBadge (如果支持)
    if ('setAppBadge' in navigator) {
      try {
        if (this.unreadCount > 0) {
          (navigator as any).setAppBadge(this.unreadCount);
        } else {
          (navigator as any).clearAppBadge();
        }
      } catch (error) {
        // 忽略不支持的情况
      }
    }
  }

  /**
   * 获取设置
   */
  getSettings(): NotificationSettings {
    return { ...this.settings };
  }

  /**
   * 更新设置
   */
  updateSettings(newSettings: Partial<NotificationSettings>) {
    this.settings = { ...this.settings, ...newSettings };
    this.saveSettings();

    // 更新音频音量
    if (this.audio && newSettings.soundVolume !== undefined) {
      this.audio.volume = newSettings.soundVolume;
    }
  }

  /**
   * 获取未读计数
   */
  getUnreadCount(): number {
    return this.unreadCount;
  }

  /**
   * 设置未读计数
   */
  setUnreadCount(count: number) {
    this.unreadCount = count;
    this.updateBadge();
  }
}

// 导出单例
export const notificationService = new NotificationService();
export default notificationService;
