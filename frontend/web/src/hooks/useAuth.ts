import { useState, useEffect } from 'react'
import type { User } from '../types/user'
import { userService } from '../services/userService'

export function useAuth() {
  const [user, setUser] = useState<User | null>(null)
  const [loading, setLoading] = useState(true)
  const [isAuthenticated, setIsAuthenticated] = useState(false)

  // 检查认证状态
  useEffect(() => {
    checkAuth()
  }, [])

  const checkAuth = async () => {
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
  }
}
