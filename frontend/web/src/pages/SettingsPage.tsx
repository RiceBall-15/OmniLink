import { useState } from 'react'
import { useAuth } from '../hooks/useAuth'
import { useToast } from '../components/Toast'
import { useI18n } from '../i18n/setup'
import { LanguageSwitcher } from '../components/LanguageSwitcher'
import { ThemeToggle } from '../components/ThemeToggle'
import { useThemeStore } from '../stores/themeStore'
import './SettingsPage.css'

export function SettingsPage() {
  const { user } = useAuth()
  const { showSuccess, showError } = useToast()
  const { t } = useI18n()
  const { theme } = useThemeStore()
  const [activeTab, setActiveTab] = useState('profile')
  const [saving, setSaving] = useState(false)

  // 个人资料状态
  const [profile, setProfile] = useState({
    username: user?.username || '',
    email: user?.email || '',
    bio: '',
    avatar: '',
  })

  // 通知设置
  const [notifications, setNotifications] = useState({
    email: true,
    push: true,
    sound: true,
    desktopNotification: false,
    messagePreview: true,
  })

  // 隐私设置
  const [privacy, setPrivacy] = useState({
    showOnline: true,
    showReadReceipts: true,
    allowSearch: false,
    readBurn: false,
  })

  const handleSaveProfile = async () => {
    setSaving(true)
    try {
      await new Promise((resolve) => setTimeout(resolve, 1000))
      showSuccess(t('common.success'))
    } catch (error) {
      showError(t('common.error'))
    } finally {
      setSaving(false)
    }
  }

  const handleSaveSettings = async () => {
    setSaving(true)
    try {
      await new Promise((resolve) => setTimeout(resolve, 1000))
      showSuccess(t('common.success'))
    } catch (error) {
      showError(t('common.error'))
    } finally {
      setSaving(false)
    }
  }

  const tabs = [
    { id: 'profile', label: t('nav.profile'), icon: '👤' },
    { id: 'appearance', label: t('settings.appearance'), icon: '🎨' },
    { id: 'notifications', label: t('settings.notifications'), icon: '🔔' },
    { id: 'privacy', label: t('settings.privacy'), icon: '🔒' },
    { id: 'about', label: t('settings.about'), icon: 'ℹ️' },
  ]

  return (
    <div className="settings-page">
      <div className="settings-header">
        <h1>⚙️ {t('settings.title')}</h1>
        <button className="back-button" onClick={() => window.history.back()}>
          ← {t('common.back')}
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
          {/* 个人资料 */}
          {activeTab === 'profile' && (
            <div className="settings-section">
              <h2>{t('nav.profile')}</h2>
              <div className="settings-form">
                <div className="form-group">
                  <label>{t('auth.username')}</label>
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
                    <button className="upload-button">{t('files.upload')}</button>
                  </div>
                </div>

                <div className="form-group">
                  <label htmlFor="username">{t('auth.username')}</label>
                  <input
                    id="username"
                    type="text"
                    value={profile.username}
                    onChange={(e) => setProfile({ ...profile, username: e.target.value })}
                  />
                </div>

                <div className="form-group">
                  <label htmlFor="email">{t('auth.email')}</label>
                  <input
                    id="email"
                    type="email"
                    value={profile.email}
                    onChange={(e) => setProfile({ ...profile, email: e.target.value })}
                  />
                </div>

                <div className="form-actions">
                  <button
                    className="save-button"
                    onClick={handleSaveProfile}
                    disabled={saving}
                  >
                    {saving ? t('common.loading') : t('common.save')}
                  </button>
                </div>
              </div>
            </div>
          )}

          {/* 外观设置 */}
          {activeTab === 'appearance' && (
            <div className="settings-section">
              <h2>{t('settings.appearance')}</h2>
              <div className="settings-form">
                <div className="form-group">
                  <label>{t('settings.language')}</label>
                  <LanguageSwitcher />
                </div>

                <div className="form-group">
                  <label>{t('settings.theme')}</label>
                  <ThemeToggle />
                  <p className="form-hint">
                    {t('settings.theme')}: {theme}
                  </p>
                </div>

                <div className="form-group">
                  <label>{t('settings.fontSize')}</label>
                  <select className="form-select">
                    <option value="small">A-</option>
                    <option value="medium" defaultValue="medium">A</option>
                    <option value="large">A+</option>
                  </select>
                </div>

                <div className="form-actions">
                  <button
                    className="save-button"
                    onClick={handleSaveSettings}
                    disabled={saving}
                  >
                    {saving ? t('common.loading') : t('common.save')}
                  </button>
                </div>
              </div>
            </div>
          )}

          {/* 通知设置 */}
          {activeTab === 'notifications' && (
            <div className="settings-section">
              <h2>{t('settings.notifications')}</h2>
              <div className="settings-form">
                <div className="form-group">
                  <div className="toggle-list">
                    {[
                      { key: 'email', label: t('notifications.newMessage'), desc: '接收邮件通知' },
                      { key: 'push', label: t('notifications.pushEnabled'), desc: '接收浏览器推送' },
                      { key: 'sound', label: t('notifications.soundEnabled'), desc: '新消息提示音' },
                      { key: 'desktopNotification', label: t('settings.desktopNotification'), desc: '桌面弹窗通知' },
                      { key: 'messagePreview', label: t('settings.messagePreview'), desc: '通知中显示消息内容' },
                    ].map(item => (
                      <div className="toggle-item" key={item.key}>
                        <div className="toggle-info">
                          <span className="toggle-label">{item.label}</span>
                          <span className="toggle-description">{item.desc}</span>
                        </div>
                        <button
                          className={`toggle-button ${(notifications as any)[item.key] ? 'active' : ''}`}
                          onClick={() => setNotifications({
                            ...notifications,
                            [item.key]: !(notifications as any)[item.key],
                          })}
                        >
                          <span className="toggle-slider"></span>
                        </button>
                      </div>
                    ))}
                  </div>
                </div>

                <div className="form-group">
                  <label>{t('notifications.quietHours')}</label>
                  <div className="time-range">
                    <input type="time" defaultValue="22:00" className="time-input" />
                    <span>—</span>
                    <input type="time" defaultValue="08:00" className="time-input" />
                  </div>
                </div>

                <div className="form-actions">
                  <button
                    className="save-button"
                    onClick={handleSaveSettings}
                    disabled={saving}
                  >
                    {saving ? t('common.loading') : t('common.save')}
                  </button>
                </div>
              </div>
            </div>
          )}

          {/* 隐私安全 */}
          {activeTab === 'privacy' && (
            <div className="settings-section">
              <h2>{t('settings.privacy')}</h2>
              <div className="settings-form">
                <div className="form-group">
                  <div className="toggle-list">
                    {[
                      { key: 'showOnline', label: t('chat.online'), desc: '显示你的在线状态' },
                      { key: 'showReadReceipts', label: t('chat.readReceipt'), desc: '显示消息已读状态' },
                      { key: 'allowSearch', label: '允许搜索', desc: '其他用户可通过邮箱找到你' },
                      { key: 'readBurn', label: t('chat.burnAfterRead'), desc: '阅后即焚默认开启' },
                    ].map(item => (
                      <div className="toggle-item" key={item.key}>
                        <div className="toggle-info">
                          <span className="toggle-label">{item.label}</span>
                          <span className="toggle-description">{item.desc}</span>
                        </div>
                        <button
                          className={`toggle-button ${(privacy as any)[item.key] ? 'active' : ''}`}
                          onClick={() => setPrivacy({
                            ...privacy,
                            [item.key]: !(privacy as any)[item.key],
                          })}
                        >
                          <span className="toggle-slider"></span>
                        </button>
                      </div>
                    ))}
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
                    {saving ? t('common.loading') : t('common.save')}
                  </button>
                </div>
              </div>
            </div>
          )}

          {/* 关于 */}
          {activeTab === 'about' && (
            <div className="settings-section">
              <h2>{t('settings.about')}</h2>
              <div className="settings-form">
                <div className="about-info">
                  <div className="about-logo">
                    <span className="logo-icon">🔗</span>
                    <h3>OmniLink</h3>
                  </div>
                  <p className="about-version">{t('settings.version')}: v2.9.0</p>
                  <p className="about-desc">
                    OmniLink 是一个全功能即时通讯平台，支持 AI 助手、文件管理、群组聊天等功能。
                  </p>
                  <div className="about-links">
                    <a href="/admin/performance">{t('admin.performance')}</a>
                    <a href="/admin/health">{t('admin.healthCheck')}</a>
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
