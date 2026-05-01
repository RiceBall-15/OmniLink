import request, { ApiResponse } from './api'
import type {
  User,
  RegisterRequest,
  LoginRequest,
  LoginResponse,
  Device,
} from '../types/user'

// 用户服务
export const userService = {
  // 用户注册
  register: async (data: RegisterRequest): Promise<ApiResponse<User>> => {
    return request<User>('/api/auth/register', {
      method: 'POST',
      body: JSON.stringify(data),
    })
  },

  // 用户登录
  login: async (data: LoginRequest): Promise<ApiResponse<LoginResponse>> => {
    return request<LoginResponse>('/api/auth/login', {
      method: 'POST',
      body: JSON.stringify(data),
    })
  },

  // 获取当前用户信息
  getCurrentUser: async (): Promise<ApiResponse<User>> => {
    return request<User>('/api/user/me')
  },

  // 更新用户资料
  updateProfile: async (data: Partial<User>): Promise<ApiResponse<User>> => {
    return request<User>('/api/user/me', {
      method: 'PUT',
      body: JSON.stringify(data),
    })
  },

  // 获取设备列表
  getDevices: async (): Promise<ApiResponse<Device[]>> => {
    return request<Device[]>('/api/user/devices')
  },

  // 注册设备
  registerDevice: async (data: {
    deviceType: 'web' | 'mobile' | 'desktop'
    deviceName: string
    platform: string
  }): Promise<ApiResponse<Device>> => {
    return request<Device>('/api/user/devices', {
      method: 'POST',
      body: JSON.stringify(data),
    })
  },

  // 删除设备
  deleteDevice: async (deviceId: string): Promise<ApiResponse<void>> => {
    return request<void>(`/api/user/devices/${deviceId}`, {
      method: 'DELETE',
    })
  },
}