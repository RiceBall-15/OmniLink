/**
 * useThrottle Hook
 * 节流 hook，在指定时间间隔内最多执行一次
 */

import { useState, useEffect, useRef, useCallback } from 'react'

/**
 * 节流值 hook
 * @param value 要节流的值
 * @param interval 节流间隔（毫秒）
 * @returns 节流后的值
 *
 * @example
 * const [scrollY, setScrollY] = useState(0)
 * const throttledScrollY = useThrottle(scrollY, 100)
 * // scrollY 频繁变化时，throttledScrollY 每 100ms 最多更新一次
 */
export function useThrottle<T>(value: T, interval: number): T {
  const [throttledValue, setThrottledValue] = useState<T>(value)
  const lastUpdatedRef = useRef<number>(Date.now())
  const timerRef = useRef<NodeJS.Timeout | null>(null)

  useEffect(() => {
    const now = Date.now()
    const timeSinceLastUpdate = now - lastUpdatedRef.current

    if (timeSinceLastUpdate >= interval) {
      // 距离上次更新已超过间隔，立即更新
      setThrottledValue(value)
      lastUpdatedRef.current = now
    } else {
      // 设置定时器，在间隔结束时更新
      if (timerRef.current) {
        clearTimeout(timerRef.current)
      }

      timerRef.current = setTimeout(() => {
        setThrottledValue(value)
        lastUpdatedRef.current = Date.now()
      }, interval - timeSinceLastUpdate)
    }

    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current)
      }
    }
  }, [value, interval])

  return throttledValue
}

/**
 * 节流回调 hook
 * @param callback 要节流的回调函数
 * @param interval 节流间隔（毫秒）
 * @param deps 依赖数组
 * @returns 节流后的回调函数
 *
 * @example
 * const throttledScroll = useThrottledCallback(
 *   (event: Event) => updateScrollIndicator(event),
 *   100,
 *   []
 * )
 * window.addEventListener('scroll', throttledScroll)
 */
export function useThrottledCallback<T extends (...args: any[]) => any>(
  callback: T,
  interval: number,
  deps: React.DependencyList = []
): T {
  const lastCallRef = useRef<number>(0)
  const timerRef = useRef<NodeJS.Timeout | null>(null)
  const callbackRef = useRef<T>(callback)

  // 更新回调引用
  useEffect(() => {
    callbackRef.current = callback
  }, [callback])

  // 清理定时器
  useEffect(() => {
    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current)
      }
    }
  }, [])

  const throttledCallback = useCallback(
    ((...args: any[]) => {
      const now = Date.now()
      const timeSinceLastCall = now - lastCallRef.current

      if (timeSinceLastCall >= interval) {
        // 距离上次调用已超过间隔，立即执行
        lastCallRef.current = now
        callbackRef.current(...args)
      } else {
        // 设置定时器，在间隔结束时执行
        if (timerRef.current) {
          clearTimeout(timerRef.current)
        }

        timerRef.current = setTimeout(() => {
          lastCallRef.current = Date.now()
          callbackRef.current(...args)
        }, interval - timeSinceLastCall)
      }
    }) as T,
    [interval, ...deps]
  )

  return throttledCallback
}
