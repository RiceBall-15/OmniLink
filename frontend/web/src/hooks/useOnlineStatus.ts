import { useEffect, useState, useRef, useCallback } from 'react'
import { OnlineStatus } from '../types/message'
import { messageService } from '../services/messageService'

interface UseOnlineStatusOptions {
  // 心跳间隔（毫秒），默认30秒
  heartbeatInterval?: number
  // 离线超时时间（毫秒），默认60秒
  offlineTimeout?: number
  // 是否自动更新状态
  autoUpdate?: boolean
}

export const useOnlineStatus = (options: UseOnlineStatusOptions = {}) => {
  const {
    heartbeatInterval = 30000,
    offlineTimeout = 60000,
    autoUpdate = true,
  } = options

  const [status, setStatus] = useState<OnlineStatus>(OnlineStatus.ONLINE)
  const [usersStatus, setUsersStatus] = useState<Map<string, OnlineStatus>>(new Map())
  const lastActivityRef = useRef<number>(Date.now())
  const heartbeatTimerRef = useRef<NodeJS.Timeout | null>(null)
  const awayCheckTimerRef = useRef<NodeJS.Timeout | null>(null)

  // 计算是否在线
  const isOnline = status !== OnlineStatus.OFFLINE

  // 监听用户状态变化
  const onUserStatusChange = useCallback((userId: string, newStatus: OnlineStatus) => {
    setUsersStatus((prev) => new Map(prev).set(userId, newStatus))
  }, [])

  // 更新在线状态到服务器
  const updateStatus = useCallback(async (newStatus: OnlineStatus) => {
    try {
      await messageService.updateOnlineStatus(newStatus)
      setStatus(newStatus)
    } catch (error) {
      console.error('Failed to update online status:', error)
    }
  }, [])

  // 发送心跳 PING
  const sendHeartbeat = useCallback(async () => {
    try {
      await updateStatus(status)
    } catch (error) {
      console.error('Failed to send heartbeat:', error)
    }
  }, [status, updateStatus])

  // 检测是否进入离开状态
  const checkAwayStatus = useCallback(() => {
    const now = Date.now()
    const timeSinceLastActivity = now - lastActivityRef.current

    if (timeSinceLastActivity > offlineTimeout && status !== OnlineStatus.AWAY) {
      updateStatus(OnlineStatus.AWAY)
    }
  }, [offlineTimeout, status, updateStatus])

  // 记录用户活动
  const trackActivity = useCallback(() => {
    lastActivityRef.current = Date.now()
    if (status === OnlineStatus.AWAY) {
      updateStatus(OnlineStatus.ONLINE)
    }
  }, [status, updateStatus])

  // 启动心跳定时器
  useEffect(() => {
    if (!autoUpdate) {
      return
    }

    // 发送初始心跳
    sendHeartbeat()

    // 设置心跳定时器
    heartbeatTimerRef.current = setInterval(() => {
      sendHeartbeat()
    }, heartbeatInterval)

    // 设置离开状态检查定时器
    awayCheckTimerRef.current = setInterval(() => {
      checkAwayStatus()
    }, 60000) // 每分钟检查一次

    // 监听用户活动
    const activityEvents = ['mousedown', 'mousemove', 'keydown', 'scroll', 'touchstart']
    activityEvents.forEach(event => {
      window.addEventListener(event, trackActivity)
    })

    return () => {
      // 清理定时器
      if (heartbeatTimerRef.current) {
        clearInterval(heartbeatTimerRef.current)
      }
      if (awayCheckTimerRef.current) {
        clearInterval(awayCheckTimerRef.current)
      }

      // 移除事件监听
      activityEvents.forEach(event => {
        window.removeEventListener(event, trackActivity)
      })

      // 组件卸载时设置为离线
      updateStatus(OnlineStatus.OFFLINE)
    }
  }, [heartbeatInterval, sendHeartbeat, checkAwayStatus, trackActivity, updateStatus, autoUpdate])

  // 手动设置状态
  const setStatusManually = useCallback((newStatus: OnlineStatus) => {
    updateStatus(newStatus)
    if (newStatus !== OnlineStatus.AWAY) {
      lastActivityRef.current = Date.now()
    }
  }, [updateStatus])

  return {
    status,
    isOnline,
    updateStatus,
    sendHeartbeat,
    onUserStatusChange,
    usersStatus,
    setStatus: setStatusManually,
    trackActivity,
  }
}
