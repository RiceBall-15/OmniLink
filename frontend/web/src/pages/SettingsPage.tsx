import React, { useState } from 'react'
import { useAuth } from '../hooks/useAuth'
import { useToast } from '../components/Toast'
import './SettingsPage.css'

export function SettingsPage() {
  const { user, logout } = useAuth()
  const { showSuccess, showError } = useToast()
  const [activeTab, setActiveTab] = useState('profile')
  const [saving, setSaving] = useState(false)

  // 个人资料状态
  const [profile, setProfile] = useState({
    username: user?.username || '',
    email: user?.email || '',
    bio: '',
    avatar: '',
  })

  // 主题设置
  const [theme, setTheme] = useState('light')

  // 通知设置
  const [notifications, setNotifications] = useState({
    email: true,
    push: true,
    sound: true,
  })

  // 隐私设置
  const [privacy, setPrivacy] = useState({
    showOnline: true,
    showReadReceipts: true,
    allowSearch: false,
  })

  const handleSaveProfile = async () => {
    setSaving(true)
    try {
      // 模拟保存
      await new Promise((resolve) => setTimeout(resolve, 1000))
      showSuccess('个人资料保存成功')
    } catch (error) {
      showError('保存失败，请稍后重试')
    } finally {
      setSaving(false)
    }
  }

  const handleSaveSettings = async () => {
    setSaving(true)
    try {
      // 模拟保存
      await new Promise((resolve) => setTimeout(resolve, 1000))
      showSuccess('设置保存成功')
    } catch (error) {
      showError('保存失败，请稍后重试')
    } finally {
      setSaving(false)
    }
  }

  const tabs = [
    { id: 'profile', label: '个人资料', icon: '👤' },
    { id: 'theme', label: '主题外观', icon: '🎨' },
    { id: 'notifications', label: '通知设置', icon: '🔔' },
    { id: 'privacy', label: '隐私安全', icon: '🔒' },
  ]

  return (
    <div className="settings-page">
      <div className="settings-header">
        <h1>⚙️ 设置</h1>
        <button className="back-button" onClick={() => window.history.back()}>
          ← 返回
        </button>
      </div>

      <div className="settings-container">
        {/* 侧边栏标签 */}
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

        {/* 设置内容 */}
        <div className="settings-content">
          {activeTab === 'profile' && (
            <div className="settings-section">
              <h2>个人资料</h2>
              <div className="settings-form">
                <div className="form-group">
                  <label>头像</label>
                  <div className="avatar-upload">
                    <div className="avatar-preview">
                      {profile.avatar ? (
                        <img src={profile.avatar} alt="avatar" />
                      ) : (
                        <div className="avatar-placeholder">
                          {profile.username.charAt(0).toUpperCase()}
                        </div>
                      )}
                    </div>
                    <button className="upload-button">更换头像</button>
                  </div>
                </div>

                <div className="form-group">
                  <label htmlFor="username">用户名</label>
                  <input
                    id="username"
                    type="text"
                    value={profile.username}
                    onChange={(e) => setProfile({ ...profile, username: e.target.value })}
                    placeholder="输入用户名"
                  />
                </div>

                <div className="form-group">
                  <label htmlFor="email">邮箱</label>
                  <input
                    id="email"
                    type="email"
                    value={profile.email}
                    onChange={(e) => setProfile({ ...profile, email: e.target.value })}
                    placeholder="输入邮箱"
                  />
                </div>

                <div className="form-group">
                  <label htmlFor="bio">个人简介</label>
                  <textarea
                    id="bio"
                    value={profile.bio}
                    onChange={(e) => setProfile({ ...profile, bio: e.target.value })}
                    placeholder="介绍一下自己..."
                    rows={4}
                  />
                </div>

                <div className="form-actions">
                  <button
                    className="save-button"
                    onClick={handleSaveProfile}
                    disabled={saving}
                  >
                    {saving ? '保存中...' : '保存更改'}
                  </button>
                </div>
              </div>
            </div>
          )}

          {activeTab === 'theme' && (
            <div className="settings-section">
              <h2>主题外观</h2>
              <div className="settings-form">
                <div className="form-group">
                  <label>主题模式</label>
                  <div className="theme-selector">
                    <button
                      className={`theme-option ${theme === 'light' ? 'active' : ''}`}
                      onClick={() => setTheme('light')}
                    >
                      <span className="theme-icon">☀️</span>
                      <span>浅色</span>
                    </button>
                    <button
                      className={`theme-option ${theme === 'dark' ? 'active' : ''}`}
                      onClick={() => setTheme('dark')}
                    >
                      <span className="theme-icon">🌙</span>
                      <span>深色</span>
                    </button>
                    <button
                      className={`theme-option ${theme === 'auto' ? 'active' : ''}`}
                      onClick={() => setTheme('auto')}
                    >
                      <span className="theme-icon">🌓</span>
                      <span>自动</span>
                    </button>
                  </div>
                </div>

                <div className="form-group">
                  <label>字体大小</label>
                  <select className="form-select">
                    <option value="small">小</option>
                    <option value="medium" selected>中</option>
                    <option value="large">大</option>
                  </select>
                </div>

                <div className="form-actions">
                  <button
                    className="save-button"
                    onClick={handleSaveSettings}
                    disabled={saving}
                  >
                    {saving ? '保存中...' : '保存更改'}
                  </button>
                </div>
              </div>
            </div>
          )}

          {activeTab === 'notifications' && (
            <div className="settings-section">
              <h2>通知设置</h2>
              <div className="settings-form">
                <div className="form-group">
                  <label>通知类型</label>
                  <div className="toggle-list">
                    <div className="toggle-item">
                      <div className="toggle-info">
                        <span className="toggle-label">邮件通知</span>
                        <span className="toggle-description">接收重要消息的邮件提醒</span>
                      </div>
                      <button
                        className={`toggle-button ${notifications.email ? 'active' : ''}`}
                        onClick={() => setNotifications({ ...notifications, email: !notifications.email })}
                      >
                        <span className="toggle-slider"></span>
                      </button>
                    </div>

                    <div className="toggle-item">
                      <div className="toggle-info">
                        <span className="toggle-label">推送通知</span>
                        <span className="toggle-description">接收浏览器推送通知</span>
                      </div>
                      <button
                        className={`toggle-button ${notifications.push ? 'active' : ''}`}
                        onClick={() => setNotifications({ ...notifications, push: !notifications.push })}
                      >
                        <span className="toggle-slider"></span>
                      </button>
                    </div>

                    <div className="toggle-item">
                      <div className="toggle-info">
                        <span className="toggle-label">声音提示</span>
                        <span className="toggle-description">新消息时播放提示音</span>
                      </div>
                      <button
                        className={`toggle-button ${notifications.sound ? 'active' : ''}`}
                        onClick={() => setNotifications({ ...notifications, sound: !notifications.sound })}
                      >
                        <span className="toggle-slider"></span>
                      </button>
                    </div>
                  </div>
                </div>

                <div className="form-actions">
                  <button
                    className="save-button"
                    onClick={handleSaveSettings}
                    disabled={saving}
                  >
                    {saving ? '保存中...' : '保存更改'}
                  </button>
                </div>
              </div>
            </div>
          )}

          {activeTab === 'privacy' && (
            <div className="settings-section">
              <h2>隐私安全</h2>
              <div className="settings-form">
                <div className="form-group">
                  <label>在线状态</label>
                  <div className="toggle-list">
                    <div className="toggle-item">
                      <div className="toggle-info">
                        <span className="toggle-label">显示在线状态</span>
                        <span className="toggle-description">其他用户可以看到你是否在线</span>
                      </div>
                      <button
                        className={`toggle-button ${privacy.showOnline ? 'active' : ''}`}
                        onClick={() => setPrivacy({ ...privacy, showOnline: !privacy.showOnline })}
                      >
                        <span className="toggle-slider"></span>
                      </button>
                    </div>

                    <div className="toggle-item">
                      <div className="toggle-info">
                        <span className="toggle-label">已读回执</span>
                        <span className="toggle-description">显示消息已读状态</span>
                      </div>
                      <button
                        className={`toggle-button ${privacy.showReadReceipts ? 'active' : ''}`}
                        onClick={() => setPrivacy({ ...privacy, showReadReceipts: !privacy.showReadReceipts })}
                      >
                        <span className="toggle-slider"></span>
                      </button>
                    </div>

                    <div className="toggle-item">
                      <div className="toggle-info">
                        <span className="toggle-label">允许搜索</span>
                        <span className="toggle-description">其他用户可以通过邮箱找到你</span>
                      </div>
                      <button
                        className={`toggle-button ${privacy.allowSearch ? 'active' : ''}`}
                        onClick={() => setPrivacy({ ...privacy, allowSearch: !privacy.allowSearch })}
                      >
                        <span className="toggle-slider"></span>
                      </button>
                    </div>
                  </div>
                </div>

                <div className="form-group">
                  <label>账号安全</label>
                  <div className="security-actions">
                    <button className="security-button">修改密码</button>
                    <button className="security-button">启用两步验证</button>
                    <button className="security-button danger">删除账号</button>
                  </div>
                </div>

                <div className="form-actions">
                  <button
                    className="save-button"
                    onClick={handleSaveSettings}
                    disabled={saving}
                  >
                    {saving ? '保存中...' : '保存更改'}
                  </button>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
