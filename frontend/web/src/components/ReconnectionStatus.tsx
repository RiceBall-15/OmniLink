import { useEffect, useState } from 'react'
import './ReconnectionStatus.css'

/**
 * 重连状态组件属性
 */
interface ReconnectionStatusProps {
  /** 连接状态 */
  status: 'connecting' | 'connected' | 'disconnected' | 'error' | 'reconnecting'
  /** 当前重连尝试次数 */
  reconnectAttempt: number
  /** 下次重连延迟（毫秒） */
  nextReconnectDelay: number | null
  /** 是否在线（浏览器网络状态） */
  isOnline: boolean
  /** 手动重连回调 */
  onReconnect?: () => void
}

/**
 * 重连状态指示器组件
 * 显示 WebSocket 连接状态、重连进度和网络状态
 */
export function ReconnectionStatus({
  status,
  reconnectAttempt,
  nextReconnectDelay,
  isOnline,
  onReconnect,
}: ReconnectionStatusProps) {
  // 倒计时显示
  const [countdown, setCountdown] = useState<number | null>(null)

  useEffect(() => {
    if (nextReconnectDelay === null) {
      setCountdown(null)
      return
    }

    const startTime = Date.now()
    const endTime = startTime + nextReconnectDelay

    const timer = setInterval(() => {
      const remaining = Math.max(0, endTime - Date.now())
      setCountdown(Math.ceil(remaining / 1000))

      if (remaining <= 0) {
        clearInterval(timer)
      }
    }, 100)

    return () => clearInterval(timer)
  }, [nextReconnectDelay])

  // 离线状态
  if (!isOnline) {
    return (
      <div className="reconnection-status reconnection-status--offline">
        <div className="reconnection-status__icon">📡</div>
        <div className="reconnection-status__content">
          <div className="reconnection-status__title">网络已断开</div>
          <div className="reconnection-status__message">
            请检查您的网络连接，恢复后将自动重连
          </div>
        </div>
      </div>
    )
  }

  // 连接中
  if (status === 'connecting') {
    return (
      <div className="reconnection-status reconnection-status--connecting">
        <div className="reconnection-status__spinner" />
        <div className="reconnection-status__content">
          <div className="reconnection-status__title">正在连接...</div>
          <div className="reconnection-status__message">
            正在建立实时通信连接
          </div>
        </div>
      </div>
    )
  }

  // 重连中
  if (status === 'reconnecting') {
    return (
      <div className="reconnection-status reconnection-status--reconnecting">
        <div className="reconnection-status__spinner" />
        <div className="reconnection-status__content">
          <div className="reconnection-status__title">
            连接已断开，正在重连 ({reconnectAttempt})
          </div>
          <div className="reconnection-status__message">
            {countdown !== null && countdown > 0
              ? `${countdown} 秒后尝试重新连接...`
              : '正在重新连接...'}
          </div>
          <div className="reconnection-status__progress">
            <div
              className="reconnection-status__progress-bar"
              style={{
                width: `${Math.min((reconnectAttempt / 15) * 100, 100)}%`,
              }}
            />
          </div>
        </div>
        {onReconnect && (
          <button
            className="reconnection-status__action"
            onClick={onReconnect}
          >
            立即重连
          </button>
        )}
      </div>
    )
  }

  // 错误状态
  if (status === 'error') {
    return (
      <div className="reconnection-status reconnection-status--error">
        <div className="reconnection-status__icon">⚠️</div>
        <div className="reconnection-status__content">
          <div className="reconnection-status__title">连接错误</div>
          <div className="reconnection-status__message">
            无法建立实时通信连接
          </div>
        </div>
        {onReconnect && (
          <button
            className="reconnection-status__action"
            onClick={onReconnect}
          >
            重试
          </button>
        )}
      </div>
    )
  }

  // 已连接状态不显示
  return null
}
