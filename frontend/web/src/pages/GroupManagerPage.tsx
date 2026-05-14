import React, { useState, useEffect, useCallback } from 'react'
import { apiRequest } from '../services/api'
import './GroupManagerPage.css'

// ============================================================
// 类型定义
// ============================================================

interface GroupMember {
  user_id: string
  username?: string
  avatar?: string
  role: 'owner' | 'admin' | 'member'
  joined_at: string
}

interface GroupInfo {
  id: string
  name: string
  type: 'direct' | 'group' | 'ai'
  avatar?: string
  description?: string
  announcement?: string
  member_count?: number
  is_pinned?: boolean
  is_muted?: boolean
  is_archived?: boolean
  created_at: string
  updated_at: string
}

interface CreateGroupRequest {
  name: string
  type: 'group'
  member_ids: string[]
  description?: string
}

// ============================================================
// 子组件：创建群聊对话框
// ============================================================

function CreateGroupDialog({
  onClose,
  onCreate,
}: {
  onClose: () => void
  onCreate: (data: CreateGroupRequest) => void
}) {
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [memberIds, setMemberIds] = useState('')
  const [step, setStep] = useState<'info' | 'members'>('info')

  const handleSubmit = () => {
    if (!name.trim()) return
    const ids = memberIds
      .split(/[,\n]/)
      .map((s) => s.trim())
      .filter(Boolean)
    onCreate({
      name: name.trim(),
      type: 'group',
      member_ids: ids,
      description: description.trim() || undefined,
    })
  }

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog" onClick={(e) => e.stopPropagation()}>
        <div className="dialog__header">
          <h3>创建群聊</h3>
          <button className="dialog__close" onClick={onClose}>✕</button>
        </div>

        <div className="dialog__body">
          {/* 步骤指示器 */}
          <div className="create-steps">
            <div className={`create-step ${step === 'info' ? 'active' : ''}`}>
              1. 群信息
            </div>
            <div className={`create-step ${step === 'members' ? 'active' : ''}`}>
              2. 添加成员
            </div>
          </div>

          {step === 'info' ? (
            <div className="create-step-content">
              <div className="form-group">
                <label>群名称 *</label>
                <input
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="输入群聊名称"
                  required
                />
              </div>
              <div className="form-group">
                <label>群描述</label>
                <textarea
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  placeholder="输入群聊描述（可选）"
                  rows={3}
                />
              </div>
              <div className="dialog__actions">
                <button className="btn btn--secondary" onClick={onClose}>取消</button>
                <button
                  className="btn btn--primary"
                  onClick={() => setStep('members')}
                  disabled={!name.trim()}
                >
                  下一步 →
                </button>
              </div>
            </div>
          ) : (
            <div className="create-step-content">
              <div className="form-group">
                <label>成员 ID</label>
                <textarea
                  value={memberIds}
                  onChange={(e) => setMemberIds(e.target.value)}
                  placeholder="输入用户 ID，用逗号或换行分隔"
                  rows={4}
                />
                <p className="form-hint">创建后也可以在群管理中添加成员</p>
              </div>
              <div className="dialog__actions">
                <button className="btn btn--secondary" onClick={() => setStep('info')}>
                  ← 上一步
                </button>
                <button className="btn btn--primary" onClick={handleSubmit}>
                  ✅ 创建群聊
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

// ============================================================
// 子组件：群信息编辑对话框
// ============================================================

function EditGroupDialog({
  group,
  onClose,
  onSave,
}: {
  group: GroupInfo
  onClose: () => void
  onSave: (data: Partial<GroupInfo>) => void
}) {
  const [name, setName] = useState(group.name)
  const [description, setDescription] = useState(group.description || '')
  const [announcement, setAnnouncement] = useState(group.announcement || '')

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    onSave({ name, description, announcement })
  }

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog" onClick={(e) => e.stopPropagation()}>
        <div className="dialog__header">
          <h3>编辑群信息</h3>
          <button className="dialog__close" onClick={onClose}>✕</button>
        </div>

        <form onSubmit={handleSubmit} className="dialog__body">
          <div className="form-group">
            <label>群名称</label>
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              required
            />
          </div>
          <div className="form-group">
            <label>群描述</label>
            <textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={3}
            />
          </div>
          <div className="form-group">
            <label>群公告</label>
            <textarea
              value={announcement}
              onChange={(e) => setAnnouncement(e.target.value)}
              rows={3}
              placeholder="设置群公告，所有成员可见"
            />
          </div>
          <div className="dialog__actions">
            <button type="button" className="btn btn--secondary" onClick={onClose}>取消</button>
            <button type="submit" className="btn btn--primary">保存</button>
          </div>
        </form>
      </div>
    </div>
  )
}

// ============================================================
// 子组件：成员管理面板
// ============================================================

function MemberPanel({
  groupId,
  members,
  onRefresh,
}: {
  groupId: string
  members: GroupMember[]
  onRefresh: () => void
}) {
  const [addMemberId, setAddMemberId] = useState('')
  const [loading, setLoading] = useState(false)

  const handleAddMember = async () => {
    if (!addMemberId.trim()) return
    setLoading(true)
    try {
      await apiRequest(`/api/im/conversations/${groupId}/members`, {
        method: 'POST',
        body: JSON.stringify({ user_id: addMemberId.trim() }),
      })
      setAddMemberId('')
      onRefresh()
    } catch (err: any) {
      alert(err.message || '添加成员失败')
    } finally {
      setLoading(false)
    }
  }

  const handleRemoveMember = async (memberId: string) => {
    if (!confirm('确定移除该成员吗？')) return
    try {
      await apiRequest(`/api/im/conversations/${groupId}/members/${memberId}`, {
        method: 'DELETE',
      })
      onRefresh()
    } catch (err: any) {
      alert(err.message || '移除成员失败')
    }
  }

  const handleUpdateRole = async (memberId: string, newRole: string) => {
    try {
      await apiRequest(`/api/im/conversations/${groupId}/members/${memberId}/role`, {
        method: 'PUT',
        body: JSON.stringify({ role: newRole }),
      })
      onRefresh()
    } catch (err: any) {
      alert(err.message || '更新角色失败')
    }
  }

  return (
    <div className="member-panel">
      <div className="member-panel__header">
        <h3>成员管理 ({members.length})</h3>
      </div>

      {/* 添加成员 */}
      <div className="member-add">
        <input
          value={addMemberId}
          onChange={(e) => setAddMemberId(e.target.value)}
          placeholder="输入用户 ID"
        />
        <button
          className="btn btn--small btn--primary"
          onClick={handleAddMember}
          disabled={loading || !addMemberId.trim()}
        >
          {loading ? '...' : '+ 添加'}
        </button>
      </div>

      {/* 成员列表 */}
      <div className="member-list">
        {members.map((member) => (
          <div key={member.user_id} className="member-item">
            <div className="member-item__avatar">
              {member.avatar ? (
                <img src={member.avatar} alt="" />
              ) : (
                <span>👤</span>
              )}
            </div>
            <div className="member-item__info">
              <div className="member-item__name">
                {member.username || member.user_id}
              </div>
              <div className="member-item__meta">
                加入时间: {new Date(member.joined_at).toLocaleDateString()}
              </div>
            </div>
            <div className="member-item__actions">
              <select
                value={member.role}
                onChange={(e) => handleUpdateRole(member.user_id, e.target.value)}
                className="role-select"
                disabled={member.role === 'owner'}
              >
                <option value="owner">群主</option>
                <option value="admin">管理员</option>
                <option value="member">成员</option>
              </select>
              {member.role !== 'owner' && (
                <button
                  className="btn btn--small btn--danger"
                  onClick={() => handleRemoveMember(member.user_id)}
                >
                  移除
                </button>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}

// ============================================================
// 主页面组件
// ============================================================

export default function GroupManagerPage() {
  const [groups, setGroups] = useState<GroupInfo[]>([])
  const [selectedGroup, setSelectedGroup] = useState<GroupInfo | null>(null)
  const [members, setMembers] = useState<GroupMember[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [showCreateDialog, setShowCreateDialog] = useState(false)
  const [showEditDialog, setShowEditDialog] = useState(false)
  const [activeTab, setActiveTab] = useState<'info' | 'members'>('info')

  // 加载群聊列表
  const loadGroups = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const data = await apiRequest<any[]>('/api/im/conversations')
      // 过滤出群聊类型
      const groupList = data.filter((c: any) => c.type === 'group')
      setGroups(groupList)
    } catch (err: any) {
      setError(err.message || '加载群聊列表失败')
    } finally {
      setLoading(false)
    }
  }, [])

  // 加载群成员
  const loadMembers = useCallback(async (groupId: string) => {
    try {
      const data = await apiRequest<GroupMember[]>(`/api/im/conversations/${groupId}/members`)
      setMembers(data)
    } catch (err: any) {
      console.error('Failed to load members:', err)
    }
  }, [])

  useEffect(() => {
    loadGroups()
  }, [loadGroups])

  useEffect(() => {
    if (selectedGroup) {
      loadMembers(selectedGroup.id)
    }
  }, [selectedGroup, loadMembers])

  // 创建群聊
  const handleCreate = async (data: CreateGroupRequest) => {
    try {
      await apiRequest('/api/im/conversations', {
        method: 'POST',
        body: JSON.stringify(data),
      })
      setShowCreateDialog(false)
      loadGroups()
    } catch (err: any) {
      setError(err.message || '创建群聊失败')
    }
  }

  // 更新群信息
  const handleUpdateGroup = async (data: Partial<GroupInfo>) => {
    if (!selectedGroup) return
    try {
      await apiRequest(`/api/im/conversations/${selectedGroup.id}/group`, {
        method: 'PUT',
        body: JSON.stringify(data),
      })
      // 更新公告
      if (data.announcement !== undefined) {
        await apiRequest(`/api/im/conversations/${selectedGroup.id}/announcement`, {
          method: 'PUT',
          body: JSON.stringify({ content: data.announcement }),
        })
      }
      setShowEditDialog(false)
      loadGroups()
    } catch (err: any) {
      setError(err.message || '更新群信息失败')
    }
  }

  // 切换置顶
  const handleTogglePin = async (groupId: string) => {
    try {
      await apiRequest(`/api/im/conversations/${groupId}/pin`, {
        method: 'PUT',
      })
      loadGroups()
    } catch (err: any) {
      setError(err.message || '操作失败')
    }
  }

  // 切换免打扰
  const handleToggleMute = async (groupId: string) => {
    try {
      await apiRequest(`/api/im/conversations/${groupId}/mute`, {
        method: 'PUT',
      })
      loadGroups()
    } catch (err: any) {
      setError(err.message || '操作失败')
    }
  }

  // 归档群聊
  const handleToggleArchive = async (groupId: string) => {
    try {
      await apiRequest(`/api/im/conversations/${groupId}/archive`, {
        method: 'PUT',
      })
      loadGroups()
    } catch (err: any) {
      setError(err.message || '操作失败')
    }
  }

  return (
    <div className="group-manager-page">
      {/* 左侧：群聊列表 */}
      <div className="group-sidebar">
        <div className="group-sidebar__header">
          <h2>👥 群聊管理</h2>
          <button
            className="btn btn--small btn--primary"
            onClick={() => setShowCreateDialog(true)}
          >
            + 创建群聊
          </button>
        </div>

        {error && (
          <div className="group-error">
            <span>{error}</span>
            <button onClick={() => setError(null)}>✕</button>
          </div>
        )}

        {loading ? (
          <div className="group-loading">加载中...</div>
        ) : groups.length === 0 ? (
          <div className="group-empty">
            <span>📭</span>
            <p>暂无群聊</p>
          </div>
        ) : (
          <div className="group-list">
            {groups.map((group) => (
              <button
                key={group.id}
                className={`group-item ${selectedGroup?.id === group.id ? 'active' : ''}`}
                onClick={() => setSelectedGroup(group)}
              >
                <div className="group-item__avatar">
                  {group.avatar ? (
                    <img src={group.avatar} alt="" />
                  ) : (
                    <span>👥</span>
                  )}
                </div>
                <div className="group-item__info">
                  <div className="group-item__name">{group.name}</div>
                  <div className="group-item__meta">
                    {group.member_count || 0} 成员
                    {group.is_pinned && ' • 📌'}
                    {group.is_muted && ' • 🔇'}
                  </div>
                </div>
              </button>
            ))}
          </div>
        )}
      </div>

      {/* 右侧：群详情 */}
      <div className="group-detail">
        {selectedGroup ? (
          <>
            {/* 群头像和名称 */}
            <div className="group-detail__header">
              <div className="group-detail__avatar">
                {selectedGroup.avatar ? (
                  <img src={selectedGroup.avatar} alt="" />
                ) : (
                  <span>👥</span>
                )}
              </div>
              <div className="group-detail__title">
                <h2>{selectedGroup.name}</h2>
                <p>{selectedGroup.description || '暂无描述'}</p>
              </div>
              <div className="group-detail__actions">
                <button
                  className="btn btn--small btn--secondary"
                  onClick={() => setShowEditDialog(true)}
                >
                  ✏️ 编辑
                </button>
                <button
                  className={`btn btn--small ${selectedGroup.is_pinned ? 'btn--primary' : 'btn--secondary'}`}
                  onClick={() => handleTogglePin(selectedGroup.id)}
                >
                  📌 {selectedGroup.is_pinned ? '已置顶' : '置顶'}
                </button>
                <button
                  className={`btn btn--small ${selectedGroup.is_muted ? 'btn--primary' : 'btn--secondary'}`}
                  onClick={() => handleToggleMute(selectedGroup.id)}
                >
                  {selectedGroup.is_muted ? '🔔 开启通知' : '🔇 免打扰'}
                </button>
                <button
                  className="btn btn--small btn--secondary"
                  onClick={() => handleToggleArchive(selectedGroup.id)}
                >
                  {selectedGroup.is_archived ? '📤 取消归档' : '📥 归档'}
                </button>
              </div>
            </div>

            {/* Tab 切换 */}
            <div className="group-tabs">
              <button
                className={`group-tab ${activeTab === 'info' ? 'active' : ''}`}
                onClick={() => setActiveTab('info')}
              >
                📋 群信息
              </button>
              <button
                className={`group-tab ${activeTab === 'members' ? 'active' : ''}`}
                onClick={() => setActiveTab('members')}
              >
                👥 成员管理
              </button>
            </div>

            {/* Tab 内容 */}
            <div className="group-tab-content">
              {activeTab === 'info' ? (
                <div className="group-info-panel">
                  <div className="info-section">
                    <h3>基本信息</h3>
                    <div className="info-grid">
                      <div className="info-item">
                        <span className="info-label">群名称</span>
                        <span className="info-value">{selectedGroup.name}</span>
                      </div>
                      <div className="info-item">
                        <span className="info-label">群描述</span>
                        <span className="info-value">{selectedGroup.description || '暂无'}</span>
                      </div>
                      <div className="info-item">
                        <span className="info-label">创建时间</span>
                        <span className="info-value">
                          {new Date(selectedGroup.created_at).toLocaleString()}
                        </span>
                      </div>
                      <div className="info-item">
                        <span className="info-label">成员数</span>
                        <span className="info-value">{selectedGroup.member_count || members.length}</span>
                      </div>
                    </div>
                  </div>

                  {selectedGroup.announcement && (
                    <div className="info-section">
                      <h3>📢 群公告</h3>
                      <div className="announcement-content">
                        {selectedGroup.announcement}
                      </div>
                    </div>
                  )}
                </div>
              ) : (
                <MemberPanel
                  groupId={selectedGroup.id}
                  members={members}
                  onRefresh={() => loadMembers(selectedGroup.id)}
                />
              )}
            </div>
          </>
        ) : (
          <div className="group-detail__empty">
            <span>👈</span>
            <p>选择一个群聊查看详情</p>
          </div>
        )}
      </div>

      {/* 创建群聊对话框 */}
      {showCreateDialog && (
        <CreateGroupDialog
          onClose={() => setShowCreateDialog(false)}
          onCreate={handleCreate}
        />
      )}

      {/* 编辑群信息对话框 */}
      {showEditDialog && selectedGroup && (
        <EditGroupDialog
          group={selectedGroup}
          onClose={() => setShowEditDialog(false)}
          onSave={handleUpdateGroup}
        />
      )}
    </div>
  )
}
