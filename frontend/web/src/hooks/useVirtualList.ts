import { useState, useEffect, useRef, useCallback, type RefObject } from 'react'

/**
 * 虚拟列表配置
 */
interface VirtualListOptions {
  /** 列表项高度（固定） */
  itemHeight: number
  /** 列表总项数 */
  itemCount: number
  /** 容器 Ref */
  containerRef: RefObject<HTMLElement | null>
  /** 额外渲染的缓冲项数 */
  overscan?: number
}

/**
 * 虚拟列表返回值
 */
interface VirtualListResult {
  /** 可见区域起始索引 */
  startIndex: number
  /** 可见区域结束索引 */
  endIndex: number
  /** 列表总高度 */
  totalHeight: number
  /** 容器当前滚动偏移 */
  offsetY: number
  /** 滚动到指定索引 */
  scrollToIndex: (index: number) => void
}

/**
 * 虚拟列表 Hook
 * 实现高性能长列表渲染，只渲染可见区域的元素
 */
export function useVirtualList({
  itemHeight,
  itemCount,
  containerRef,
  overscan = 5,
}: VirtualListOptions): VirtualListResult {
  const [scrollTop, setScrollTop] = useState(0)
  const [containerHeight, setContainerHeight] = useState(0)

  // 监听滚动
  const handleScroll = useCallback(() => {
    const el = containerRef.current
    if (el) {
      setScrollTop(el.scrollTop)
    }
  }, [containerRef])

  // 监听容器尺寸变化
  useEffect(() => {
    const el = containerRef.current
    if (!el) return

    const observer = new ResizeObserver(entries => {
      for (const entry of entries) {
        setContainerHeight(entry.contentRect.height)
      }
    })

    observer.observe(el)
    setContainerHeight(el.clientHeight)
    el.addEventListener('scroll', handleScroll, { passive: true })

    return () => {
      observer.disconnect()
      el.removeEventListener('scroll', handleScroll)
    }
  }, [containerRef, handleScroll])

  // 计算可见范围
  const startIndex = Math.max(0, Math.floor(scrollTop / itemHeight) - overscan)
  const visibleCount = Math.ceil(containerHeight / itemHeight)
  const endIndex = Math.min(itemCount - 1, startIndex + visibleCount + overscan * 2)
  const totalHeight = itemCount * itemHeight
  const offsetY = scrollTop

  // 滚动到指定索引
  const scrollToIndex = useCallback(
    (index: number) => {
      const el = containerRef.current
      if (el) {
        el.scrollTop = index * itemHeight
      }
    },
    [containerRef, itemHeight]
  )

  return {
    startIndex,
    endIndex,
    totalHeight,
    offsetY,
    scrollToIndex,
  }
}
