import React, { useState, useEffect, useCallback } from 'react'
import { apiRequest } from '../services/api'
import './NotificationSettingsPage.css'

// ============================================================
// 类型定义
// ============================================================

interface GlobalNotificationSettings {
  enabled: boolean
  sound: boolean
  vibration: boolean
  desktop: boolean
  preview: boolean
  quiet_hours_enabled: boolean
  quiet_hours_start: string
  quiet_hours_end: string
  dnd_enabled: boolean
}

interface ConversationNotificationSettings {
  conversation_id: string
  conversation_name?: string
  conversation_type?: string
  muted: boolean
  sound: boolean
  desktop: boolean
  mention_only: boolean
}

// ============================================================
// 主页面组件
// ============================================================

export default function NotificationSettingsPage() {
  const [globalSettings, setGlobalSettings] = useState<GlobalNotificationSettings | null>(null)
  const [convSettings, setConvSettings] = useState<ConversationNotificationSettings[]>([])
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState<string | null>(null)
  const [activeTab, setActiveTab] = useState<'global' | 'conversations'>('global')

  // 加载全局通知设置
  const loadGlobalSettings = useCallback(async () => {
    try {
      const data = await apiRequest<GlobalNotificationSettings>('/api/im/notifications/settings')
      setGlobalSettings(data)
    } catch (err: any) {
      setError(err.message || '加载通知设置失败')
    }
  }, [])

  // 加载会话通知设置
  const loadConvSettings = useCallback(async () => {
    try {
      // 获取会话列表，每个会话的通知设置
      const convs = await apiRequest<any[]>('/api/im/conversations')
      const settings: ConversationNotificationSettings[] = convs.map((c: any) => ({
        conversation_id: c.id,
        conversation_name: c.name,
        conversation_type: c.type,
        muted: c.is_muted || false,
        sound: true,
        desktop: true,
        mention_only: false,
      }))
      setConvSettings(settings)
    } catch (err: any) {
      console.error('Failed to load conversation settings:', err)
    }
  }, [])

  useEffect(() => {
    const init = async () => {
      setLoading(true)
      await Promise.all([loadGlobalSettings(), loadConvSettings()])
      setLoading(false)
    }
    init()
  }, [loadGlobalSettings, loadConvSettings])

  // 更新全局设置
  const handleGlobalChange = async (key: keyof GlobalNotificationSettings, value: boolean | string) => {
    if (!globalSettings) return
    const newSettings = { ...globalSettings, [key]: value }
    setGlobalSettings(newSettings)

    setSaving(true)
    setError(null)
    setSuccess(null)
    try {
      await apiRequest('/api/im/notifications/settings', {
        method: 'PUT',
        body: JSON.stringify(newSettings),
      })
      setSuccess('设置已保存')
      setTimeout(() => setSuccess(null), 2000)
    } catch (err: any) {
      setError(err.message || '保存失败')
      // 回滚
      setGlobalSettings(globalSettings)
    } finally {
      setSaving(false)
    }
  }

  // 更新会话通知设置
  const handleConvChange = async (convId: string, key: string, value: boolean) => {
    const idx = convSettings.findIndex((s) => s.conversation_id === convId)
    if (idx === -1) return

    const oldSettings = [...convSettings]
    const newSettings = [...convSettings]
    newSettings[idx] = { ...newSettings[idx], [key]: value }
    setConvSettings(newSettings)

    try {
      await apiRequest(`/api/im/conversations/${convId}/notification-settings`, {
        method: 'PUT',
        body: JSON.stringify({ [key]: value }),
      })
    } catch (err: any) {
      setError(err.message || '保存失败')
      setConvSettings(oldSettings)
    }
  }

  // 重置会话通知设置
  const handleResetConv = async (convId: string) => {
    try {
      await apiRequest(`/api/im/conversations/${convId}/notification-settings`, {
        method: 'DELETE',
      })
      // 重新加载
      await loadConvSettings()
      setSuccess('已重置为默认设置')
      setTimeout(() => setSuccess(null), 2000)
    } catch (err: any) {
      setError(err.message || '重置失败')
    }
  }

  // 切换 DND
  const handleToggleDND = async () => {
    if (!globalSettings) return
    try {
      await apiRequest('/api/im/notifications/dnd-status', {
        method: 'GET',
      })
      handleGlobalChange('dnd_enabled', !globalSettings.dnd_enabled)
    } catch (err: any) {
      // 直接切换
      handleGlobalChange('dnd_enabled', !globalSettings.dnd_enabled)
    }
  }

  if (loading) {
    return (
      <div className="notification-settings-page">
        <div className="settings-loading">加载中...</div>
      </div>
    )
  }

  return (
    <div className="notification-settings-page">
      {/* 头部 */}
      <div className="settings-header">
        <h1>🔔 通知设置</h1>
        <p>管理您的消息通知偏好</p>
      </div>

      {/* 状态消息 */}
      {error && (
        <div className="settings-alert settings-alert--error">
          <span>{error}</span>
          <button onClick={() => setError(null)}>✕</button>
        </div>
      )}
      {success && (
        <div className="settings-alert settings-alert--success">
          <span>✅ {success}</span>
        </div>
      )}

      {/* DND 快捷开关 */}
      <div className="dnd-banner">
        <div className="dnd-banner__info">
          <span className="dnd-banner__icon">🌙</span>
          <div>
            <h3>免打扰模式</h3>
            <p>开启后将暂停所有通知</p>
          </div>
        </div>
        <button
          className={`dnd-toggle ${globalSettings?.dnd_enabled ? 'active' : ''}`}
          onClick={handleToggleDND}
        >
          {globalSettings?.dnd_enabled ? '已开启' : '已关闭'}
        </button>
      </div>

      {/* Tab 切换 */}
      <div className="settings-tabs">
        <button
          className={`settings-tab ${activeTab === 'global' ? 'active' : ''}`}
          onClick={() => setActiveTab('global')}
        >
          ⚙️ 全局设置
        </button>
        <button
          className={`settings-tab ${activeTab === 'conversations' ? 'active' : ''}`}
          onClick={() => setActiveTab('conversations')}
        >
          💬 会话设置
        </button>
      </div>

      {/* Tab 内容 */}
      <div className="settings-content">
        {activeTab === 'global' ? (
          <div className="global-settings">
            {/* 基本通知 */}
            <div className="settings-section">
              <h2>基本通知</h2>
              <div className="settings-card">
                <SettingToggle
                  icon="🔔"
                  title="启用通知"
                  description="接收新消息通知"
                  value={globalSettings?.enabled ?? true}
                  onChange={(v) => handleGlobalChange('enabled', v)}
                  disabled={saving}
                />
                <SettingToggle
                  icon="🔊"
                  title="通知声音"
                  description="收到消息时播放声音"
                  value={globalSettings?.sound ?? true}
                  onChange={(v) => handleGlobalChange('sound', v)}
                  disabled={saving}
                />
                <SettingToggle
                  icon="📳"
                  title="震动反馈"
                  description="收到消息时震动（移动端）"
                  value={globalSettings?.vibration ?? true}
                  onChange={(v) => handleGlobalChange('vibration', v)}
                  disabled={saving}
                />
                <SettingToggle
                  icon="🖥️"
                  title="桌面通知"
                  description="在桌面显示通知弹窗"
                  value={globalSettings?.desktop ?? true}
                  onChange={(v) => handleGlobalChange('desktop', v)}
                  disabled={saving}
                />
                <SettingToggle
                  icon="👁️"
                  title="消息预览"
                  description="在通知中显示消息内容"
                  value={globalSettings?.preview ?? true}
                  onChange={(v) => handleGlobalChange('preview', v)}
                  disabled={saving}
                />
              </div>
            </div>

            {/* 免打扰时段 */}
            <div className="settings-section">
              <h2>免打扰时段</h2>
              <div className="settings-card">
                <SettingToggle
                  icon="🌙"
                  title="启用免打扰时段"
                  description="在指定时间段内暂停通知"
                  value={globalSettings?.quiet_hours_enabled ?? false}
                  onChange={(v) => handleGlobalChange('quiet_hours_enabled', v)}
                  disabled={saving}
                />
                {globalSettings?.quiet_hours_enabled && (
                  <div className="quiet-hours-config">
                    <div className="time-range">
                      <div className="time-input">
                        <label>开始时间</label>
                        <input
                          type="time"
                          value={globalSettings?.quiet_hours_start || '22:00'}
                          onChange={(e) => handleGlobalChange('quiet_hours_start', e.target.value)}
                        />
                      </div>
                      <span className="time-separator">至</span>
                      <div className="time-input">
                        <label>结束时间</label>
                        <input
                          type="time"
                          value={globalSettings?.quiet_hours_end || '08:00'}
                          onChange={(e) => handleGlobalChange('quiet_hours_end', e.target.value)}
                        />
                      </div>
                    </div>
                  </div>
                )}
              </div>
            </div>
          </div>
        ) : (
          <div className="conversation-settings">
            {convSettings.length === 0 ? (
              <div className="settings-empty">
                <span>📭</span>
                <p>暂无会话</p>
              </div>
            ) : (
              convSettings.map((setting) => (
                <div key={setting.conversation_id} className="conv-setting-card">
                  <div className="conv-setting__header">
                    <div className="conv-setting__info">
                      <span className="conv-setting__icon">
                        {setting.conversation_type === 'group' ? '👥' : setting.conversation_type === 'ai' ? '🤖' : '💬'}
                      </span>
                      <div>
                        <h3>{setting.conversation_name || '未命名会话'}</h3>
                        <span className="conv-setting__type">
                          {setting.conversation_type === 'group' ? '群聊' : setting.conversation_type === 'ai' ? 'AI 助手' : '私聊'}
                        </span>
                      </div>
                    </div>
                    <button
                      className="btn btn--small btn--secondary"
                      onClick={() => handleResetConv(setting.conversation_id)}
                    >
                      重置
                    </button>
                  </div>
                  <div className="conv-setting__toggles">
                    <SettingToggle
                      icon="🔇"
                      title="免打扰"
                      description="不接收此会话的通知"
                      value={setting.muted}
                      onChange={(v) => handleConvChange(setting.conversation_id, 'muted', v)}
                      compact
                    />
                    <SettingToggle
                      icon="🔊"
                      title="声音"
                      description="收到消息时播放声音"
                      value={setting.sound}
                      onChange={(v) => handleConvChange(setting.conversation_id, 'sound', v)}
                      compact
                    />
                    <SettingToggle
                      icon="🖥️"
                      title="桌面通知"
                      description="显示桌面弹窗"
                      value={setting.desktop}
                      onChange={(v) => handleConvChange(setting.conversation_id, 'desktop', v)}
                      compact
                    />
                    <SettingToggle
                      icon="📢"
                      title="仅@提及"
                      description="仅在被提及时通知"
                      value={setting.mention_only}
                      onChange={(v) => handleConvChange(setting.conversation_id, 'mention_only', v)}
                      compact
                    />
                  </div>
                </div>
              ))
            )}
          </div>
        )}
      </div>
    </div>
  )
}

// ============================================================
// 子组件：设置开关
// ============================================================

function SettingToggle({
  icon,
  title,
  description,
  value,
  onChange,
  disabled = false,
  compact = false,
}: {
  icon: string
  title: string
  description: string
  value: boolean
  onChange: (value: boolean) => void
  disabled?: boolean
  compact?: boolean
}) {
  return (
    <div className={`setting-toggle ${compact ? 'setting-toggle--compact' : ''}`}>
      <div className="setting-toggle__info">
        <span className="setting-toggle__icon">{icon}</span>
        <div>
          <h4>{title}</h4>
          {!compact && <p>{description}</p>}
        </div>
      </div>
      <button
        className={`toggle-switch ${value ? 'active' : ''}`}
        onClick={() => onChange(!value)}
        disabled={disabled}
        aria-label={title}
      >
        <span className="toggle-switch__slider" />
      </button>
    </div>
  )
}
