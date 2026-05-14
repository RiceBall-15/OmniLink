import { api, ApiResponse } from './api'

// 管理员仪表板数据类型
export interface DashboardStats {
  totalUsers: number
  onlineUsers: number
  totalMessages: number
  totalConversations: number
  totalFiles: number
  messagesTrend: { date: string; count: number }[]
  usersTrend: { date: string; count: number }[]
}

// 系统公告类型
export interface Announcement {
  id: string
  title: string
  content: string
  type: 'info' | 'warning' | 'maintenance'
  priority: number
  created_by: string
  expires_at?: string
  created_at: string
  updated_at: string
}

export interface CreateAnnouncementRequest {
  title: string
  content: string
  type: 'info' | 'warning' | 'maintenance'
  priority: number
  expires_at?: string
}

// 用户反馈类型
export interface Feedback {
  id: string
  user_id: string
  type: 'bug' | 'feature' | 'other'
  content: string
  status: 'pending' | 'in_progress' | 'resolved' | 'rejected'
  priority: number
  admin_reply?: string
  created_at: string
  updated_at: string
}

export interface FeedbackStats {
  total: number
  pending: number
  in_progress: number
  resolved: number
  rejected: number
  by_type: { type: string; count: number }[]
}

// 用户管理类型
export interface UserListItem {
  id: string
  email: string
  nickname?: string
  role: string
  status: string
  created_at: string
  last_login?: string
}

// 限流配置
export interface RateLimitConfig {
  enabled: boolean
  default_limit: number
  default_window_seconds: number
  per_user_limit?: number
  per_ip_limit?: number
  per_path_limits?: Record<string, number>
}

// 日志级别
export interface LogLevelConfig {
  level: string
  modules?: Record<string, string>
}

// 服务健康状态
export interface HealthStatus {
  status: string
  version: string
  uptime_seconds: number
  dependencies: {
    database: { status: string; latency_ms?: number }
    redis: { status: string; latency_ms?: number }
  }
}

// 业务指标
export interface BusinessMetrics {
  total_requests: number
  total_errors: number
  total_messages_sent: number
  total_messages_received: number
  total_conversations_created: number
  total_users_registered: number
  total_ws_connections: number
  total_auth_failures: number
  uptime_seconds: number
}

// 管理员 API 服务
export const adminService = {
  // ===== 仪表板统计 =====
  getDashboardStats: async (): Promise<ApiResponse<DashboardStats>> => {
    return api.get<DashboardStats>('/api/admin/dashboard/stats')
  },

  // ===== 系统公告 =====
  getAnnouncements: async (): Promise<ApiResponse<Announcement[]>> => {
    return api.get<Announcement[]>('/api/admin/announcements')
  },

  createAnnouncement: async (data: CreateAnnouncementRequest): Promise<ApiResponse<Announcement>> => {
    return api.post<Announcement>('/api/admin/announcements', data)
  },

  updateAnnouncement: async (id: string, data: Partial<CreateAnnouncementRequest>): Promise<ApiResponse<Announcement>> => {
    return api.put<Announcement>(`/api/admin/announcements/${id}`, data)
  },

  deleteAnnouncement: async (id: string): Promise<ApiResponse<void>> => {
    return api.delete<void>(`/api/admin/announcements/${id}`)
  },

  // ===== 用户反馈 =====
  getFeedbacks: async (params?: { status?: string; type?: string }): Promise<ApiResponse<Feedback[]>> => {
    const query = new URLSearchParams()
    if (params?.status) query.set('status', params.status)
    if (params?.type) query.set('type', params.type)
    const qs = query.toString()
    return api.get<Feedback[]>(`/api/admin/feedbacks${qs ? '?' + qs : ''}`)
  },

  getFeedbackStats: async (): Promise<ApiResponse<FeedbackStats>> => {
    return api.get<FeedbackStats>('/api/admin/feedbacks/stats')
  },

  updateFeedback: async (id: string, data: { status?: string; admin_reply?: string }): Promise<ApiResponse<Feedback>> => {
    return api.put<Feedback>(`/api/admin/feedbacks/${id}`, data)
  },

  deleteFeedback: async (id: string): Promise<ApiResponse<void>> => {
    return api.delete<void>(`/api/admin/feedbacks/${id}`)
  },

  // ===== 限流配置 =====
  getRateLimitConfig: async (): Promise<ApiResponse<RateLimitConfig>> => {
    return api.get<RateLimitConfig>('/api/admin/rate-limit')
  },

  updateRateLimitConfig: async (data: RateLimitConfig): Promise<ApiResponse<RateLimitConfig>> => {
    return api.put<RateLimitConfig>('/api/admin/rate-limit', data)
  },

  // ===== 日志级别 =====
  getLogLevel: async (): Promise<ApiResponse<LogLevelConfig>> => {
    return api.get<LogLevelConfig>('/api/admin/log-level')
  },

  updateLogLevel: async (data: LogLevelConfig): Promise<ApiResponse<LogLevelConfig>> => {
    return api.put<LogLevelConfig>('/api/admin/log-level', data)
  },

  // ===== 健康检查 =====
  getHealth: async (): Promise<ApiResponse<HealthStatus>> => {
    return api.get<HealthStatus>('/health')
  },

  // ===== 业务指标 =====
  getMetrics: async (): Promise<ApiResponse<BusinessMetrics>> => {
    return api.get<BusinessMetrics>('/api/metrics')
  },
}

export default adminService
