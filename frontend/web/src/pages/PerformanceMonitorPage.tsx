import { useState, useEffect, useCallback } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuth } from '../hooks/useAuth'
import { useFPSMonitor, usePagePerformance, useMemoryMonitor } from '../hooks/usePerformance'
import {
  getSystemHealth,
  getRequestStats,
  getWebSocketStats,
  type SystemHealth,
  type RequestStats,
  type WebSocketStats,
  type TimeRange,
} from '../services/monitoringService'
import './PerformanceMonitorPage.css'

/**
 * 格式化运行时间
 */
function formatUptime(seconds: number): string {
  if (seconds < 60) return `${seconds}秒`
  if (seconds < 3600) return `${Math.floor(seconds / 60)}分钟`
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}小时 ${Math.floor((seconds % 3600) / 60)}分钟`
  return `${Math.floor(seconds / 86400)}天 ${Math.floor((seconds % 86400) / 3600)}小时`
}

/**
 * 格式化字节
 */
function formatBytes(mb: number): string {
  if (mb >= 1024) return `${(mb / 1024).toFixed(1)} GB`
  return `${Math.round(mb)} MB`
}

/**
 * 状态颜色
 */
function statusColor(status: string): string {
  switch (status.toLowerCase()) {
    case 'healthy':
    case 'ok':
    case 'connected':
      return 'green'
    case 'degraded':
    case 'warning':
      return 'yellow'
    case 'unhealthy':
    case 'error':
    case 'disconnected':
      return 'red'
    default:
      return 'gray'
  }
}

/**
 * 指标卡片组件
 */
function MetricCard({
  icon,
  label,
  value,
  unit,
  color,
  detail,
}: {
  icon: string
  label: string
  value: string | number
  unit?: string
  color?: string
  detail?: string
}) {
  return (
    <div className={`metric-card metric-card--${color || 'blue'}`}>
      <div className="metric-card__icon">{icon}</div>
      <div className="metric-card__content">
        <div className="metric-card__value">
          {value}
          {unit && <span className="metric-card__unit">{unit}</span>}
        </div>
        <div className="metric-card__label">{label}</div>
        {detail && <div className="metric-card__detail">{detail}</div>}
      </div>
    </div>
  )
}

/**
 * 进度条组件
 */
function ProgressBar({ value, max = 100, color }: { value: number; max?: number; color?: string }) {
  const percent = Math.min((value / max) * 100, 100)
  const barColor = color || (percent > 80 ? '#ef4444' : percent > 60 ? '#f59e0b' : '#22c55e')

  return (
    <div className="progress-bar">
      <div
        className="progress-bar__fill"
        style={{ width: `${percent}%`, backgroundColor: barColor }}
      />
      <span className="progress-bar__label">{Math.round(percent)}%</span>
    </div>
  )
}

/**
 * 性能监控页面
 */
export default function PerformanceMonitorPage() {
  const navigate = useNavigate()
  const { user } = useAuth()
  const [health, setHealth] = useState<SystemHealth | null>(null)
  const [requestStats, setRequestStats] = useState<RequestStats | null>(null)
  const [wsStats, setWsStats] = useState<WebSocketStats | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [timeRange, setTimeRange] = useState<TimeRange>('1h')
  const [lastRefresh, setLastRefresh] = useState<Date>(new Date())

  // 前端性能监控
  const { fps, avgFPS, minFPS, isLowFPS } = useFPSMonitor({ autoStart: true })
  const pagePerformance = usePagePerformance()
  const memoryInfo = useMemoryMonitor(3000)

  // 加载数据
  const fetchData = useCallback(async () => {
    try {
      setError(null)
      const [healthData, reqData, wsData] = await Promise.allSettled([
        getSystemHealth(),
        getRequestStats(timeRange),
        getWebSocketStats(),
      ])

      if (healthData.status === 'fulfilled') setHealth(healthData.value)
      if (reqData.status === 'fulfilled') setRequestStats(reqData.value)
      if (wsData.status === 'fulfilled') setWsStats(wsData.value)

      setLastRefresh(new Date())
    } catch (err: any) {
      setError(err.message || '获取监控数据失败')
    } finally {
      setLoading(false)
    }
  }, [timeRange])

  useEffect(() => {
    fetchData()
    // 每 30 秒自动刷新
    const timer = setInterval(fetchData, 30000)
    return () => clearInterval(timer)
  }, [fetchData])

  // 非管理员跳转
  if (user && user.role !== 'admin') {
    navigate('/chat')
    return null
  }

  if (loading) {
    return (
      <div className="perf-page">
        <div className="perf-page__loading">
          <div className="loading-spinner" />
          <p>加载监控数据...</p>
        </div>
      </div>
    )
  }

  return (
    <div className="perf-page">
      {/* 顶部导航 */}
      <header className="perf-page__header">
        <div className="perf-page__header-left">
          <button className="perf-page__back" onClick={() => navigate('/admin')}>
            ← 返回
          </button>
          <h1 className="perf-page__title">📊 性能监控</h1>
        </div>
        <div className="perf-page__header-right">
          <select
            className="perf-page__range"
            value={timeRange}
            onChange={e => setTimeRange(e.target.value as TimeRange)}
          >
            <option value="1h">最近 1 小时</option>
            <option value="6h">最近 6 小时</option>
            <option value="24h">最近 24 小时</option>
            <option value="7d">最近 7 天</option>
          </select>
          <button className="perf-page__refresh" onClick={fetchData}>
            🔄 刷新
          </button>
          <span className="perf-page__last-update">
            更新于 {lastRefresh.toLocaleTimeString()}
          </span>
        </div>
      </header>

      {error && (
        <div className="perf-page__error">
          ⚠️ {error}（部分数据可能不可用）
        </div>
      )}

      {/* 系统概览 */}
      <section className="perf-section">
        <h2 className="perf-section__title">🖥️ 系统概览</h2>
        <div className="perf-section__grid">
          {health?.system && (
            <>
              <MetricCard
                icon="⏱️"
                label="运行时间"
                value={formatUptime(health.system.uptime_seconds)}
                color="blue"
              />
              <MetricCard
                icon="🔗"
                label="活跃连接"
                value={health.system.active_connections}
                color="purple"
              />
              <MetricCard
                icon="📈"
                label="请求速率"
                value={health.system.request_rate.toFixed(1)}
                unit="req/s"
                color="green"
              />
              <MetricCard
                icon="❌"
                label="错误率"
                value={(health.system.error_rate * 100).toFixed(2)}
                unit="%"
                color={health.system.error_rate > 0.05 ? 'red' : 'green'}
              />
            </>
          )}
        </div>
      </section>

      {/* 资源使用 */}
      <section className="perf-section">
        <h2 className="perf-section__title">📊 资源使用</h2>
        <div className="perf-section__grid perf-section__grid--wide">
          {health?.system && (
            <>
              <div className="resource-card">
                <div className="resource-card__header">
                  <span>CPU 使用率</span>
                  <span className="resource-card__value">
                    {health.system.cpu_usage_percent.toFixed(1)}%
                  </span>
                </div>
                <ProgressBar value={health.system.cpu_usage_percent} />
              </div>
              <div className="resource-card">
                <div className="resource-card__header">
                  <span>内存使用</span>
                  <span className="resource-card__value">
                    {formatBytes(health.system.memory_used_mb)} / {formatBytes(health.system.memory_total_mb)}
                  </span>
                </div>
                <ProgressBar value={health.system.memory_usage_percent} />
              </div>
            </>
          )}
        </div>
      </section>

      {/* 服务状态 */}
      {health?.services && Object.keys(health.services).length > 0 && (
        <section className="perf-section">
          <h2 className="perf-section__title">🏥 服务状态</h2>
          <div className="service-grid">
            {Object.values(health.services).map(service => (
              <div key={service.name} className={`service-card service-card--${statusColor(service.status)}`}>
                <div className="service-card__header">
                  <span className="service-card__name">{service.name}</span>
                  <span className={`service-card__status service-card__status--${statusColor(service.status)}`}>
                    {service.status}
                  </span>
                </div>
                <div className="service-card__latency">
                  延迟: {service.latency_ms}ms
                </div>
                {service.message && (
                  <div className="service-card__message">{service.message}</div>
                )}
              </div>
            ))}
          </div>
        </section>
      )}

      {/* API 请求统计 */}
      {requestStats && (
        <section className="perf-section">
          <h2 className="perf-section__title">🌐 API 请求统计</h2>
          <div className="perf-section__grid">
            <MetricCard
              icon="📊"
              label="总请求数"
              value={requestStats.total_requests.toLocaleString()}
              color="blue"
            />
            <MetricCard
              icon="✅"
              label="成功请求"
              value={requestStats.success_count.toLocaleString()}
              color="green"
            />
            <MetricCard
              icon="⏱️"
              label="平均响应"
              value={requestStats.avg_response_time_ms.toFixed(0)}
              unit="ms"
              color="purple"
            />
            <MetricCard
              icon="🚀"
              label="P95 响应"
              value={requestStats.p95_response_time_ms.toFixed(0)}
              unit="ms"
              color="orange"
            />
          </div>

          {/* 端点列表 */}
          {requestStats.endpoints.length > 0 && (
            <div className="endpoint-table-wrapper">
              <table className="endpoint-table">
                <thead>
                  <tr>
                    <th>端点</th>
                    <th>方法</th>
                    <th>请求数</th>
                    <th>平均耗时</th>
                    <th>错误数</th>
                  </tr>
                </thead>
                <tbody>
                  {requestStats.endpoints.slice(0, 20).map((ep, i) => (
                    <tr key={i}>
                      <td className="endpoint-table__path">{ep.path}</td>
                      <td>
                        <span className={`method-badge method-badge--${ep.method.toLowerCase()}`}>
                          {ep.method}
                        </span>
                      </td>
                      <td>{ep.count}</td>
                      <td>{ep.avg_time_ms.toFixed(1)}ms</td>
                      <td className={ep.error_count > 0 ? 'endpoint-table__error' : ''}>
                        {ep.error_count}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </section>
      )}

      {/* WebSocket 统计 */}
      {wsStats && (
        <section className="perf-section">
          <h2 className="perf-section__title">🔌 WebSocket 统计</h2>
          <div className="perf-section__grid">
            <MetricCard
              icon="🔗"
              label="总连接数"
              value={wsStats.total_connections}
              color="blue"
            />
            <MetricCard
              icon="🔐"
              label="已认证连接"
              value={wsStats.authenticated_connections}
              color="green"
            />
            <MetricCard
              icon="📤"
              label="发送消息"
              value={wsStats.messages_sent.toLocaleString()}
              color="purple"
            />
            <MetricCard
              icon="📥"
              label="接收消息"
              value={wsStats.messages_received.toLocaleString()}
              color="cyan"
            />
          </div>
        </section>
      )}

      {/* 前端性能 */}
      <section className="perf-section">
        <h2 className="perf-section__title">🎨 前端性能</h2>
        <div className="perf-section__grid">
          <MetricCard
            icon="🖥️"
            label="当前 FPS"
            value={fps}
            color={isLowFPS ? 'red' : 'green'}
            detail={isLowFPS ? '⚠️ 帧率偏低' : undefined}
          />
          <MetricCard
            icon="📊"
            label="平均 FPS"
            value={avgFPS}
            color={avgFPS < 30 ? 'red' : 'green'}
          />
          {memoryInfo && (
            <MetricCard
              icon="💾"
              label="JS 堆内存"
              value={`${(memoryInfo.usedJSHeapSize / 1024 / 1024).toFixed(1)}`}
              unit="MB"
              color={memoryInfo.usagePercent > 80 ? 'red' : 'blue'}
              detail={`使用率 ${memoryInfo.usagePercent}%`}
            />
          )}
          {pagePerformance && (
            <MetricCard
              icon="⚡"
              label="FCP"
              value={pagePerformance.fcp.toFixed(0)}
              unit="ms"
              color={pagePerformance.fcp > 2000 ? 'red' : 'green'}
            />
          )}
        </div>
        {pagePerformance && (
          <div className="perf-metrics-detail">
            <div className="perf-metrics-detail__item">
              <span>DNS 查询</span>
              <span>{pagePerformance.dnsTime.toFixed(1)}ms</span>
            </div>
            <div className="perf-metrics-detail__item">
              <span>TCP 连接</span>
              <span>{pagePerformance.tcpTime.toFixed(1)}ms</span>
            </div>
            <div className="perf-metrics-detail__item">
              <span>请求响应</span>
              <span>{pagePerformance.requestTime.toFixed(1)}ms</span>
            </div>
            <div className="perf-metrics-detail__item">
              <span>DOM 解析</span>
              <span>{pagePerformance.domParseTime.toFixed(1)}ms</span>
            </div>
            <div className="perf-metrics-detail__item">
              <span>DCL</span>
              <span>{pagePerformance.domContentLoaded.toFixed(1)}ms</span>
            </div>
            <div className="perf-metrics-detail__item">
              <span>页面加载</span>
              <span>{pagePerformance.loadComplete.toFixed(1)}ms</span>
            </div>
            {pagePerformance.lcp !== null && (
              <div className="perf-metrics-detail__item">
                <span>LCP</span>
                <span>{pagePerformance.lcp.toFixed(0)}ms</span>
              </div>
            )}
            {pagePerformance.cls !== null && (
              <div className="perf-metrics-detail__item">
                <span>CLS</span>
                <span>{pagePerformance.cls.toFixed(4)}</span>
              </div>
            )}
          </div>
        )}
      </section>
    </div>
  )
}
