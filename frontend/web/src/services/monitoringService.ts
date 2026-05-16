import { api } from './api'

/**
 * 系统健康状态（后端 /health 端点）
 */
export interface HealthCheckResponse {
  status: string
  version: string
  timestamp: string
  dependencies: {
    database: DependencyStatus
    redis: DependencyStatus
  }
}

/**
 * 依赖状态
 */
export interface DependencyStatus {
  status: string
  response_time_ms: number
  error?: string
}

/**
 * 应用指标（后端 /metrics 端点）
 */
export interface MetricsResponse {
  uptime_seconds: number
  timestamp: string
  database: {
    pool_size: number
    idle_connections: number
  }
  business: {
    total_requests: number
    total_errors: number
    error_rate_percent: number
    messages_sent: number
    messages_received: number
    conversations_created: number
    users_registered: number
    ws_connections: number
    auth_failures: number
  }
}

/**
 * 聚合健康状态（后端 /api/health/status 端点）
 */
export interface AggregatedHealthResponse {
  status: string
  services: Array<{
    name: string
    healthy: boolean
    latency_ms: number
    error?: string
  }>
  total_latency_ms: number
  timestamp: number
  version: string
}

/**
 * 适配后的系统健康数据（前端展示用）
 */
export interface SystemHealth {
  status: string
  timestamp: string
  services: Record<string, {
    name: string
    status: string
    latency_ms: number
    message?: string
  }>
  system: {
    cpu_usage_percent: number
    memory_usage_percent: number
    memory_used_mb: number
    memory_total_mb: number
    uptime_seconds: number
    active_connections: number
    request_rate: number
    error_rate: number
  }
}

/**
 * 获取应用指标
 */
export async function getMetrics(): Promise<MetricsResponse> {
  return api.get<MetricsResponse>('/metrics')
}

/**
 * 获取聚合健康状态
 */
export async function getAggregatedHealth(): Promise<AggregatedHealthResponse> {
  return api.get<AggregatedHealthResponse>('/api/health/status')
}

/**
 * 获取系统健康状态（整合多个端点数据）
 */
export async function getSystemHealth(): Promise<SystemHealth> {
  const [metrics, health] = await Promise.allSettled([
    getMetrics(),
    getAggregatedHealth(),
  ])

  const metricsData = metrics.status === 'fulfilled' ? metrics.value : null
  const healthData = health.status === 'fulfilled' ? health.value : null

  // 适配为前端展示格式
  const services: SystemHealth['services'] = {}

  if (healthData) {
    for (const svc of healthData.services) {
      services[svc.name] = {
        name: svc.name,
        status: svc.healthy ? 'healthy' : 'unhealthy',
        latency_ms: svc.latency_ms,
        message: svc.error,
      }
    }
  }

  return {
    status: healthData?.status || 'unknown',
    timestamp: metricsData?.timestamp || new Date().toISOString(),
    services,
    system: {
      cpu_usage_percent: 0,
      memory_usage_percent: 0,
      memory_used_mb: 0,
      memory_total_mb: 0,
      uptime_seconds: metricsData?.uptime_seconds || 0,
      active_connections: metricsData?.business.ws_connections || 0,
      request_rate: 0,
      error_rate: (metricsData?.business.error_rate_percent || 0) / 100,
    },
  }
}

/**
 * 获取 API 请求统计（适配前端展示）
 */
export interface RequestStats {
  total_requests: number
  success_count: number
  error_count: number
  avg_response_time_ms: number
  p95_response_time_ms: number
  p99_response_time_ms: number
  endpoints: Array<{
    path: string
    method: string
    count: number
    avg_time_ms: number
    error_count: number
  }>
}

export type TimeRange = '1h' | '6h' | '24h' | '7d' | '30d'

/**
 * 获取请求统计（从 /metrics 适配）
 */
export async function getRequestStats(_range: TimeRange = '1h'): Promise<RequestStats> {
  const metrics = await getMetrics()

  return {
    total_requests: metrics.business.total_requests,
    success_count: metrics.business.total_requests - metrics.business.total_errors,
    error_count: metrics.business.total_errors,
    avg_response_time_ms: 0,
    p95_response_time_ms: 0,
    p99_response_time_ms: 0,
    endpoints: [],
  }
}

/**
 * WebSocket 统计（适配前端展示）
 */
export interface WebSocketStats {
  total_connections: number
  authenticated_connections: number
  messages_sent: number
  messages_received: number
  errors: number
  avg_latency_ms: number
}

/**
 * 获取 WebSocket 统计
 */
export async function getWebSocketStats(): Promise<WebSocketStats> {
  const metrics = await getMetrics()

  return {
    total_connections: metrics.business.ws_connections,
    authenticated_connections: metrics.business.ws_connections,
    messages_sent: metrics.business.messages_sent,
    messages_received: metrics.business.messages_received,
    errors: metrics.business.auth_failures,
    avg_latency_ms: 0,
  }
}
