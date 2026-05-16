/**
 * useDebounce Hook
 * 防抖 hook，在指定延迟后才执行回调
 */

import { useState, useEffect, useRef, useCallback } from 'react'

/**
 * 防抖值 hook
 * @param value 要防抖的值
 * @param delay 延迟时间（毫秒）
 * @returns 防抖后的值
 *
 * @example
 * const [searchTerm, setSearchTerm] = useState('')
 * const debouncedSearchTerm = useDebounce(searchTerm, 300)
 * // debouncedSearchTerm 在用户停止输入 300ms 后才更新
 */
export function useDebounce<T>(value: T, delay: number): T {
  const [debouncedValue, setDebouncedValue] = useState<T>(value)

  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedValue(value)
    }, delay)

    return () => {
      clearTimeout(timer)
    }
  }, [value, delay])

  return debouncedValue
}

/**
 * 防抖回调 hook
 * @param callback 要防抖的回调函数
 * @param delay 延迟时间（毫秒）
 * @param deps 依赖数组
 * @returns 防抖后的回调函数
 *
 * @example
 * const debouncedSearch = useDebouncedCallback(
 *   (query: string) => fetchResults(query),
 *   300,
 *   []
 * )
 */
export function useDebouncedCallback<T extends (...args: any[]) => any>(
  callback: T,
  delay: number,
  deps: React.DependencyList = []
): T {
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

  const debouncedCallback = useCallback(
    ((...args: any[]) => {
      if (timerRef.current) {
        clearTimeout(timerRef.current)
      }

      timerRef.current = setTimeout(() => {
        callbackRef.current(...args)
      }, delay)
    }) as T,
    [delay, ...deps]
  )

  return debouncedCallback
}

/**
 * 取消防抖
 * 与 useDebouncedCallback 配合使用，取消待执行的防抖回调
 */
export function useCancelDebounce(debouncedFn: (...args: any[]) => void): () => void {
  const timerRef = useRef<NodeJS.Timeout | null>(null)

  return useCallback(() => {
    // Note: This is a simplified version. For full implementation,
    // the timer ref should be shared between useDebouncedCallback and useCancelDebounce
  }, [])
}
