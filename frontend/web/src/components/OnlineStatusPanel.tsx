import { useState, useEffect } from 'react'
import { OnlineStatus, OnlineStatus as OnlineStatusEnum } from '../types/message'
import { useOnlineStatus } from '../hooks/useOnlineStatus'
import { useWebSocket } from '../hooks/useWebSocket'
import { OnlineUsersList, UserOnlineInfo } from './OnlineUsersList'
import { OnlineStatusIndicator } from './OnlineStatusIndicator'
import { UserOnlineStatus } from './UserOnlineStatus'
import './OnlineStatusPanel.css'

/**
 * 在线状态面板组件
 * 集成在线状态管理、WebSocket 连接、用户列表等功能
 */
export function OnlineStatusPanel() {
  // 在线状态管理
  const {
    status: currentStatus,
    isOnline,
    updateStatus,
    sendHeartbeat,
    onUserStatusChange,
    usersStatus,
  } = useOnlineStatus({
    heartbeatInterval: 30000, // 30 秒
    offlineTimeout: 60000, // 60 秒
    autoUpdate: true,
  })

  // WebSocket 连接
  const { status: wsStatus, isConnected } = useWebSocket({
    url: import.meta.env.VITE_WS_URL || 'ws://localhost:8080/ws',
    autoConnect: true,
    autoReconnect: true,
    heartbeatInterval: 30000,
    heartbeatTimeout: 60000,
    onStatusUpdate: (data) => {
      console.log('收到状态更新:', data)
      onUserStatusChange(data.userId, data.status)
    },
    onMessage: (message) => {
      console.log('收到消息:', message)
    },
  })

  // 当前用户信息
  const currentUser = {
    userId: 'current-user',
    nickname: '我',
    avatar: undefined,
    status: currentStatus,
  }

  // 模拟用户列表数据（实际应该从 API 获取）
  const [users, setUsers] = useState<UserOnlineInfo[]>([
    {
      userId: '1',
      nickname: '张三',
      avatar: undefined,
      status: OnlineStatusEnum.ONLINE,
    },
    {
      userId: '2',
      nickname: '李四',
      avatar: undefined,
      status: OnlineStatusEnum.BUSY,
    },
    {
      userId: '3',
      nickname: '王五',
      avatar: undefined,
      status: OnlineStatusEnum.AWAY,
    },
    {
      userId: '4',
      nickname: '赵六',
      avatar: undefined,
      status: OnlineStatusEnum.OFFLINE,
    },
  ])

  // 更新用户列表中的在线状态
  useEffect(() => {
    setUsers((prevUsers) =>
      prevUsers.map((user) => {
        const userStatus = usersStatus.get(user.userId)
        if (userStatus) {
          return { ...user, status: userStatus }
        }
        return user
      })
    )
  }, [usersStatus])

  // 处理状态切换
  const handleStatusChange = async (newStatus: OnlineStatus) => {
    await updateStatus(newStatus)
  }

  // 处理用户点击
  const handleUserClick = (userId: string) => {
    console.log('点击用户:', userId)
    // 这里可以打开聊天窗口或用户详情
  }

  // 手动发送心跳
  const handleSendHeartbeat = async () => {
    await sendHeartbeat()
  }

  return (
    <div className="online-status-panel">
      {/* 面板标题 */}
      <div className="panel-header">
        <h2 className="panel-title">在线状态</h2>
        <div className="panel-status">
          <span className={`status-indicator ${isConnected ? 'connected' : 'disconnected'}`} />
          <span className="status-text">
            {isConnected ? '已连接' : '未连接'}
          </span>
        </div>
      </div>

      {/* 当前用户状态 */}
      <div className="current-user-section">
        <div className="section-title">我的状态</div>
        <div className="current-user-info">
          <UserOnlineStatus
            userId={currentUser.userId}
            avatar={currentUser.avatar}
            nickname={currentUser.nickname}
            status={currentUser.status}
            avatarSize={48}
            showStatusLabel={true}
          />
        </div>
        <div className="status-selector">
          <button
            className={`status-btn ${currentStatus === OnlineStatusEnum.ONLINE ? 'active' : ''}`}
            onClick={() => handleStatusChange(OnlineStatusEnum.ONLINE)}
          >
            <OnlineStatusIndicator status={OnlineStatusEnum.ONLINE} size={10} />
            在线
          </button>
          <button
            className={`status-btn ${currentStatus === OnlineStatusEnum.AWAY ? 'active' : ''}`}
            onClick={() => handleStatusChange(OnlineStatusEnum.AWAY)}
          >
            <OnlineStatusIndicator status={OnlineStatusEnum.AWAY} size={10} />
            离开
          </button>
          <button
            className={`status-btn ${currentStatus === OnlineStatusEnum.BUSY ? 'active' : ''}`}
            onClick={() => handleStatusChange(OnlineStatusEnum.BUSY)}
          >
            <OnlineStatusIndicator status={OnlineStatusEnum.BUSY} size={10} />
            忙碌
          </button>
          <button
            className={`status-btn ${currentStatus === OnlineStatusEnum.OFFLINE ? 'active' : ''}`}
            onClick={() => handleStatusChange(OnlineStatusEnum.OFFLINE)}
          >
            <OnlineStatusIndicator status={OnlineStatusEnum.OFFLINE} size={10} />
            离线
          </button>
        </div>
      </div>

      {/* 调试信息 */}
      <div className="debug-section">
        <div className="section-title">调试信息</div>
        <div className="debug-info">
          <div className="debug-item">
            <span className="debug-label">WebSocket 状态:</span>
            <span className={`debug-value ws-${wsStatus}`}>{wsStatus}</span>
          </div>
          <div className="debug-item">
            <span className="debug-label">在线状态:</span>
            <span className={`debug-value status-${currentStatus}`}>{currentStatus}</span>
          </div>
          <div className="debug-item">
            <span className="debug-label">是否在线:</span>
            <span className="debug-value">{isOnline ? '是' : '否'}</span>
          </div>
          <div className="debug-actions">
            <button className="btn btn-sm btn-secondary" onClick={handleSendHeartbeat}>
              发送心跳
            </button>
          </div>
        </div>
      </div>

      {/* 在线用户列表 */}
      <div className="users-section">
        <div className="section-title">在线用户</div>
        <OnlineUsersList
          users={users}
          showSearch={true}
          showStatusLabel={false}
          onUserClick={handleUserClick}
          showGroups={true}
          maxHeight={300}
        />
      </div>
    </div>
  )
}
