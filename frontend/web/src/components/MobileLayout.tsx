/**
 * 移动端响应式布局组件
 * 支持触摸手势、键盘弹出处理
 */

import React, { useState, useEffect, useCallback, useRef } from 'react';

export interface MobileLayoutProps {
  children: React.ReactNode;
  sidebar?: React.ReactNode;
  header?: React.ReactNode;
  footer?: React.ReactNode;
  className?: string;
}

export function MobileLayout({
  children,
  sidebar,
  header,
  footer,
  className = '',
}: MobileLayoutProps) {
  const [isMobile, setIsMobile] = useState(false);
  const [isSidebarOpen, setIsSidebarOpen] = useState(false);
  const [keyboardHeight, setKeyboardHeight] = useState(0);
  const touchStartRef = useRef<{ x: number; y: number } | null>(null);

  // 检测是否为移动设备
  useEffect(() => {
    const checkMobile = () => {
      setIsMobile(window.innerWidth <= 768);
    };

    checkMobile();
    window.addEventListener('resize', checkMobile);

    return () => window.removeEventListener('resize', checkMobile);
  }, []);

  // 监听键盘弹出
  useEffect(() => {
    if (!isMobile) return;

    const handleResize = () => {
      const visualViewport = window.visualViewport;
      if (visualViewport) {
        const heightDiff = window.innerHeight - visualViewport.height;
        setKeyboardHeight(heightDiff > 0 ? heightDiff : 0);
      }
    };

    window.visualViewport?.addEventListener('resize', handleResize);
    return () => window.visualViewport?.removeEventListener('resize', handleResize);
  }, [isMobile]);

  // 触摸手势处理
  const handleTouchStart = useCallback((e: React.TouchEvent) => {
    const touch = e.touches[0];
    touchStartRef.current = { x: touch.clientX, y: touch.clientY };
  }, []);

  const handleTouchEnd = useCallback(
    (e: React.TouchEvent) => {
      if (!touchStartRef.current) return;

      const touch = e.changedTouches[0];
      const deltaX = touch.clientX - touchStartRef.current.x;
      const deltaY = touch.clientY - touchStartRef.current.y;

      // 判断是否为水平滑动
      if (Math.abs(deltaX) > Math.abs(deltaY) && Math.abs(deltaX) > 50) {
        if (deltaX > 0 && !isSidebarOpen) {
          // 右滑打开侧边栏
          setIsSidebarOpen(true);
        } else if (deltaX < 0 && isSidebarOpen) {
          // 左滑关闭侧边栏
          setIsSidebarOpen(false);
        }
      }

      touchStartRef.current = null;
    },
    [isSidebarOpen]
  );

  // 切换侧边栏
  const toggleSidebar = useCallback(() => {
    setIsSidebarOpen(!isSidebarOpen);
  }, [isSidebarOpen]);

  return (
    <div
      className={`mobile-layout ${isMobile ? 'is-mobile' : ''} ${className}`}
      onTouchStart={handleTouchStart}
      onTouchEnd={handleTouchEnd}
      style={{
        paddingBottom: keyboardHeight,
      }}
    >
      {/* 头部 */}
      {header && (
        <header className="mobile-header">
          {isMobile && sidebar && (
            <button
              className="menu-toggle"
              onClick={toggleSidebar}
              aria-label="Toggle menu"
            >
              ☰
            </button>
          )}
          {header}
        </header>
      )}

      <div className="mobile-content">
        {/* 侧边栏 */}
        {sidebar && (
          <>
            {/* 遮罩层 */}
            {isMobile && isSidebarOpen && (
              <div
                className="sidebar-overlay"
                onClick={() => setIsSidebarOpen(false)}
              />
            )}

            {/* 侧边栏 */}
            <aside
              className={`mobile-sidebar ${isSidebarOpen ? 'open' : ''}`}
            >
              {sidebar}
            </aside>
          </>
        )}

        {/* 主内容 */}
        <main className="mobile-main">{children}</main>
      </div>

      {/* 底部 */}
      {footer && <footer className="mobile-footer">{footer}</footer>}
    </div>
  );
}

/**
 * 触摸友好的按钮组件
 */
export interface TouchButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  children: React.ReactNode;
  variant?: 'primary' | 'secondary' | 'ghost';
  size?: 'small' | 'medium' | 'large';
}

export function TouchButton({
  children,
  variant = 'primary',
  size = 'medium',
  className = '',
  ...props
}: TouchButtonProps) {
  return (
    <button
      className={`touch-button ${variant} ${size} ${className}`}
      {...props}
    >
      {children}
    </button>
  );
}

/**
 * 触摸友好的输入框组件
 */
export interface TouchInputProps
  extends React.InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
}

export function TouchInput({
  label,
  error,
  className = '',
  ...props
}: TouchInputProps) {
  return (
    <div className={`touch-input ${error ? 'has-error' : ''}`}>
      {label && <label className="input-label">{label}</label>}
      <input className={`input-field ${className}`} {...props} />
      {error && <span className="input-error">{error}</span>}
    </div>
  );
}

export default MobileLayout;
