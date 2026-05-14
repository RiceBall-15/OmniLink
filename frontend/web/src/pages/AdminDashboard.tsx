import React, { useState, useEffect, useCallback } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuth } from '../hooks/useAuth'
import { adminService } from '../services/adminService'
import type {
  Announcement,
  CreateAnnouncementRequest,
  Feedback,
  FeedbackStats,
  RateLimitConfig,
  LogLevelConfig,
  HealthStatus,
  BusinessMetrics,
} from '../services/adminService'
import './AdminDashboard.css'

// Tab 类型
type AdminTab = 'overview' | 'announcements' | 'feedbacks' | 'settings' | 'health'

// 统计卡片组件
function StatCard({ icon, label, value, trend, color }: {
  icon: string
  label: string
  value: string | number
  trend?: string
  color: string
}) {
  return (
    <div className={`stat-card stat-card--${color}`}>
      <div className="stat-card__icon">{icon}</div>
      <div className="stat-card__content">
        <div className="stat-card__value">{value}</div>
        <div className="stat-card__label">{label}</div>
        {trend && <div className="stat-card__trend">{trend}</div>}
      </div>
    </div>
  )
}

// 公告表单组件
function AnnouncementForm({
  initial,
  onSubmit,
  onCancel,
}: {
  initial?: Partial<Announcement>
  onSubmit: (data: CreateAnnouncementRequest) => void
  onCancel: () => void
}) {
  const [title, setTitle] = useState(initial?.title || '')
  const [content, setContent] = useState(initial?.content || '')
  const [type, setType] = useState<CreateAnnouncementRequest['type']>(initial?.type || 'info')
  const [priority, setPriority] = useState(initial?.priority || 0)
  const [expiresAt, setExpiresAt] = useState(initial?.expires_at || '')

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    onSubmit({
      title,
      content,
      type,
      priority,
      expires_at: expiresAt || undefined,
    })
  }

  return (
    <form className="announcement-form" onSubmit={handleSubmit}>
      <h3>{initial?.id ? '编辑公告' : '创建公告'}</h3>
      <div className="form-group">
        <label>标题</label>
        <input
          type="text"
          value={title}
          onChange={e => setTitle(e.target.value)}
          placeholder="公告标题"
          required
        />
      </div>
      <div className="form-group">
        <label>内容</label>
        <textarea
          value={content}
          onChange={e => setContent(e.target.value)}
          placeholder="公告内容"
          rows={4}
          required
        />
      </div>
      <div className="form-row">
        <div className="form-group">
          <label>类型</label>
          <select value={type} onChange={e => setType(e.target.value as any)}>
            <option value="info">ℹ️ 信息</option>
            <option value="warning">⚠️ 警告</option>
            <option value="maintenance">🔧 维护</option>
          </select>
        </div>
        <div className="form-group">
          <label>优先级</label>
          <input
            type="number"
            value={priority}
            onChange={e => setPriority(Number(e.target.value))}
            min={0}
            max={10}
          />
        </div>
        <div className="form-group">
          <label>过期时间</label>
          <input
            type="datetime-local"
            value={expiresAt}
            onChange={e => setExpiresAt(e.target.value)}
          />
        </div>
      </div>
      <div className="form-actions">
        <button type="button" className="btn btn--secondary" onClick={onCancel}>取消</button>
        <button type="submit" className="btn btn--primary">{initial?.id ? '保存' : '创建'}</button>
      </div>
    </form>
  )
}

