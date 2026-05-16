import { useNetworkStatus, formatOfflineDuration } from '../hooks/useNetworkStatus'
import './OfflineBanner.css'

/**
 * 离线状态横幅组件
 * 当用户网络断开时显示提示横幅
 */
export function OfflineBanner() {
  const { isOnline, offlineDuration, quality, connectionType } = useNetworkStatus()

  if (isOnline && quality !== 'poor') {
    return null
  }

  if (!isOnline) {
    return (
      <div className="offline-banner offline-banner--offline">
        <div className="offline-banner__content">
          <span className="offline-banner__icon">📡</span>
          <span className="offline-banner__text">
            网络已断开 · 离线 {formatOfflineDuration(offlineDuration)}
          </span>
        </div>
        <div className="offline-banner__subtext">
          恢复网络后将自动同步消息
        </div>
      </div>
    )
  }

  // 弱网提示
  if (quality === 'poor') {
    return (
      <div className="offline-banner offline-banner--poor">
        <div className="offline-banner__content">
          <span className="offline-banner__icon">🐌</span>
          <span className="offline-banner__text">
            网络较慢 {connectionType ? `(${connectionType.toUpperCase()})` : ''}
          </span>
        </div>
      </div>
    )
  }

  return null
}
