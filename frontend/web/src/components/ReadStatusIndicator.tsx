import type { MessageStatus } from '../types/message'

/**
 * 消息状态指示器属性
 */
interface ReadStatusIndicatorProps {
  /** 消息状态 */
  status: MessageStatus
  /** 点击回调（可选） */
  onClick?: () => void
}

/**
 * 消息状态指示器组件
 * 显示四种状态：sending（灰色✓）、sent（灰色✓）、delivered（灰色✓✓）、read（蓝色✓✓）
 */
export function ReadStatusIndicator({ status, onClick }: ReadStatusIndicatorProps) {
  // 定义图标样式
  const getIcon = () => {
    switch (status) {
      case 'sending':
        return <span style={{ color: '#9ca3af', fontSize: '12px' }}>✓</span>
      case 'sent':
        return <span style={{ color: '#9ca3af', fontSize: '12px' }}>✓</span>
      case 'delivered':
        return <span style={{ color: '#9ca3af', fontSize: '12px' }}>✓✓</span>
      case 'read':
        return <span style={{ color: '#3b82f6', fontSize: '12px' }}>✓✓</span>
      case 'failed':
        return <span style={{ color: '#ef4444', fontSize: '12px' }}>✗</span>
      default:
        return null
    }
  }

  return (
    <span
      className="read-status-indicator"
      style={{ marginLeft: '4px', display: 'inline-flex', alignItems: 'center', cursor: onClick ? 'pointer' : 'default' }}
      onClick={onClick}
    >
      {getIcon()}
    </span>
  )
}
