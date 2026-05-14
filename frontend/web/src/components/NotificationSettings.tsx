/**
 * 通知设置组件
 */

import React from 'react';
import { useNotification } from '../hooks/useNotification';

export function NotificationSettings() {
  const {
    settings,
    permission,
    requestPermission,
    updateSettings,
    sendTestNotification,
  } = useNotification();

  const handleRequestPermission = async () => {
    const result = await requestPermission();
    if (result === 'granted') {
      alert('通知权限已授予！');
    } else {
      alert('通知权限被拒绝，请在浏览器设置中允许通知。');
    }
  };

  const handleToggle = (key: keyof typeof settings) => {
    updateSettings({ [key]: !settings[key] });
  };

  const handleVolumeChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    updateSettings({ soundVolume: parseFloat(e.target.value) });
  };

  return (
    <div className="notification-settings">
      <h3>通知设置</h3>
      
      {/* 权限状态 */}
      <div className="setting-item">
        <label>通知权限</label>
        <div className="permission-status">
          <span className={`status-badge ${permission === 'granted' ? 'granted' : 'denied'}`}>
            {permission === 'granted' ? '已授权' : permission === 'denied' ? '已拒绝' : '未授权'}
          </span>
          {permission !== 'granted' && (
            <button onClick={handleRequestPermission} className="btn-primary">
              请求权限
            </button>
          )}
        </div>
      </div>

      {/* 启用通知 */}
      <div className="setting-item">
        <label>
          <input
            type="checkbox"
            checked={settings.enabled}
            onChange={() => handleToggle('enabled')}
          />
          启用通知
        </label>
      </div>

      {/* 声音设置 */}
      <div className="setting-item">
        <label>
          <input
            type="checkbox"
            checked={settings.sound}
            onChange={() => handleToggle('sound')}
            disabled={!settings.enabled}
          />
          播放通知声音
        </label>
        {settings.sound && settings.enabled && (
          <div className="volume-control">
            <label>音量：</label>
            <input
              type="range"
              min="0"
              max="1"
              step="0.1"
              value={settings.soundVolume}
              onChange={handleVolumeChange}
            />
            <span>{Math.round(settings.soundVolume * 100)}%</span>
          </div>
        )}
      </div>

      {/* 显示角标 */}
      <div className="setting-item">
        <label>
          <input
            type="checkbox"
            checked={settings.showBadge}
            onChange={() => handleToggle('showBadge')}
            disabled={!settings.enabled}
          />
          显示未读角标
        </label>
      </div>

      {/* 显示消息预览 */}
      <div className="setting-item">
        <label>
          <input
            type="checkbox"
            checked={settings.showMessagePreview}
            onChange={() => handleToggle('showMessagePreview')}
            disabled={!settings.enabled}
          />
          显示消息预览
        </label>
      </div>

      {/* 测试按钮 */}
      <div className="setting-item">
        <button onClick={sendTestNotification} className="btn-secondary">
          发送测试通知
        </button>
      </div>
    </div>
  );
}

export default NotificationSettings;
