import { useEffect, useRef, useState, useCallback } from 'react'

/**
 * FPS 监控配置
 */
interface FPSMonitorOptions {
  /** 采样间隔（毫秒） */
  sampleInterval?: number
  /** 低 FPS 阈值 */
  lowFPSThreshold?: number
  /** 是否自动开始 */
  autoStart?: boolean
}

/**
 * FPS 监控结果
 */
interface FPSMonitorResult {
  /** 当前 FPS */
  fps: number
  /** 平均 FPS */
  avgFPS: number
  /** 最低 FPS */
  minFPS: number
  /** 是否低 FPS */
  isLowFPS: boolean
  /** 开始监控 */
  start: () => void
  /** 停止监控 */
  stop: () => void
  /** 重置数据 */
  reset: () => void
}

/**
 * FPS 监控钩子
 * 使用 requestAnimationFrame 精确计算帧率
 */
export function useFPSMonitor(options: FPSMonitorOptions = {}): FPSMonitorResult {
  const { sampleInterval = 1000, lowFPSThreshold = 30, autoStart = true } = options

  const [fps, setFps] = useState(60)
  const [avgFPS, setAvgFPS] = useState(60)
  const [minFPS, setMinFPS] = useState(60)

  const frameCountRef = useRef(0)
  const lastTimeRef = useRef(0)
  const fpsHistoryRef = useRef<number[]>([])
  const rafIdRef = useRef<number>(0)
  const isRunningRef = useRef(false)

  const tick = useCallback((timestamp: number) => {
    if (!lastTimeRef.current) {
      lastTimeRef.current = timestamp
    }

    frameCountRef.current++

    const elapsed = timestamp - lastTimeRef.current

    if (elapsed >= sampleInterval) {
      const currentFPS = Math.round((frameCountRef.current * 1000) / elapsed)

      setFps(currentFPS)

      fpsHistoryRef.current.push(currentFPS)
      // 只保留最近 60 个采样
      if (fpsHistoryRef.current.length > 60) {
        fpsHistoryRef.current.shift()
      }

      const history = fpsHistoryRef.current
      setAvgFPS(Math.round(history.reduce((a, b) => a + b, 0) / history.length))
      setMinFPS(Math.min(...history))

      frameCountRef.current = 0
      lastTimeRef.current = timestamp
    }

    if (isRunningRef.current) {
      rafIdRef.current = requestAnimationFrame(tick)
    }
  }, [sampleInterval])

  const start = useCallback(() => {
    if (isRunningRef.current) return
    isRunningRef.current = true
    lastTimeRef.current = 0
    frameCountRef.current = 0
    rafIdRef.current = requestAnimationFrame(tick)
  }, [tick])

  const stop = useCallback(() => {
    isRunningRef.current = false
    if (rafIdRef.current) {
      cancelAnimationFrame(rafIdRef.current)
      rafIdRef.current = 0
    }
  }, [])

  const reset = useCallback(() => {
    fpsHistoryRef.current = []
    setFps(60)
    setAvgFPS(60)
    setMinFPS(60)
  }, [])

  useEffect(() => {
    if (autoStart) {
      start()
    }
    return stop
  }, [autoStart, start, stop])

  return {
    fps,
    avgFPS,
    minFPS,
    isLowFPS: fps < lowFPSThreshold,
    start,
    stop,
    reset,
  }
}

/**
 * 组件渲染性能配置
 */
interface RenderPerformanceOptions {
  /** 组件名称（用于日志） */
  name: string
  /** 慢渲染阈值（毫秒） */
  slowThreshold?: number
  /** 是否启用 */
  enabled?: boolean
}

/**
 * 组件渲染性能监控钩子
 * 记录渲染次数和耗时
 */
