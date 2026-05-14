import React, { useState, useEffect, useRef, useCallback } from 'react'
import { apiRequest } from '../services/api'
import './UserProfilePage.css'

// ============================================================
// 类型定义
// ============================================================

interface UserProfile {
  id: string
  username: string
  email: string
  avatar?: string
  nickname?: string
  signature?: string
  status_message?: string
  online_status: 'offline' | 'online' | 'away' | 'busy'
  created_at: string
  updated_at: string
}

interface Contact {
  id: string
  user_id: string
  nickname?: string
  remark?: string
  avatar?: string
  username?: string
  online_status?: string
  created_at: string
}

interface QuickReply {
  id: string
  shortcut: string
  content: string
  created_at: string
}

// ============================================================
// 主页面组件
// ============================================================

export default function UserProfilePage() {
  const [profile, setProfile] = useState<UserProfile | null>(null)
  const [contacts, setContacts] = useState<Contact[]>([])
  const [quickReplies, setQuickReplies] = useState<QuickReply[]>([])
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState<string | null>(null)
  const [activeTab, setActiveTab] = useState<'profile' | 'contacts' | 'quick-replies'>('profile')
  const [editingField, setEditingField] = useState<string | null>(null)
  const [editValue, setEditValue] = useState('')
  const fileInputRef = useRef<HTMLInputElement>(null)

  // 加载用户资料
  const loadProfile = useCallback(async () => {
    try {
      const data = await apiRequest<UserProfile>('/api/user/me')
      setProfile(data)
    } catch (err: any) {
      setError(err.message || '加载用户资料失败')
    }
  }, [])

  // 加载联系人
  const loadContacts = useCallback(async () => {
    try {
      const data = await apiRequest<Contact[]>('/api/users/contacts')
      setContacts(data)
    } catch (err: any) {
      console.error('Failed to load contacts:', err)
    }
  }, [])

  // 加载快捷回复
  const loadQuickReplies = useCallback(async () => {
    try {
      const data = await apiRequest<QuickReply[]>('/api/users/quick-replies')
      setQuickReplies(data)
    } catch (err: any) {
      console.error('Failed to load quick replies:', err)
    }
  }, [])

  useEffect(() => {
    const init = async () => {
      setLoading(true)
      await Promise.all([loadProfile(), loadContacts(), loadQuickReplies()])
      setLoading(false)
    }
    init()
  }, [loadProfile, loadContacts, loadQuickReplies])

  // 更新头像
  const handleAvatarUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file) return

    // 验证文件类型和大小
    if (!file.type.startsWith('image/')) {
      setError('请选择图片文件')
      return
    }
    if (file.size > 5 * 1024 * 1024) {
      setError('图片大小不能超过 5MB')
      return
    }

    setSaving(true)
    setError(null)
    try {
      // 上传头像
      const formData = new FormData()
      formData.append('file', file)
      const uploadResult = await apiRequest<{ url: string }>('/api/files/upload', {
        method: 'POST',
        body: formData,
        headers: {}, // 让浏览器自动设置 Content-Type
      })

      // 更新用户头像
      await apiRequest('/api/user/profile', {
        method: 'PUT',
        body: JSON.stringify({ avatar: uploadResult.url }),
      })

      setProfile((prev) => prev ? { ...prev, avatar: uploadResult.url } : null)
      setSuccess('头像已更新')
      setTimeout(() => setSuccess(null), 2000)
    } catch (err: any) {
      setError(err.message || '上传头像失败')
    } finally {
      setSaving(false)
    }
  }

  // 开始编辑字段
  const startEdit = (field: string, value: string) => {
    setEditingField(field)
    setEditValue(value || '')
  }

  // 保存字段
  const saveField = async (field: string) => {
    if (!profile) return

    setSaving(true)
    setError(null)
    try {
      await apiRequest('/api/user/profile', {
        method: 'PUT',
        body: JSON.stringify({ [field]: editValue }),
      })

      setProfile((prev) => prev ? { ...prev, [field]: editValue } : null)
      setEditingField(null)
      setSuccess('已保存')
      setTimeout(() => setSuccess(null), 2000)
    } catch (err: any) {
      setError(err.message || '保存失败')
    } finally {
      setSaving(false)
    }
  }

  // 取消编辑
  const cancelEdit = () => {
    setEditingField(null)
    setEditValue('')
  }

  // 更新在线状态
  const handleStatusChange = async (status: string) => {
    try {
      await apiRequest('/api/users/status', {
        method: 'PUT',
        body: JSON.stringify({ status }),
      })
      setProfile((prev) => prev ? { ...prev, online_status: status as any } : null)
      setSuccess('状态已更新')
      setTimeout(() => setSuccess(null), 2000)
    } catch (err: any) {
      setError(err.message || '更新状态失败')
    }
  }

  // 删除联系人
  const handleDeleteContact = async (contactId: string) => {
    if (!confirm('确定删除该联系人吗？')) return
    try {
      await apiRequest(`/api/users/contacts/${contactId}`, {
        method: 'DELETE',
      })
      setContacts((prev) => prev.filter((c) => c.id !== contactId))
    } catch (err: any) {
      setError(err.message || '删除联系人失败')
    }
  }

  // 创建快捷回复
  const handleCreateQuickReply = async () => {
    const shortcut = prompt('输入快捷关键词:')
    const content = prompt('输入回复内容:')
    if (!shortcut || !content) return

    try {
      const newReply = await apiRequest<QuickReply>('/api/users/quick-replies', {
        method: 'POST',
        body: JSON.stringify({ shortcut, content }),
      })
      setQuickReplies((prev) => [...prev, newReply])
      setSuccess('快捷回复已创建')
      setTimeout(() => setSuccess(null), 2000)
    } catch (err: any) {
      setError(err.message || '创建快捷回复失败')
    }
  }

  // 删除快捷回复
  const handleDeleteQuickReply = async (id: string) => {
    if (!confirm('确定删除该快捷回复吗？')) return
    try {
      await apiRequest(`/api/users/quick-replies/${id}`, {
        method: 'DELETE',
      })
      setQuickReplies((prev) => prev.filter((r) => r.id !== id))
    } catch (err: any) {
      setError(err.message || '删除快捷回复失败')
    }
  }

  if (loading) {
    return (
      <div className="user-profile-page">
        <div className="profile-loading">加载中...</div>
      </div>
    )
  }

  return (
    <div className="user-profile-page">
      {/* 头部 */}
      <div className="profile-header">
        <div className="profile-header__cover">
          <div className="profile-header__avatar-wrapper">
            <div className="profile-header__avatar" onClick={() => fileInputRef.current?.click()}>
              {profile?.avatar ? (
                <img src={profile.avatar} alt="" />
              ) : (
                <span>👤</span>
              )}
              <div className="avatar-overlay">
                <span>📷</span>
                <span>更换头像</span>
              </div>
            </div>
            <input
              ref={fileInputRef}
              type="file"
              accept="image/*"
              onChange={handleAvatarUpload}
              style={{ display: 'none' }}
            />
          </div>
          <div className="profile-header__info">
            <h1>{profile?.nickname || profile?.username || '用户'}</h1>
            <p className="profile-header__signature">
              {profile?.signature || '这个人很懒，什么都没留下'}
            </p>
            <div className="profile-header__status">
              <span className={`status-dot status-dot--${profile?.online_status}`} />
              <select
                value={profile?.online_status || 'online'}
                onChange={(e) => handleStatusChange(e.target.value)}
                className="status-select"
              >
                <option value="online">在线</option>
                <option value="away">离开</option>
                <option value="busy">忙碌</option>
                <option value="offline">离线</option>
              </select>
            </div>
          </div>
        </div>
      </div>

      {/* 状态消息 */}
      {error && (
        <div className="profile-alert profile-alert--error">
          <span>{error}</span>
          <button onClick={() => setError(null)}>✕</button>
        </div>
      )}
      {success && (
        <div className="profile-alert profile-alert--success">
          <span>✅ {success}</span>
        </div>
      )}

      {/* Tab 切换 */}
      <div className="profile-tabs">
        <button
          className={`profile-tab ${activeTab === 'profile' ? 'active' : ''}`}
          onClick={() => setActiveTab('profile')}
        >
          👤 个人资料
        </button>
        <button
          className={`profile-tab ${activeTab === 'contacts' ? 'active' : ''}`}
          onClick={() => setActiveTab('contacts')}
        >
          📇 联系人 ({contacts.length})
        </button>
        <button
          className={`profile-tab ${activeTab === 'quick-replies' ? 'active' : ''}`}
          onClick={() => setActiveTab('quick-replies')}
        >
          ⚡ 快捷回复
        </button>
      </div>

      {/* Tab 内容 */}
      <div className="profile-content">
        {activeTab === 'profile' ? (
          <div className="profile-edit">
            <div className="profile-card">
              <h2>基本信息</h2>
              <ProfileField
                label="用户名"
                value={profile?.username}
                editable={false}
              />
              <ProfileField
                label="邮箱"
                value={profile?.email}
                editable={false}
              />
              <ProfileField
                label="昵称"
                value={profile?.nickname}
                placeholder="设置昵称"
                editing={editingField === 'nickname'}
                editValue={editValue}
                onStartEdit={() => startEdit('nickname', profile?.nickname || '')}
                onSave={() => saveField('nickname')}
                onCancel={cancelEdit}
                onChange={setEditValue}
                disabled={saving}
              />
              <ProfileField
                label="个性签名"
                value={profile?.signature}
                placeholder="设置个性签名"
                editing={editingField === 'signature'}
                editValue={editValue}
                onStartEdit={() => startEdit('signature', profile?.signature || '')}
                onSave={() => saveField('signature')}
                onCancel={cancelEdit}
                onChange={setEditValue}
                disabled={saving}
              />
              <ProfileField
                label="状态消息"
                value={profile?.status_message}
                placeholder="设置状态消息"
                editing={editingField === 'status_message'}
                editValue={editValue}
                onStartEdit={() => startEdit('status_message', profile?.status_message || '')}
                onSave={() => saveField('status_message')}
                onCancel={cancelEdit}
                onChange={setEditValue}
                disabled={saving}
              />
            </div>

            <div className="profile-card">
              <h2>账号信息</h2>
              <ProfileField
                label="用户 ID"
                value={profile?.id}
                editable={false}
                copyable
              />
              <ProfileField
                label="注册时间"
                value={profile?.created_at ? new Date(profile.created_at).toLocaleString() : ''}
                editable={false}
              />
              <ProfileField
                label="最后更新"
                value={profile?.updated_at ? new Date(profile.updated_at).toLocaleString() : ''}
                editable={false}
              />
            </div>
          </div>
        ) : activeTab === 'contacts' ? (
          <div className="contacts-list">
            {contacts.length === 0 ? (
              <div className="contacts-empty">
                <span>📇</span>
                <p>暂无联系人</p>
              </div>
            ) : (
              contacts.map((contact) => (
                <div key={contact.id} className="contact-card">
                  <div className="contact-card__avatar">
                    {contact.avatar ? (
                      <img src={contact.avatar} alt="" />
                    ) : (
                      <span>👤</span>
                    )}
                    <span className={`status-dot status-dot--${contact.online_status || 'offline'}`} />
                  </div>
                  <div className="contact-card__info">
                    <h3>{contact.nickname || contact.username || contact.user_id}</h3>
                    {contact.remark && <p className="contact-card__remark">备注: {contact.remark}</p>}
                    <p className="contact-card__meta">
                      添加时间: {new Date(contact.created_at).toLocaleDateString()}
                    </p>
                  </div>
                  <div className="contact-card__actions">
                    <button
                      className="btn btn--small btn--secondary"
                      onClick={() => window.location.href = `/chat?user=${contact.user_id}`}
                    >
                      💬 聊天
                    </button>
                    <button
                      className="btn btn--small btn--danger"
                      onClick={() => handleDeleteContact(contact.id)}
                    >
                      删除
                    </button>
                  </div>
                </div>
              ))
            )}
          </div>
        ) : (
          <div className="quick-replies-list">
            <div className="quick-replies-header">
              <h2>快捷回复</h2>
              <button
                className="btn btn--primary"
                onClick={handleCreateQuickReply}
              >
                + 添加快捷回复
              </button>
            </div>
            {quickReplies.length === 0 ? (
              <div className="quick-replies-empty">
                <span>⚡</span>
                <p>暂无快捷回复</p>
                <p className="hint">快捷回复可以帮助您快速回复常用消息</p>
              </div>
            ) : (
              quickReplies.map((reply) => (
                <div key={reply.id} className="quick-reply-card">
                  <div className="quick-reply-card__shortcut">
                    /{reply.shortcut}
                  </div>
                  <div className="quick-reply-card__content">
                    {reply.content}
                  </div>
                  <button
                    className="btn btn--small btn--danger"
                    onClick={() => handleDeleteQuickReply(reply.id)}
                  >
                    删除
                  </button>
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
// 子组件：资料字段
// ============================================================

function ProfileField({
  label,
  value,
  placeholder,
  editable = true,
  copyable = false,
  editing = false,
  editValue = '',
  onStartEdit,
  onSave,
  onCancel,
  onChange,
  disabled = false,
}: {
  label: string
  value?: string
  placeholder?: string
  editable?: boolean
  copyable?: boolean
  editing?: boolean
  editValue?: string
  onStartEdit?: () => void
  onSave?: () => void
  onCancel?: () => void
  onChange?: (value: string) => void
  disabled?: boolean
}) {
  const handleCopy = () => {
    if (value) {
      navigator.clipboard.writeText(value)
      alert('已复制到剪贴板')
    }
  }

  return (
    <div className="profile-field">
      <label className="profile-field__label">{label}</label>
      {editing ? (
        <div className="profile-field__edit">
          <input
            value={editValue}
            onChange={(e) => onChange?.(e.target.value)}
            placeholder={placeholder}
            disabled={disabled}
            autoFocus
          />
          <div className="profile-field__actions">
            <button
              className="btn btn--small btn--primary"
              onClick={onSave}
              disabled={disabled}
            >
              保存
            </button>
            <button
              className="btn btn--small btn--secondary"
              onClick={onCancel}
              disabled={disabled}
            >
              取消
            </button>
          </div>
        </div>
      ) : (
        <div className="profile-field__value">
          <span className={value ? '' : 'placeholder'}>
            {value || placeholder || '未设置'}
          </span>
          <div className="profile-field__actions">
            {editable && (
              <button
                className="btn btn--small btn--secondary"
                onClick={onStartEdit}
              >
                ✏️ 编辑
              </button>
            )}
            {copyable && value && (
              <button
                className="btn btn--small btn--secondary"
                onClick={handleCopy}
              >
                📋 复制
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  )
}
