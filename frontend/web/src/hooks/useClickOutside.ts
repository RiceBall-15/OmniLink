/**
 * useClickOutside Hook
 * 检测点击元素外部的事件
 */

import { useEffect, RefObject } from 'react'

/**
 * 点击外部事件 hook
 * @param ref 要检测的元素引用
 * @param handler 点击外部时的回调
 * @param excludeRefs 可选，排除的元素引用（点击这些元素不触发回调）
 *
 * @example
 * const ref = useRef<HTMLDivElement>(null)
 * useClickOutside(ref, () => {
 *   setIsOpen(false)
 * })
 */
export function useClickOutside(
  ref: RefObject<HTMLElement>,
  handler: (event: MouseEvent | TouchEvent) => void,
  excludeRefs: RefObject<HTMLElement>[] = []
): void {
  useEffect(() => {
    const listener = (event: MouseEvent | TouchEvent) => {
      const target = event.target as HTMLElement

      // 检查点击是否在目标元素内
      if (!ref.current || ref.current.contains(target)) {
        return
      }

      // 检查点击是否在排除元素内
      for (const excludeRef of excludeRefs) {
        if (excludeRef.current && excludeRef.current.contains(target)) {
          return
        }
      }

      handler(event)
    }

    document.addEventListener('mousedown', listener)
    document.addEventListener('touchstart', listener)

    return () => {
      document.removeEventListener('mousedown', listener)
      document.removeEventListener('touchstart', listener)
    }
  }, [ref, handler, ...excludeRefs])
}

export default useClickOutside
