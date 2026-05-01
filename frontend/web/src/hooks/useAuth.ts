import { useState, useEffect } from 'react'
import type { User } from '../types/user'
import { userService } from '../services/userService'
import { mockApi } from '../services/mockApi'

const USE_MOCK_DATA = import.meta.env.VITE_USE_MOCK_DATA === 'true'

export function useAuth() {
  const [user, setUser] = useState<User | null>(null)
  const [loading, setLoading] = useState(true)
  const [isAuthenticated, setIsAuthenticated] = useState(false)

  // 检查认证状态
  useEffect(() => {
    checkAuth()
  }, [])

  const checkAuth = async () => {
    if (USE_MOCK_DATA) {
      // 模拟模式：自动登录
      setUser(mockUser)
      setIsAuthenticated(true)
      setLoading(false)
      return
    }

    const token = localStorage.getItem('token')
    if (!token) {
      setLoading(false)
      return
    }

    try {
      const response = await userService.getCurrentUser()
      if (response.success && response.data) {
        setUser(response.data)
        setIsAuthenticated(true)
      } else {
        localStorage.removeItem('token')
      }
    } catch (error) {
      console.error('Auth check failed:', error)
      localStorage.removeItem('token')
    } finally {
      setLoading(false)
    }
  }

  const login = async (email: string, password: string) => {
    if (USE_MOCK_DATA) {
      const response = await mockApi.login(email, password)
      if (response.success && response.data) {
        localStorage.setItem('token', response.data.token)
        setUser(response.data.user)
        setIsAuthenticated(true)
        return { success: true }
      }
      return { success: false, error: '登录失败' }
    }

    const response = await userService.login({ email, password })
    if (response.success && response.data) {
      localStorage.setItem('token', response.data.token)
      setUser(response.data.user)
      setIsAuthenticated(true)
      return { success: true }
    }
    return { success: false, error: response.error?.message || '登录失败' }
  }

  const register = async (username: string, email: string, password: string) => {
    if (USE_MOCK_DATA) {
      const response = await mockApi.register(username, email, password)
      if (response.success && response.data) {
        return { success: true }
      }
      return { success: false, error: '注册失败' }
    }

    const response = await userService.register({ username, email, password })
    if (response.success && response.data) {
      return { success: true }
    }
    return { success: false, error: response.error?.message || '注册失败' }
  }

  const logout = () => {
    localStorage.removeItem('token')
    setUser(null)
    setIsAuthenticated(false)
  }

  return {
    user,
    loading,
    isAuthenticated,
    login,
    register,
    logout,
    checkAuth,
    useMockData: USE_MOCK_DATA,
  }
}