// 主仪表板组件
export default function AdminDashboard() {
  const navigate = useNavigate()
  const { user } = useAuth()
  const [activeTab, setActiveTab] = useState<AdminTab>('overview')
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // 数据状态
  const [metrics, setMetrics] = useState<BusinessMetrics | null>(null)
  const [health, setHealth] = useState<HealthStatus | null>(null)
  const [announcements, setAnnouncements] = useState<Announcement[]>([])
  const [feedbacks, setFeedbacks] = useState<Feedback[]>([])
  const [feedbackStats, setFeedbackStats] = useState<FeedbackStats | null>(null)
  const [rateLimitConfig, setRateLimitConfig] = useState<RateLimitConfig | null>(null)
  const [logLevel, setLogLevel] = useState<LogLevelConfig | null>(null)

  // 表单状态
  const [showAnnouncementForm, setShowAnnouncementForm] = useState(false)
  const [editingAnnouncement, setEditingAnnouncement] = useState<Announcement | null>(null)
  const [feedbackFilter, setFeedbackFilter] = useState<{ status?: string; type?: string }>({})

  // 检查管理员权限
  useEffect(() => {
    if (user && user.role !== 'admin') {
      navigate('/chat')
    }
  }, [user, navigate])

  // 加载概览数据
  const loadOverview = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const [metricsRes, healthRes] = await Promise.all([
        adminService.getMetrics(),
        adminService.getHealth(),
      ])
      if (metricsRes.success && metricsRes.data) setMetrics(metricsRes.data)
      if (healthRes.success && healthRes.data) setHealth(healthRes.data)
    } catch {
      setError('加载数据失败')
    } finally {
      setLoading(false)
    }
  }, [])

  // 加载公告
  const loadAnnouncements = useCallback(async () => {
    setLoading(true)
    try {
      const res = await adminService.getAnnouncements()
      if (res.success && res.data) setAnnouncements(res.data)
    } catch {
      setError('加载公告失败')
    } finally {
      setLoading(false)
    }
  }, [])

  // 加载反馈
  const loadFeedbacks = useCallback(async () => {
    setLoading(true)
    try {
      const [feedbackRes, statsRes] = await Promise.all([
        adminService.getFeedbacks(feedbackFilter),
        adminService.getFeedbackStats(),
      ])
      if (feedbackRes.success && feedbackRes.data) setFeedbacks(feedbackRes.data)
      if (statsRes.success && statsRes.data) setFeedbackStats(statsRes.data)
    } catch {
      setError('加载反馈失败')
    } finally {
      setLoading(false)
    }
  }, [feedbackFilter])

  // 加载设置
  const loadSettings = useCallback(async () => {
    setLoading(true)
    try {
      const [rlRes, llRes] = await Promise.all([
        adminService.getRateLimitConfig(),
        adminService.getLogLevel(),
      ])
      if (rlRes.success && rlRes.data) setRateLimitConfig(rlRes.data)
      if (llRes.success && llRes.data) setLogLevel(llRes.data)
    } catch {
      setError('加载设置失败')
    } finally {
      setLoading(false)
    }
  }, [])

  // Tab 切换时加载数据
  useEffect(() => {
    switch (activeTab) {
      case 'overview': loadOverview(); break
      case 'announcements': loadAnnouncements(); break
      case 'feedbacks': loadFeedbacks(); break
      case 'settings': loadSettings(); break
      case 'health': loadOverview(); break
    }
  }, [activeTab, loadOverview, loadAnnouncements, loadFeedbacks, loadSettings])

  // 创建/编辑公告
  const handleSaveAnnouncement = async (data: CreateAnnouncementRequest) => {
    try {
      if (editingAnnouncement) {
        await adminService.updateAnnouncement(editingAnnouncement.id, data)
      } else {
        await adminService.createAnnouncement(data)
      }
      setShowAnnouncementForm(false)
      setEditingAnnouncement(null)
      loadAnnouncements()
    } catch {
      setError('保存公告失败')
    }
  }

  // 删除公告
  const handleDeleteAnnouncement = async (id: string) => {
    if (!confirm('确定要删除此公告吗？')) return
    try {
      await adminService.deleteAnnouncement(id)
      loadAnnouncements()
    } catch {
      setError('删除公告失败')
    }
  }

  // 更新反馈状态
  const handleUpdateFeedback = async (id: string, data: { status?: string; admin_reply?: string }) => {
    try {
      await adminService.updateFeedback(id, data)
      loadFeedbacks()
    } catch {
      setError('更新反馈失败')
    }
  }

  // 保存限流配置
  const handleSaveRateLimit = async () => {
    if (!rateLimitConfig) return
    try {
      await adminService.updateRateLimitConfig(rateLimitConfig)
      alert('限流配置已更新')
    } catch {
      setError('保存限流配置失败')
    }
  }

  // 保存日志级别
  const handleSaveLogLevel = async () => {
    if (!logLevel) return
    try {
      await adminService.updateLogLevel(logLevel)
      alert('日志级别已更新')
    } catch {
      setError('保存日志级别失败')
    }
  }

  // 格式化数字
  const formatNumber = (n: number): string => {
    if (n >= 1000000) return (n / 1000000).toFixed(1) + 'M'
    if (n >= 1000) return (n / 1000).toFixed(1) + 'K'
    return n.toString()
  }

  // 格式化运行时间
  const formatUptime = (seconds: number): string => {
    const days = Math.floor(seconds / 86400)
    const hours = Math.floor((seconds % 86400) / 3600)
    const minutes = Math.floor((seconds % 3600) / 60)
    if (days > 0) return `${days}天 ${hours}小时`
    if (hours > 0) return `${hours}小时 ${minutes}分钟`
    return `${minutes}分钟`
  }

  // 状态颜色
  const statusColor = (status: string): string => {
    switch (status) {
      case 'ok': case 'healthy': return 'green'
      case 'degraded': return 'yellow'
      case 'error': case 'unhealthy': return 'red'
      default: return 'gray'
    }
  }

  return (
    <div className="admin-dashboard">
      {/* 侧边栏 */}
      <aside className="admin-sidebar">
        <div className="admin-sidebar__header">
          <h2>⚙️ 管理后台</h2>
          <button className="btn btn--text" onClick={() => navigate('/chat')}>返回聊天</button>
        </div>
        <nav className="admin-nav">
          <button
            className={`admin-nav__item ${activeTab === 'overview' ? 'active' : ''}`}
            onClick={() => setActiveTab('overview')}
          >
            📊 系统概览
          </button>
          <button
            className={`admin-nav__item ${activeTab === 'announcements' ? 'active' : ''}`}
            onClick={() => setActiveTab('announcements')}
          >
            📢 系统公告
          </button>
          <button
            className={`admin-nav__item ${activeTab === 'feedbacks' ? 'active' : ''}`}
            onClick={() => setActiveTab('feedbacks')}
          >
            💬 用户反馈
          </button>
          <button
            className={`admin-nav__item ${activeTab === 'settings' ? 'active' : ''}`}
            onClick={() => setActiveTab('settings')}
          >
            ⚙️ 系统设置
          </button>
          <button
            className={`admin-nav__item ${activeTab === 'health' ? 'active' : ''}`}
            onClick={() => setActiveTab('health')}
          >
            🏥 健康检查
          </button>
        </nav>
      </aside>

      {/* 主内容区 */}
      <main className="admin-content">
        {error && (
          <div className="admin-error">
            <span>{error}</span>
            <button onClick={() => setError(null)}>✕</button>
          </div>
        )}

        {loading && <div className="admin-loading">加载中...</div>}

        {/* 系统概览 */}
        {activeTab === 'overview' && metrics && (
          <div className="admin-section">
            <h1>系统概览</h1>
            <div className="stats-grid">
              <StatCard
                icon="📨"
                label="消息总数"
                value={formatNumber(metrics.total_messages_sent + metrics.total_messages_received)}
                color="blue"
              />
              <StatCard
                icon="👥"
                label="注册用户"
                value={formatNumber(metrics.total_users_registered)}
                color="green"
              />
              <StatCard
                icon="💬"
                label="会话总数"
                value={formatNumber(metrics.total_conversations_created)}
                color="purple"
              />
              <StatCard
                icon="🔌"
                label="WebSocket连接"
                value={formatNumber(metrics.total_ws_connections)}
                color="orange"
              />
              <StatCard
                icon="📊"
                label="总请求数"
                value={formatNumber(metrics.total_requests)}
                color="cyan"
              />
              <StatCard
                icon="❌"
                label="错误总数"
                value={formatNumber(metrics.total_errors)}
                color="red"
              />
              <StatCard
                icon="🔒"
                label="认证失败"
                value={formatNumber(metrics.total_auth_failures)}
                color="pink"
              />
              <StatCard
                icon="⏱️"
                label="运行时间"
                value={formatUptime(metrics.uptime_seconds)}
                color="teal"
              />
            </div>
          </div>
        )}

        {/* 系统公告管理 */}
        {activeTab === 'announcements' && (
          <div className="admin-section">
            <div className="section-header">
              <h1>系统公告</h1>
              <button
                className="btn btn--primary"
                onClick={() => {
                  setEditingAnnouncement(null)
                  setShowAnnouncementForm(true)
                }}
              >
                + 创建公告
              </button>
            </div>

            {showAnnouncementForm && (
              <AnnouncementForm
                initial={editingAnnouncement || undefined}
                onSubmit={handleSaveAnnouncement}
                onCancel={() => {
                  setShowAnnouncementForm(false)
                  setEditingAnnouncement(null)
                }}
              />
            )}

            <div className="announcement-list">
              {announcements.length === 0 ? (
                <div className="empty-state">暂无公告</div>
              ) : (
                announcements.map(a => (
                  <div key={a.id} className={`announcement-card announcement-card--${a.type}`}>
                    <div className="announcement-card__header">
                      <span className="announcement-card__type">
                        {a.type === 'info' ? 'ℹ️' : a.type === 'warning' ? '⚠️' : '🔧'}
                      </span>
                      <h3>{a.title}</h3>
                      <span className="announcement-card__priority">P{a.priority}</span>
                    </div>
                    <p className="announcement-card__content">{a.content}</p>
                    <div className="announcement-card__footer">
                      <span className="announcement-card__date">
                        {new Date(a.created_at).toLocaleString()}
                      </span>
                      {a.expires_at && (
                        <span className="announcement-card__expires">
                          过期: {new Date(a.expires_at).toLocaleString()}
                        </span>
                      )}
                      <div className="announcement-card__actions">
                        <button
                          className="btn btn--small btn--secondary"
                          onClick={() => {
                            setEditingAnnouncement(a)
                            setShowAnnouncementForm(true)
                          }}
                        >
                          编辑
                        </button>
                        <button
                          className="btn btn--small btn--danger"
                          onClick={() => handleDeleteAnnouncement(a.id)}
                        >
                          删除
                        </button>
                      </div>
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>
        )}

        {/* 用户反馈管理 */}
        {activeTab === 'feedbacks' && (
          <div className="admin-section">
            <h1>用户反馈</h1>

            {feedbackStats && (
              <div className="feedback-stats">
                <div className="feedback-stat">
                  <span className="feedback-stat__value">{feedbackStats.total}</span>
                  <span className="feedback-stat__label">总计</span>
                </div>
                <div className="feedback-stat feedback-stat--pending">
                  <span className="feedback-stat__value">{feedbackStats.pending}</span>
                  <span className="feedback-stat__label">待处理</span>
                </div>
                <div className="feedback-stat feedback-stat--progress">
                  <span className="feedback-stat__value">{feedbackStats.in_progress}</span>
                  <span className="feedback-stat__label">处理中</span>
                </div>
                <div className="feedback-stat feedback-stat--resolved">
                  <span className="feedback-stat__value">{feedbackStats.resolved}</span>
                  <span className="feedback-stat__label">已解决</span>
                </div>
              </div>
            )}

            <div className="feedback-filters">
              <select
                value={feedbackFilter.status || ''}
                onChange={e => setFeedbackFilter(prev => ({ ...prev, status: e.target.value || undefined }))}
              >
                <option value="">全部状态</option>
                <option value="pending">待处理</option>
                <option value="in_progress">处理中</option>
                <option value="resolved">已解决</option>
                <option value="rejected">已拒绝</option>
              </select>
              <select
                value={feedbackFilter.type || ''}
                onChange={e => setFeedbackFilter(prev => ({ ...prev, type: e.target.value || undefined }))}
              >
                <option value="">全部类型</option>
                <option value="bug">Bug</option>
                <option value="feature">功能建议</option>
                <option value="other">其他</option>
              </select>
            </div>

            <div className="feedback-list">
              {feedbacks.length === 0 ? (
                <div className="empty-state">暂无反馈</div>
              ) : (
                feedbacks.map(f => (
                  <div key={f.id} className={`feedback-card feedback-card--${f.status}`}>
                    <div className="feedback-card__header">
                      <span className={`feedback-card__type feedback-card__type--${f.type}`}>
                        {f.type === 'bug' ? '🐛' : f.type === 'feature' ? '💡' : '📝'}
                      </span>
                      <span className="feedback-card__date">
                        {new Date(f.created_at).toLocaleString()}
                      </span>
                      <span className={`feedback-card__status feedback-card__status--${f.status}`}>
                        {f.status === 'pending' ? '待处理' :
                         f.status === 'in_progress' ? '处理中' :
                         f.status === 'resolved' ? '已解决' : '已拒绝'}
                      </span>
                    </div>
                    <p className="feedback-card__content">{f.content}</p>
                    {f.admin_reply && (
                      <div className="feedback-card__reply">
                        <strong>管理员回复：</strong> {f.admin_reply}
                      </div>
                    )}
                    <div className="feedback-card__actions">
                      {f.status === 'pending' && (
                        <>
                          <button
                            className="btn btn--small btn--primary"
                            onClick={() => handleUpdateFeedback(f.id, { status: 'in_progress' })}
                          >
                            开始处理
                          </button>
                          <button
                            className="btn btn--small btn--danger"
                            onClick={() => handleUpdateFeedback(f.id, { status: 'rejected' })}
                          >
                            拒绝
                          </button>
                        </>
                      )}
                      {f.status === 'in_progress' && (
                        <button
                          className="btn btn--small btn--success"
                          onClick={() => {
                            const reply = prompt('管理员回复（可选）:')
                            handleUpdateFeedback(f.id, {
                              status: 'resolved',
                              admin_reply: reply || undefined,
                            })
                          }}
                        >
                          标记已解决
                        </button>
                      )}
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>
        )}

        {/* 系统设置 */}
        {activeTab === 'settings' && (
          <div className="admin-section">
            <h1>系统设置</h1>

            {/* 限流配置 */}
            <div className="settings-card">
              <h2>🔒 限流配置</h2>
              {rateLimitConfig && (
                <div className="settings-form">
                  <div className="form-group">
                    <label>
                      <input
                        type="checkbox"
                        checked={rateLimitConfig.enabled}
                        onChange={e => setRateLimitConfig({ ...rateLimitConfig, enabled: e.target.checked })}
                      />
                      启用限流
                    </label>
                  </div>
                  <div className="form-group">
                    <label>默认限制（请求数/窗口）</label>
                    <input
                      type="number"
                      value={rateLimitConfig.default_limit}
                      onChange={e => setRateLimitConfig({ ...rateLimitConfig, default_limit: Number(e.target.value) })}
                    />
                  </div>
                  <div className="form-group">
                    <label>窗口时间（秒）</label>
                    <input
                      type="number"
                      value={rateLimitConfig.default_window_seconds}
                      onChange={e => setRateLimitConfig({ ...rateLimitConfig, default_window_seconds: Number(e.target.value) })}
                    />
                  </div>
                  <button className="btn btn--primary" onClick={handleSaveRateLimit}>
                    保存限流配置
                  </button>
                </div>
              )}
            </div>

            {/* 日志级别 */}
            <div className="settings-card">
              <h2>📋 日志级别</h2>
              {logLevel && (
                <div className="settings-form">
                  <div className="form-group">
                    <label>全局日志级别</label>
                    <select
                      value={logLevel.level}
                      onChange={e => setLogLevel({ ...logLevel, level: e.target.value })}
                    >
                      <option value="trace">Trace</option>
                      <option value="debug">Debug</option>
                      <option value="info">Info</option>
                      <option value="warn">Warn</option>
                      <option value="error">Error</option>
                    </select>
                  </div>
                  <button className="btn btn--primary" onClick={handleSaveLogLevel}>
                    保存日志配置
                  </button>
                </div>
              )}
            </div>
          </div>
        )}

        {/* 健康检查 */}
        {activeTab === 'health' && health && (
          <div className="admin-section">
            <h1>健康检查</h1>
            <div className="health-overview">
              <div className={`health-status health-status--${statusColor(health.status)}`}>
                <span className="health-status__icon">
                  {health.status === 'ok' ? '✅' : health.status === 'degraded' ? '⚠️' : '❌'}
                </span>
                <span className="health-status__text">
                  系统状态: {health.status === 'ok' ? '正常' : health.status === 'degraded' ? '降级' : '异常'}
                </span>
              </div>
              <div className="health-info">
                <div className="health-info__item">
                  <span>版本:</span> <strong>{health.version}</strong>
                </div>
                <div className="health-info__item">
                  <span>运行时间:</span> <strong>{formatUptime(health.uptime_seconds)}</strong>
                </div>
              </div>
            </div>

            <h2>依赖服务</h2>
            <div className="dependencies-grid">
              {Object.entries(health.dependencies).map(([name, dep]) => (
                <div key={name} className={`dependency-card dependency-card--${statusColor(dep.status)}`}>
                  <div className="dependency-card__name">{name}</div>
                  <div className="dependency-card__status">
                    {dep.status === 'ok' ? '✅ 正常' : dep.status === 'degraded' ? '⚠️ 降级' : '❌ 异常'}
                  </div>
                  {dep.latency_ms !== undefined && (
                    <div className="dependency-card__latency">{dep.latency_ms}ms</div>
                  )}
                </div>
              ))}
            </div>
          </div>
        )}
      </main>
    </div>
  )
}
