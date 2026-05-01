// 用户类型定义
export interface User {
  id: string
  username: string
  email: string
  avatar?: string
  createdAt: string
  updatedAt: string
}

// 用户注册请求
export interface RegisterRequest {
  username: string
  email: string
  password: string
}

// 用户登录请求
export interface LoginRequest {
  email: string
  password: string
}

// 登录响应
export interface LoginResponse {
  token: string
  user: User
}

// 设备信息
export interface Device {
  id: string
  userId: string
  deviceType: 'web' | 'mobile' | 'desktop'
  deviceName: string
  platform: string
  lastActiveAt: string
  createdAt: string
}
