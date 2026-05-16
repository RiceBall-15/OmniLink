import { Component, type ErrorInfo, type ReactNode } from 'react'
import './ErrorBoundary.css'

/**
 * 错误边界属性
 */
interface ErrorBoundaryProps {
  /** 子组件 */
  children: ReactNode
  /** 自定义回退 UI */
  fallback?: ReactNode
  /** 错误上报回调 */
  onError?: (error: Error, errorInfo: ErrorInfo) => void
  /** 组件名称（用于错误标识） */
  name?: string
}

/**
 * 错误边界状态
 */
interface ErrorBoundaryState {
  /** 是否有错误 */
  hasError: boolean
  /** 错误对象 */
  error: Error | null
  /** 错误信息 */
  errorInfo: ErrorInfo | null
  /** 错误ID（用于上报） */
  errorId: string | null
}

/**
 * 全局错误边界组件
 * 捕获子组件树中的 JavaScript 错误，显示优雅的回退 UI
 */
export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props)
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null,
      errorId: null,
    }
  }

  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    return {
      hasError: true,
      error,
      errorId: `err_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
    }
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    this.setState({ errorInfo })

    // 错误上报
    if (this.props.onError) {
      this.props.onError(error, errorInfo)
    }

    // 控制台输出
    console.error(`[ErrorBoundary${this.props.name ? `:${this.props.name}` : ''}]`, error)
    console.error('Component stack:', errorInfo.componentStack)

    // 存储到 localStorage 用于调试
    try {
      const errorLog = {
        id: this.state.errorId,
        name: this.props.name,
        message: error.message,
        stack: error.stack,
        componentStack: errorInfo.componentStack,
        timestamp: new Date().toISOString(),
        url: window.location.href,
      }

      const logs = JSON.parse(localStorage.getItem('error_logs') || '[]')
      logs.push(errorLog)
      // 只保留最近 20 条
      if (logs.length > 20) logs.splice(0, logs.length - 20)
      localStorage.setItem('error_logs', JSON.stringify(logs))
    } catch {
      // localStorage 可能不可用
    }
  }

  handleRetry = () => {
    this.setState({
      hasError: false,
      error: null,
      errorInfo: null,
      errorId: null,
    })
  }

  handleReload = () => {
    window.location.reload()
  }

  handleCopyError = () => {
    const { error, errorInfo, errorId } = this.state
    const text = [
      `Error ID: ${errorId}`,
      `Component: ${this.props.name || 'Unknown'}`,
      `Time: ${new Date().toISOString()}`,
      `URL: ${window.location.href}`,
      '',
      `Error: ${error?.message}`,
      '',
      `Stack:`,
      error?.stack,
      '',
      `Component Stack:`,
      errorInfo?.componentStack,
    ].join('\n')

    navigator.clipboard.writeText(text).then(() => {
      alert('错误信息已复制到剪贴板')
    })
  }

  render() {
    if (this.state.hasError) {
      // 自定义回退 UI
      if (this.props.fallback) {
        return this.props.fallback
      }

      // 默认回退 UI
      return (
        <div className="error-boundary">
          <div className="error-boundary__container">
            <div className="error-boundary__icon">⚠️</div>
            <h2 className="error-boundary__title">页面出现了问题</h2>
            <p className="error-boundary__message">
              {this.state.error?.message || '发生了未知错误'}
            </p>

            {this.state.errorId && (
              <p className="error-boundary__error-id">
                错误ID: <code>{this.state.errorId}</code>
              </p>
            )}

            <div className="error-boundary__actions">
              <button
                className="error-boundary__btn error-boundary__btn--primary"
                onClick={this.handleRetry}
              >
                重试
              </button>
              <button
                className="error-boundary__btn error-boundary__btn--secondary"
                onClick={this.handleReload}
              >
                刷新页面
              </button>
              <button
                className="error-boundary__btn error-boundary__btn--ghost"
                onClick={this.handleCopyError}
              >
                复制错误信息
              </button>
            </div>

            <details className="error-boundary__details">
              <summary>技术详情</summary>
              <pre className="error-boundary__stack">
                {this.state.error?.stack}
              </pre>
              {this.state.errorInfo?.componentStack && (
                <>
                  <h4>组件堆栈:</h4>
                  <pre className="error-boundary__stack">
                    {this.state.errorInfo.componentStack}
                  </pre>
                </>
              )}
            </details>
          </div>
        </div>
      )
    }

    return this.props.children
  }
}

/**
 * 获取存储的错误日志
 */
export function getErrorLogs(): Array<{
  id: string
  name?: string
  message: string
  stack?: string
  componentStack?: string
  timestamp: string
  url: string
}> {
  try {
    return JSON.parse(localStorage.getItem('error_logs') || '[]')
  } catch {
    return []
  }
}

/**
 * 清除错误日志
 */
export function clearErrorLogs(): void {
  localStorage.removeItem('error_logs')
}
