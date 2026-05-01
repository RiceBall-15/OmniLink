import { useState } from 'react'
import { useAuth } from '../hooks/useAuth'
import './AuthPage.css'

export function AuthPage() {
  const { login, register, isAuthenticated } = useAuth()
  const [mode, setMode] = useState<'login' | 'register'>('login')
  const [loading, setLoading] = useState(false)
  const [errors, setErrors] = useState<Record<string, string>>({})
  const [formData, setFormData] = useState({
    username: '',
    email: '',
    password: '',
    confirmPassword: '',
  })

  const validateForm = () => {
    const newErrors: Record<string, string> = {}

    if (mode === 'register' && !formData.username.trim()) {
      newErrors.username = '用户名不能为空'
    }

    if (!formData.email.trim()) {
      newErrors.email = '邮箱不能为空'
    } else if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(formData.email)) {
      newErrors.email = '请输入有效的邮箱地址'
    }

    if (!formData.password) {
      newErrors.password = '密码不能为空'
    } else if (formData.password.length < 6) {
      newErrors.password = '密码至少6个字符'
    }

    if (mode === 'register' && formData.password !== formData.confirmPassword) {
      newErrors.confirmPassword = '两次输入的密码不一致'
    }

    setErrors(newErrors)
    return Object.keys(newErrors).length === 0
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()

    if (!validateForm()) {
      return
    }

    setLoading(true)
    setErrors({})

    try {
      if (mode === 'login') {
        const result = await login(formData.email, formData.password)
        if (!result.success) {
          setErrors({ general: result.error || '登录失败，请检查邮箱和密码' })
        }
      } else {
        const result = await register(formData.username, formData.email, formData.password)
        if (!result.success) {
          setErrors({ general: result.error || '注册失败，请稍后重试' })
        } else {
          // 注册成功后自动登录
          const loginResult = await login(formData.email, formData.password)
          if (!loginResult.success) {
            setErrors({ general: '注册成功但登录失败，请手动登录' })
            setMode('login')
          }
        }
      }
    } catch (error) {
      setErrors({ general: '网络错误，请检查连接后重试' })
    } finally {
      setLoading(false)
    }
  }

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const { name, value } = e.target
    setFormData((prev) => ({ ...prev, [name]: value }))
    // 清除对应字段的错误
    if (errors[name]) {
      setErrors((prev) => {
        const newErrors = { ...prev }
        delete newErrors[name]
        return newErrors
      })
    }
  }

  const switchMode = () => {
    setMode(mode === 'login' ? 'register' : 'login')
    setErrors({})
    setFormData({
      username: '',
      email: '',
      password: '',
      confirmPassword: '',
    })
  }

  if (isAuthenticated) {
    return null
  }

  return (
    <div className="auth-container">
      <div className="auth-card">
        <div className="auth-header">
          <div className="auth-logo">🤖</div>
          <h1 className="auth-title">OmniLink</h1>
          <p className="auth-subtitle">智能对话，连接未来</p>
        </div>

        <div className="auth-tabs">
          <div
            className={`auth-tab ${mode === 'login' ? 'active' : ''}`}
            onClick={() => mode === 'register' && switchMode()}
          >
            登录
          </div>
          <div
            className={`auth-tab ${mode === 'register' ? 'active' : ''}`}
            onClick={() => mode === 'login' && switchMode()}
          >
            注册
          </div>
        </div>

        {errors.general && (
          <div className="form-error" style={{ marginBottom: '1rem', justifyContent: 'center' }}>
            ⚠️ {errors.general}
          </div>
        )}

        <form className="auth-form" onSubmit={handleSubmit}>
          {mode === 'register' && (
            <div className="form-group">
              <label className="form-label">用户名</label>
              <input
                type="text"
                name="username"
                value={formData.username}
                onChange={handleInputChange}
                className={`form-input ${errors.username ? 'error' : ''}`}
                placeholder="请输入用户名"
                disabled={loading}
              />
              {errors.username && <div className="form-error">⚠️ {errors.username}</div>}
            </div>
          )}

          <div className="form-group">
            <label className="form-label">📧 邮箱</label>
            <input
              type="email"
              name="email"
              value={formData.email}
              onChange={handleInputChange}
              className={`form-input ${errors.email ? 'error' : ''}`}
              placeholder="请输入邮箱地址"
              disabled={loading}
            />
            {errors.email && <div className="form-error">⚠️ {errors.email}</div>}
          </div>

          <div className="form-group">
            <label className="form-label">🔒 密码</label>
            <input
              type="password"
              name="password"
              value={formData.password}
              onChange={handleInputChange}
              className={`form-input ${errors.password ? 'error' : ''}`}
              placeholder="请输入密码（至少6位）"
              disabled={loading}
            />
            {errors.password && <div className="form-error">⚠️ {errors.password}</div>}
          </div>

          {mode === 'register' && (
            <div className="form-group">
              <label className="form-label">确认密码</label>
              <input
                type="password"
                name="confirmPassword"
                value={formData.confirmPassword}
                onChange={handleInputChange}
                className={`form-input ${errors.confirmPassword ? 'error' : ''}`}
                placeholder="请再次输入密码"
                disabled={loading}
              />
              {errors.confirmPassword && <div className="form-error">⚠️ {errors.confirmPassword}</div>}
            </div>
          )}

          {mode === 'login' && (
            <div className="auth-remember">
              <input type="checkbox" id="remember" />
              <label htmlFor="remember">记住我</label>
            </div>
          )}

          <button type="submit" className={`auth-button ${loading ? 'loading' : ''}`} disabled={loading}>
            {loading ? (
              '处理中...'
            ) : mode === 'login' ? (
              <>
                <span>🚀</span>
                <span>立即登录</span>
              </>
            ) : (
              <>
                <span>✨</span>
                <span>创建账户</span>
              </>
            )}
          </button>
        </form>

        <div className="auth-divider">
          <span>或</span>
        </div>

        <div className="social-login">
          <button type="button" className="social-button" disabled={loading}>
            <span className="social-icon">🔑</span>
            <span>使用 Google 账号登录</span>
          </button>
          <button type="button" className="social-button" disabled={loading}>
            <span className="social-icon">💼</span>
            <span>使用 GitHub 账号登录</span>
          </button>
        </div>

        <div className="auth-footer">
          {mode === 'login' ? (
            <>
              还没有账号？{' '}
              <button className="auth-link" onClick={switchMode} disabled={loading}>
                立即注册
              </button>
            </>
          ) : (
            <>
              已有账号？{' '}
              <button className="auth-link" onClick={switchMode} disabled={loading}>
                立即登录
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  )
}
