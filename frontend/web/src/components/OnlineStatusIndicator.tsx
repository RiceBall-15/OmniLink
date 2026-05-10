import React from 'react'
import { OnlineStatus } from '../types/message'

interface OnlineStatusIndicatorProps {
  status: OnlineStatus
  size?: number
  showLabel?: boolean
}

const STATUS_COLORS = {
  [OnlineStatus.ONLINE]: '#22c55e', // 绿色
  [OnlineStatus.OFFLINE]: '#9ca3af', // 灰色
  [OnlineStatus.AWAY]: '#eab308',    // 黄色
  [OnlineStatus.BUSY]: '#ef4444',    // 红色
}

const STATUS_LABELS = {
  [OnlineStatus.ONLINE]: '在线',
  [OnlineStatus.OFFLINE]: '离线',
  [OnlineStatus.AWAY]: '离开',
  [OnlineStatus.BUSY]: '忙碌',
}

export const OnlineStatusIndicator: React.FC<OnlineStatusIndicatorProps> = ({
  status,
  size = 8,
  showLabel = false,
}) => {
  return (
    <div className="flex items-center gap-1.5">
      <div
        style={{
          width: `${size}px`,
          height: `${size}px`,
          borderRadius: '50%',
          backgroundColor: STATUS_COLORS[status],
          animation: 'fadeIn 0.3s ease-in-out',
        }}
        title={STATUS_LABELS[status]}
      />
      {showLabel && (
        <span className="text-xs text-gray-500">{STATUS_LABELS[status]}</span>
      )}
    </div>
  )
}
