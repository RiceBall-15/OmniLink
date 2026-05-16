import { useState, useEffect, useCallback, useRef } from 'react'

/**
 * 网络连接质量
 */
type NetworkQuality = 'excellent' | 'good' | 'poor' | 'offline'

/**
 * 网络状态信息
 */
interface NetworkStatus {
  /** 是否在线 */
  isOnline: boolean
  /** 网络连接质量 */
  quality: NetworkQuality
  /** 网络连接类型 */
  connectionType: string | null
  /** 下行带宽 (Mbps) */
  downlink: number | null
  /** 往返时间 (ms) */
  rtt: number | null
  /** 是否省流量模式 */
  saveData: boolean
  /** 上次在线时间 */
  lastOnlineTime: Date | null
  /** 离线持续时间（毫秒） */
  offlineDuration: number
}

/**
 * 网络状态检测 Hook
 * 监听浏览器网络状态变化，检测连接质量
 */
export function useNetworkStatus(): NetworkStatus {
  const [isOnline, setIsOnline] = useState(
    typeof navigator !== 'undefined' ? navigator.onLine : true
  )
  const [quality, setQuality] = useState<NetworkQuality>('good')
  const [connectionType, setConnectionType] = useState<string | null>(null)
  const [downlink, setDownlink] = useState<number | null>(null)
  const [rtt, setRtt] = useState<number | null>(null)
  const [saveData, setSaveData] = useState(false)
  const [lastOnlineTime, setLastOnlineTime] = useState<Date | null>(null)
  const [offlineDuration, setOfflineDuration] = useState(0)

  const offlineStartRef = useRef<number | null>(null)
  const durationTimerRef = useRef<NodeJS.Timeout | null>(null)

  /**
   * 获取网络连接信息
   */
  const updateConnectionInfo = useCallback(() => {
    // Network Information API (部分浏览器支持)
    const nav = navigator as any
    const connection = nav.connection || nav.mozConnection || nav.webkitConnection

    if (connection) {
      setConnectionType(connection.effectiveType || connection.type || null)
      setDownlink(connection.downlink ?? null)
      setRtt(connection.rtt ?? null)
      setSaveData(connection.saveData ?? false)

      // 根据有效连接类型判断质量
      switch (connection.effectiveType) {
        case '4g':
          setQuality(connection.rtt < 100 ? 'excellent' : 'good')
          break
        case '3g':
          setQuality('poor')
          break
        case '2g':
        case 'slow-2g':
          setQuality('poor')
          break
        default:
          // 如果没有 effectiveType，根据 RTT 判断
          if (connection.rtt !== undefined) {
            if (connection.rtt < 50) setQuality('excellent')
            else if (connection.rtt < 150) setQuality('good')
            else setQuality('poor')
          }
      }
    }
  }, [])

  /**
   * 处理在线事件
   */
  const handleOnline = useCallback(() => {
    setIsOnline(true)
    setQuality('good') // 恢复时先设为 good，后续更新精确值
    setLastOnlineTime(new Date())

    // 停止离线计时
    if (durationTimerRef.current) {
      clearInterval(durationTimerRef.current)
      durationTimerRef.current = null
    }
    offlineStartRef.current = null
    setOfflineDuration(0)

    // 更新连接信息
    updateConnectionInfo()
  }, [updateConnectionInfo])

  /**
   * 处理离线事件
   */
  const handleOffline = useCallback(() => {
    setIsOnline(false)
    setQuality('offline')
    offlineStartRef.current = Date.now()

    // 开始离线计时
    durationTimerRef.current = setInterval(() => {
      if (offlineStartRef.current) {
        setOfflineDuration(Date.now() - offlineStartRef.current)
      }
    }, 1000)
  }, [])

  /**
   * 处理连接信息变化
   */
  const handleConnectionChange = useCallback(() => {
    updateConnectionInfo()
  }, [updateConnectionInfo])

  useEffect(() => {
    // 初始化
    updateConnectionInfo()

    window.addEventListener('online', handleOnline)
    window.addEventListener('offline', handleOffline)

    // 监听连接信息变化
    const nav = navigator as any
    const connection = nav.connection || nav.mozConnection || nav.webkitConnection
    if (connection) {
      connection.addEventListener('change', handleConnectionChange)
    }

    return () => {
      window.removeEventListener('online', handleOnline)
      window.removeEventListener('offline', handleOffline)

      if (connection) {
        connection.removeEventListener('change', handleConnectionChange)
      }

      if (durationTimerRef.current) {
        clearInterval(durationTimerRef.current)
      }
    }
  }, [handleOnline, handleOffline, handleConnectionChange, updateConnectionInfo])

  return {
    isOnline,
    quality,
    connectionType,
    downlink,
    rtt,
    saveData,
    lastOnlineTime,
    offlineDuration,
  }
}

/**
 * 格式化离线持续时间
 */
export function formatOfflineDuration(ms: number): string {
  if (ms < 1000) return '刚刚'
  if (ms < 60000) return `${Math.floor(ms / 1000)} 秒`
  if (ms < 3600000) return `${Math.floor(ms / 60000)} 分钟`
  return `${Math.floor(ms / 3600000)} 小时 ${Math.floor((ms % 3600000) / 60000)} 分钟`
}
