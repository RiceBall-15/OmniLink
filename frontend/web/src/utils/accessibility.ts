/**
 * 无障碍访问工具
 * 支持ARIA标签、键盘导航、屏幕阅读器
 */

import React, { useEffect, useRef, useCallback } from 'react';

// ARIA属性接口
export interface AriaProps {
  'aria-label'?: string;
  'aria-labelledby'?: string;
  'aria-describedby'?: string;
  'aria-hidden'?: boolean;
  'aria-live'?: 'off' | 'polite' | 'assertive';
  'aria-atomic'?: boolean;
  'aria-busy'?: boolean;
  'aria-current'?: boolean | 'page' | 'step' | 'location' | 'date' | 'time';
  'aria-expanded'?: boolean;
  'aria-haspopup'?: boolean | 'menu' | 'listbox' | 'tree' | 'grid' | 'dialog';
  'aria-pressed'?: boolean;
  'aria-selected'?: boolean;
  'aria-checked'?: boolean | 'mixed';
  'aria-disabled'?: boolean;
  'aria-invalid'?: boolean | 'grammar' | 'spelling';
  'aria-required'?: boolean;
  'aria-multiline'?: boolean;
  'aria-multiselectable'?: boolean;
  'aria-orientation'?: 'horizontal' | 'vertical';
  'aria-readonly'?: boolean;
  'aria-sort'?: 'none' | 'ascending' | 'descending' | 'other';
  'aria-valuemax'?: number;
  'aria-valuemin'?: number;
  'aria-valuenow'?: number;
  'aria-valuetext'?: string;
  role?: string;
}

// 键盘导航Hook
export function useKeyboardNavigation(
  items: HTMLElement[],
  options?: {
    loop?: boolean;
    orientation?: 'horizontal' | 'vertical';
    onSelect?: (index: number) => void;
  }
) {
  const { loop = true, orientation = 'vertical', onSelect } = options || {};
  const currentIndex = useRef(0);

  const handleKeyDown = useCallback(
    (event: KeyboardEvent) => {
      const { key } = event;
      let newIndex = currentIndex.current;

      switch (key) {
        case 'ArrowUp':
        case 'ArrowLeft':
          event.preventDefault();
          newIndex = currentIndex.current - 1;
          if (newIndex < 0) {
            newIndex = loop ? items.length - 1 : 0;
          }
          break;

        case 'ArrowDown':
        case 'ArrowRight':
          event.preventDefault();
          newIndex = currentIndex.current + 1;
          if (newIndex >= items.length) {
            newIndex = loop ? 0 : items.length - 1;
          }
          break;

        case 'Home':
          event.preventDefault();
          newIndex = 0;
          break;

        case 'End':
          event.preventDefault();
          newIndex = items.length - 1;
          break;

        case 'Enter':
        case ' ':
          event.preventDefault();
          onSelect?.(currentIndex.current);
          return;

        default:
          return;
      }

      // 更新焦点
      if (newIndex !== currentIndex.current && items[newIndex]) {
        items[currentIndex.current]?.setAttribute('tabindex', '-1');
        items[newIndex].setAttribute('tabindex', '0');
        items[newIndex].focus();
        currentIndex.current = newIndex;
      }
    },
    [items, loop, onSelect]
  );

  useEffect(() => {
    // 初始化第一个项目
    if (items.length > 0) {
      items[0]?.setAttribute('tabindex', '0');
      items.slice(1).forEach((item) => item.setAttribute('tabindex', '-1'));
    }

    // 添加键盘事件监听
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [items, handleKeyDown]);

  return {
    currentIndex: currentIndex.current,
    setFocus: (index: number) => {
      if (items[index]) {
        items[currentIndex.current]?.setAttribute('tabindex', '-1');
        items[index].setAttribute('tabindex', '0');
        items[index].focus();
        currentIndex.current = index;
      }
    },
  };
}

// 焦点陷阱Hook
export function useFocusTrap(isActive: boolean) {
  const containerRef = useRef<HTMLElement | null>(null);
  const previousFocusRef = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (!isActive) return;

    // 保存当前焦点
    previousFocusRef.current = document.activeElement as HTMLElement;

    // 获取可聚焦元素
    const getFocusableElements = () => {
      if (!containerRef.current) return [];
      return Array.from(
        containerRef.current.querySelectorAll(
          'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
        )
      ) as HTMLElement[];
    };

    // 设置焦点到第一个元素
    const focusableElements = getFocusableElements();
    if (focusableElements.length > 0) {
      focusableElements[0].focus();
    }

    // 处理Tab键
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Tab') return;

      const focusableElements = getFocusableElements();
      const firstElement = focusableElements[0];
      const lastElement = focusableElements[focusableElements.length - 1];

      if (event.shiftKey) {
        if (document.activeElement === firstElement) {
          event.preventDefault();
          lastElement?.focus();
        }
      } else {
        if (document.activeElement === lastElement) {
          event.preventDefault();
          firstElement?.focus();
        }
      }
    };

    document.addEventListener('keydown', handleKeyDown);

    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      // 恢复之前的焦点
      previousFocusRef.current?.focus();
    };
  }, [isActive]);

  return containerRef;
}

// 屏幕阅读器公告
export function announce(message: string, priority: 'polite' | 'assertive' = 'polite') {
  const announcer = document.getElementById('sr-announcer');
  if (announcer) {
    announcer.setAttribute('aria-live', priority);
    announcer.textContent = message;
  }
}

// 跳过链接组件
export function SkipLink({ targetId, children }: { targetId: string; children: React.ReactNode }) {
  return (
    <a
      href={`#${targetId}`}
      className="skip-link"
      style={{
        position: 'absolute',
        top: '-40px',
        left: 0,
        background: '#000',
        color: '#fff',
        padding: '8px',
        zIndex: 1000,
        transition: 'top 0.2s',
      }}
      onFocus={(e) => {
        e.currentTarget.style.top = '0';
      }}
      onBlur={(e) => {
        e.currentTarget.style.top = '-40px';
      }}
    >
      {children}
    </a>
  );
}

// 屏幕阅读器公告组件
export function ScreenReaderAnnouncer() {
  return (
    <div
      id="sr-announcer"
      aria-live="polite"
      aria-atomic="true"
      style={{
        position: 'absolute',
        width: '1px',
        height: '1px',
        padding: 0,
        margin: '-1px',
        overflow: 'hidden',
        clip: 'rect(0, 0, 0, 0)',
        whiteSpace: 'nowrap',
        border: 0,
      }}
    />
  );
}

export default {
  useKeyboardNavigation,
  useFocusTrap,
  announce,
  SkipLink,
  ScreenReaderAnnouncer,
};
