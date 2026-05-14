import React from 'react'
import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom'
import { useAuth } from './hooks/useAuth'
import { createRouteComponent } from './utils/codeSplitting'

// 懒加载页面组件
const AuthPage = createRouteComponent(
  () => import('./pages/AuthPage'),
  { preload: true }
)

const ChatPage = createRouteComponent(
  () => import('./pages/ChatPage'),
  { preload: true }
)

const SettingsPage = createRouteComponent(
  () => import('./pages/SettingsPage'),
  { preload: false }
)

// 路由守卫组件
function ProtectedRoute({ children }: { children: React.ReactNode }) {
  const { isAuthenticated, loading } = useAuth()

  if (loading) {
    return (
      <div className="loading-screen">
        <div className="loading-spinner"></div>
        <p>加载中...</p>
      </div>
    )
  }

  if (!isAuthenticated) {
    return <Navigate to="/auth" replace />
  }

  return <>{children}</>
}

// 公共路由组件
function PublicRoute({ children }: { children: React.ReactNode }) {
  const { isAuthenticated, loading } = useAuth()

  if (loading) {
    return (
      <div className="loading-screen">
        <div className="loading-spinner"></div>
        <p>加载中...</p>
      </div>
    )
  }

  if (isAuthenticated) {
    return <Navigate to="/chat" replace />
  }

  return <>{children}</>
}

function App() {
  return (
    <Router>
      <Routes>
        {/* 公共路由 */}
        <Route
          path="/auth"
          element={
            <PublicRoute>
              <AuthPage />
            </PublicRoute>
          }
        />

        {/* 受保护的路由 */}
        <Route
          path="/chat"
          element={
            <ProtectedRoute>
              <ChatPage />
            </ProtectedRoute>
          }
        />

        <Route
          path="/settings"
          element={
            <ProtectedRoute>
              <SettingsPage />
            </ProtectedRoute>
          }
        />

        {/* 默认重定向 */}
        <Route path="/" element={<Navigate to="/chat" replace />} />

        {/* 404 页面 */}
        <Route
          path="*"
          element={
            <div className="not-found">
              <h1>404</h1>
              <p>页面不存在</p>
              <a href="/chat">返回聊天</a>
            </div>
          }
        />
      </Routes>
    </Router>
  )
}

export default App