export function useRenderPerformance(options: RenderPerformanceOptions) {
  const { name, slowThreshold = 16, enabled = process.env.NODE_ENV === 'development' } = options

  const renderCountRef = useRef(0)
  const renderTimeRef = useRef<number[]>([])
  const lastRenderStartRef = useRef(0)

  // 标记渲染开始
  if (enabled) {
    lastRenderStartRef.current = performance.now()
  }

  useEffect(() => {
    if (!enabled) return

    const renderTime = performance.now() - lastRenderStartRef.current
    renderCountRef.current++
    renderTimeRef.current.push(renderTime)

    // 只保留最近 100 次
    if (renderTimeRef.current.length > 100) {
      renderTimeRef.current.shift()
    }

    // 慢渲染警告
    if (renderTime > slowThreshold) {
      console.warn(
        `[RenderPerf] ${name} 渲染耗时 ${renderTime.toFixed(2)}ms (阈值: ${slowThreshold}ms)，第 ${renderCountRef.current} 次渲染`
      )
    }
  })

  const getStats = useCallback(() => {
    const times = renderTimeRef.current
    if (times.length === 0) return null

    return {
      renderCount: renderCountRef.current,
      avgRenderTime: times.reduce((a, b) => a + b, 0) / times.length,
      maxRenderTime: Math.max(...times),
      minRenderTime: Math.min(...times),
      slowRenders: times.filter(t => t > slowThreshold).length,
    }
  }, [slowThreshold])

  return { getStats }
}

/**
 * 页面加载性能指标
 */
interface PagePerformance {
  /** DNS 查询时间 */
  dnsTime: number
  /** TCP 连接时间 */
  tcpTime: number
  /** 请求响应时间 */
  requestTime: number
  /** DOM 解析时间 */
  domParseTime: number
  /** DOM Content Loaded */
  domContentLoaded: number
  /** 页面完全加载 */
  loadComplete: number
  /** 首次内容绘制 */
  fcp: number
  /** 最大内容绘制 */
  lcp: number | null
  /** 累积布局偏移 */
  cls: number | null
}

/**
 * 页面加载性能监控钩子
 * 收集 Web Vitals 指标
 */
export function usePagePerformance(): PagePerformance | null {
  const [metrics, setMetrics] = useState<PagePerformance | null>(null)

  useEffect(() => {
    // 等待页面加载完成
    const collectMetrics = () => {
      const nav = performance.getEntriesByType('navigation')[0] as PerformanceNavigationTiming | undefined
      if (!nav) return

      const fcp = performance.getEntriesByName('first-contentful-paint')[0]

      const data: PagePerformance = {
        dnsTime: nav.domainLookupEnd - nav.domainLookupStart,
        tcpTime: nav.connectEnd - nav.connectStart,
        requestTime: nav.responseEnd - nav.requestStart,
        domParseTime: nav.domInteractive - nav.startTime,
        domContentLoaded: nav.domContentLoadedEventEnd - nav.startTime,
        loadComplete: nav.loadEventEnd - nav.startTime,
        fcp: fcp?.startTime ?? 0,
        lcp: null,
        cls: null,
      }

      // LCP
      try {
        const lcpEntries = performance.getEntriesByType('largest-contentful-paint')
        if (lcpEntries.length > 0) {
          data.lcp = lcpEntries[lcpEntries.length - 1].startTime
        }
      } catch {
        // 浏览器不支持
      }

      // CLS
      try {
        const clsEntries = performance.getEntriesByType('layout-shift') as any[]
        data.cls = clsEntries.reduce((sum, entry) => sum + (entry.hadRecentInput ? 0 : entry.value), 0)
      } catch {
        // 浏览器不支持
      }

      setMetrics(data)
    }

    if (document.readyState === 'complete') {
      // 页面已加载，延迟收集以确保所有指标可用
      setTimeout(collectMetrics, 100)
    } else {
      window.addEventListener('load', () => setTimeout(collectMetrics, 100))
    }
  }, [])

  return metrics
}

/**
 * 内存使用监控钩子
 * 仅在支持 performance.memory 的浏览器中有效
 */
export function useMemoryMonitor(interval = 5000) {
  const [memoryInfo, setMemoryInfo] = useState<{
    usedJSHeapSize: number
    totalJSHeapSize: number
    jsHeapSizeLimit: number
    usagePercent: number
  } | null>(null)

  useEffect(() => {
    // performance.memory 仅在 Chrome 中可用
    const perfMemory = (performance as any).memory
    if (!perfMemory) return

    const updateMemory = () => {
      setMemoryInfo({
        usedJSHeapSize: perfMemory.usedJSHeapSize,
        totalJSHeapSize: perfMemory.totalJSHeapSize,
        jsHeapSizeLimit: perfMemory.jsHeapSizeLimit,
        usagePercent: Math.round((perfMemory.usedJSHeapSize / perfMemory.jsHeapSizeLimit) * 100),
      })
    }

    updateMemory()
    const timer = setInterval(updateMemory, interval)

    return () => clearInterval(timer)
  }, [interval])

  return memoryInfo
}
