/**
 * 高对比度主题组件
 * 支持主题切换和用户偏好
 */

import React, { useState, useEffect, useCallback } from 'react';

export interface HighContrastThemeProps {
  children: React.ReactNode;
  className?: string;
}

export function HighContrastTheme({ children, className = '' }: HighContrastThemeProps) {
  const [isHighContrast, setIsHighContrast] = useState(false);
  const [theme, setTheme] = useState<'light' | 'dark' | 'high-contrast'>('light');

  // 检测系统高对比度偏好
  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-contrast: high)');
    setIsHighContrast(mediaQuery.matches);

    const handleChange = (e: MediaQueryListEvent) => {
      setIsHighContrast(e.matches);
    };

    mediaQuery.addEventListener('change', handleChange);
    return () => mediaQuery.removeEventListener('change', handleChange);
  }, []);

  // 应用主题
  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
    document.documentElement.classList.toggle('high-contrast', isHighContrast);
  }, [theme, isHighContrast]);

  // 切换主题
  const toggleTheme = useCallback(() => {
    setTheme((prev) => {
      switch (prev) {
        case 'light':
          return 'dark';
        case 'dark':
          return 'high-contrast';
        case 'high-contrast':
          return 'light';
        default:
          return 'light';
      }
    });
  }, []);

  // 设置高对比度
  const setHighContrast = useCallback((enabled: boolean) => {
    setIsHighContrast(enabled);
  }, []);

  return (
    <div className={`theme-provider ${theme} ${isHighContrast ? 'high-contrast' : ''} ${className}`}>
      {children}
      <button
        className="theme-toggle"
        onClick={toggleTheme}
        aria-label={`当前主题: ${theme}，点击切换`}
        style={{
          position: 'fixed',
          bottom: '20px',
          right: '20px',
          width: '48px',
          height: '48px',
          borderRadius: '50%',
          border: '2px solid currentColor',
          background: 'var(--bg-primary, #fff)',
          color: 'var(--text-primary, #333)',
          cursor: 'pointer',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          fontSize: '20px',
          zIndex: 1000,
          boxShadow: '0 2px 8px rgba(0, 0, 0, 0.2)',
        }}
      >
        {theme === 'light' ? '🌙' : theme === 'dark' ? '☀️' : '🔲'}
      </button>
    </div>
  );
}

// 高对比度样式
export const highContrastStyles = `
  :root {
    --bg-primary: #ffffff;
    --bg-secondary: #f5f5f5;
    --text-primary: #000000;
    --text-secondary: #333333;
    --border-color: #000000;
    --focus-color: #0000ff;
    --link-color: #0000ff;
    --visited-color: #800080;
    --error-color: #ff0000;
    --success-color: #008000;
    --warning-color: #ffff00;
  }

  [data-theme="dark"] {
    --bg-primary: #000000;
    --bg-secondary: #1a1a1a;
    --text-primary: #ffffff;
    --text-secondary: #cccccc;
    --border-color: #ffffff;
    --focus-color: #00ffff;
    --link-color: #00ffff;
    --visited-color: #ff00ff;
    --error-color: #ff6666;
    --success-color: #66ff66;
    --warning-color: #ffff00;
  }

  [data-theme="high-contrast"] {
    --bg-primary: #000000;
    --bg-secondary: #1a1a1a;
    --text-primary: #ffffff;
    --text-secondary: #ffffff;
    --border-color: #ffffff;
    --focus-color: #ffff00;
    --link-color: #ffff00;
    --visited-color: #ff00ff;
    --error-color: #ff0000;
    --success-color: #00ff00;
    --warning-color: #ffff00;
  }

  /* 高对比度模式下的特殊样式 */
  .high-contrast * {
    border-color: var(--border-color) !important;
  }

  .high-contrast a {
    color: var(--link-color) !important;
    text-decoration: underline !important;
  }

  .high-contrast a:visited {
    color: var(--visited-color) !important;
  }

  .high-contrast button,
  .high-contrast input,
  .high-contrast select,
  .high-contrast textarea {
    border: 2px solid var(--border-color) !important;
    background-color: var(--bg-primary) !important;
    color: var(--text-primary) !important;
  }

  .high-contrast :focus {
    outline: 3px solid var(--focus-color) !important;
    outline-offset: 2px !important;
  }

  .high-contrast img {
    border: 1px solid var(--border-color) !important;
  }

  .high-contrast .error {
    color: var(--error-color) !important;
  }

  .high-contrast .success {
    color: var(--success-color) !important;
  }

  .high-contrast .warning {
    color: var(--warning-color) !important;
    background-color: #000000 !important;
  }
`;

// 注入高对比度样式
export function injectHighContrastStyles() {
  if (typeof document !== 'undefined') {
    const styleId = 'high-contrast-styles';
    if (!document.getElementById(styleId)) {
      const style = document.createElement('style');
      style.id = styleId;
      style.textContent = highContrastStyles;
      document.head.appendChild(style);
    }
  }
}

export default HighContrastTheme;
