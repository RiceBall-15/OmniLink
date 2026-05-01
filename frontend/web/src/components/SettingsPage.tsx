import React, { useState } from 'react'
import { useAuth } from '../hooks/useAuth'
import { useToast } from './Toast'
import { Modal } from './Modal'
import './SettingsPage.css'

export function SettingsPage({ isOpen, onClose }: { isOpen: boolean; onClose: () => void }) {
  const { user, logout } = useAuth()
  const { showSuccess, showError } = useToast()
  const [activeTab, setActiveTab] = useState('profile')
  const [loading, setLoading] = useState(false)

  const [profileForm, setProfileForm] = useState({
    username: user?.username || '',
    email: user?.email || '',
    bio: '',
  })

  const [settings, setSettings] = useState({
    theme: 'light' as 'light' | 'dark' | 'auto',
    notifications: true,
    soundEnabled: true,
    language: 'zh-CN',
    fontSize: 'medium' as 'small' | 'medium' | 'large',
  })

  const tabs = [
    { id: 'profile', label: '个人资料', icon: '👤' },
    { id: 'appearance', label: '外观设置', icon: '🎨' },
    { id: 'notifications', label: '通知设置', icon: '🔔' },
    { id: 'security', label: '安全设置', icon: '🔒' },
  ]

  const handleProfileUpdate = async () => {
    setLoading(true)
    try {
      // 模拟API调用
      await new Promise((resolve) => setTimeout(resolve, 1000))
      showSuccess('个人资料更新成功')
    } catch (error) {
      showError('更新失败，请稍后重试')
    } finally {
      setLoading(false)
    }
  }

  const handleSettingsUpdate = async () => {
    setLoading(true)
    try {
      // 模拟API调用
      await new Promise((resolve) => setTimeout(resolve, 500))
      localStorage.setItem('settings', JSON.stringify(settings))
      showSuccess('设置保存成功')
    } catch (error) {
      showError('保存失败，请稍后重试')
    } finally {
      setLoading(false)
    }
  }

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="⚙️ 设置" size="lg">
      <div className="settings-container">
        <div className="settings-sidebar">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              className={`settings-tab ${activeTab === tab.id ? 'active' : ''}`}
              onClick={() => setActiveTab(tab.id)}
            >
              <span className="tab-icon">{tab.icon}</span>
              <span className="tab-label">{tab.label}</span>
            </button>
          ))}
        </div>

        <div className="settings-content">
          {activeTab === 'profile' && (
            <div className="settings-section">
              <h3>个人资料</h3>
              <div className="profile-header">
                <div className="profile-avatar-large">
                  {user?.username?.charAt(0).toUpperCase() || 'U'}
                </div>
                <button className="btn btn-secondary btn-sm">更换头像</button>
              </div>

              <div className="form-group">
                <label>用户名</label>
                <input
                  type="text"
                  value={profileForm.username}
                  onChange={(e) => setProfileForm({ ...profileForm, username: e.target.value })}
                />
              </div>

              <div className="form-group">
                <label>邮箱地址</label>
                <input
                  type="email"
                  value={profileForm.email}
                  onChange={(e) => setProfileForm({ ...profileForm, email: e.target.value })}
                />
              </div>

              <div className="form-group">
                <label>个人简介</label>
                <textarea
                  rows={4}
                  placeholder="介绍一下你自己..."
                  value={profileForm.bio}
                  onChange={(e) => setProfileForm({ ...profileForm, bio: e.target.value })}
                />
              </div>

              <button className="btn btn-primary" onClick={handleProfileUpdate} disabled={loading}>
                {loading ? '保存中...' : '保存更改'}
              </button>
            </div>
          )}

          {activeTab === 'appearance' && (
            <div className="settings-section">
              <h3>外观设置</h3>

              <div className="setting-item">
                <div className="setting-info">
                  <div className="setting-label">主题模式</div>
                  <div className="setting-description">选择你喜欢的界面主题</div>
                </div>
                <div className="setting-control">
                  <select
                    value={settings.theme}
                    onChange={(e) => setSettings({ ...settings, theme: e.target.value as any })}
                  >
                    <option value="light">浅色模式</option>
                    <option value="dark">深色模式</option>
                    <option value="auto">跟随系统</option>
                  </select>
                </div>
              </div>

              <div className="setting-item">
                <div className="setting-info">
                  <div className="setting-label">字体大小</div>
                  <div className="setting-description">调整界面文字大小</div>
                </div>
                <div className="setting-control">
                  <select
                    value={settings.fontSize}
                    onChange={(e) => setSettings({ ...settings, fontSize: e.target.value as any })}
                  >
                    <option value="small">小</option>
                    <option value="medium">中</option>
                    <option value="large">大</option>
                  </select>
                </div>
              </div>

              <div className="setting-item">
                <div className="setting-info">
                  <div className="setting-label">语言</div>
                  <div className="setting-description">选择界面语言</div>
                </div>
                <div className="setting-control">
                  <select
                    value={settings.language}
                    onChange={(e) => setSettings({ ...settings, language: e.target.value })}
                  >
                    <option value="zh-CN">简体中文</option>
                    <option value="en-US">English</option>
                    <option value="ja-JP">日本語</option>
                  </select>
                </div>
              </div>

              <button className="btn btn-primary" onClick={handleSettingsUpdate} disabled={loading}>
                {loading ? '保存中...' : '保存设置'}
              </button>
            </div>
          )}

          {activeTab === 'notifications' && (
            <div className="settings-section">
              <h3>通知设置</h3>

              <div className="setting-item">
                <div className="setting-info">
                  <div className="setting-label">启用通知</div>
                  <div className="setting-description">接收新消息通知</div>
                </div>
                <div className="setting-control">
                  <label className="switch">
                    <input
                      type="checkbox"
                      checked={settings.notifications}
                      onChange={(e) => setSettings({ ...settings, notifications: e.target.checked })}
                    />
                    <span className="switch-slider"></span>
                  </label>
                </div>
              </div>

              <div className="setting-item">
                <div className="setting-info">
                  <div className="setting-label">声音提醒</div>
                  <div className="setting-description">收到消息时播放声音</div>
                </div>
                <div className="setting-control">
                  <label className="switch">
                    <input
                      type="checkbox"
                      checked={settings.soundEnabled}
                      onChange={(e) => setSettings({ ...settings, soundEnabled: e.target.checked })}
                    />
                    <span className="switch-slider"></span>
                  </label>
                </div>
              </div>

              <button className="btn btn-primary" onClick={handleSettingsUpdate} disabled={loading}>
                {loading ? '保存中...' : '保存设置'}
              </button>
            </div>
          )}

          {activeTab === 'security' && (
            <div className="settings-section">
              <h3>安全设置</h3>

              <div className="security-item">
                <h4>修改密码</h4>
                <div className="form-group">
                  <label>当前密码</label>
                  <input type="password" placeholder="输入当前密码" />
                </div>
                <div className="form-group">
                  <label>新密码</label>
                  <input type="password" placeholder="输入新密码" />
                </div>
                <div className="form-group">
                  <label>确认新密码</label>
                  <input type="password" placeholder="再次输入新密码" />
                </div>
                <button className="btn btn-primary">更新密码</button>
              </div>

              <div className="divider"></div>

              <div className="security-item">
                <h4>两步验证</h4>
                <p className="security-description">
                  启用两步验证可以提高账户安全性
                </p>
                <button className="btn btn-secondary">设置两步验证</button>
              </div>

              <div className="divider"></div>

              <div className="security-item danger">
                <h4>危险区域</h4>
                <p className="security-description">
                  这些操作不可逆，请谨慎操作
                </p>
                <button className="btn btn-danger" onClick={logout}>
                  退出登录
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </Modal>
  )
}
