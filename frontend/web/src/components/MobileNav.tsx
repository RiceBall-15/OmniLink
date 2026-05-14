/**
 * 移动端导航组件
 * 支持底部标签栏和手势导航
 */

import React, { useState, useCallback } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';

export interface NavItem {
  id: string;
  label: string;
  icon: string;
  path: string;
  badge?: number;
}

export interface MobileNavProps {
  items: NavItem[];
  className?: string;
}

export function MobileNav({ items, className = '' }: MobileNavProps) {
  const navigate = useNavigate();
  const location = useLocation();
  const [activeItem, setActiveItem] = useState<string>(() => {
    const currentItem = items.find((item) => item.path === location.pathname);
    return currentItem?.id || items[0]?.id || '';
  });

  const handleItemClick = useCallback(
    (item: NavItem) => {
      setActiveItem(item.id);
      navigate(item.path);
    },
    [navigate]
  );

  return (
    <nav className={`mobile-nav ${className}`}>
      {items.map((item) => (
        <button
          key={item.id}
          className={`nav-item ${activeItem === item.id ? 'active' : ''}`}
          onClick={() => handleItemClick(item)}
          aria-label={item.label}
          aria-current={activeItem === item.id ? 'page' : undefined}
        >
          <span className="nav-icon">{item.icon}</span>
          <span className="nav-label">{item.label}</span>
          {item.badge && item.badge > 0 && (
            <span className="nav-badge">{item.badge > 99 ? '99+' : item.badge}</span>
          )}
        </button>
      ))}
    </nav>
  );
}

/**
 * 手势导航Hook
 */
export function useGestureNavigation() {
  const navigate = useNavigate();
  const [gestureState, setGestureState] = useState<{
    startX: number;
    startY: number;
    startTime: number;
  } | null>(null);

  const handleTouchStart = useCallback((e: React.TouchEvent) => {
    const touch = e.touches[0];
    setGestureState({
      startX: touch.clientX,
      startY: touch.clientY,
      startTime: Date.now(),
    });
  }, []);

  const handleTouchEnd = useCallback(
    (e: React.TouchEvent, onSwipeLeft?: () => void, onSwipeRight?: () => void) => {
      if (!gestureState) return;

      const touch = e.changedTouches[0];
      const deltaX = touch.clientX - gestureState.startX;
      const deltaY = touch.clientY - gestureState.startY;
      const deltaTime = Date.now() - gestureState.startTime;

      // 判断是否为有效滑动
      const isValidSwipe =
        Math.abs(deltaX) > 50 && // 水平距离大于50px
        Math.abs(deltaX) > Math.abs(deltaY) && // 水平距离大于垂直距离
        deltaTime < 300; // 时间小于300ms

      if (isValidSwipe) {
        if (deltaX > 0 && onSwipeRight) {
          onSwipeRight();
        } else if (deltaX < 0 && onSwipeLeft) {
          onSwipeLeft();
        }
      }

      setGestureState(null);
    },
    [gestureState]
  );

  return {
    handleTouchStart,
    handleTouchEnd,
  };
}

export default MobileNav;
