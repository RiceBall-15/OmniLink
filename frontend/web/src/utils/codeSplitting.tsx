/**
 * 代码分割工具
 * 支持组件懒加载和错误边界
 */

import React, { Suspense, Component, ErrorInfo, ReactNode } from 'react';

// 错误边界属性
interface ErrorBoundaryProps {
  children: ReactNode;
  fallback?: ReactNode;
  onError?: (error: Error, errorInfo: ErrorInfo) => void;
}

// 错误边界状态
interface ErrorBoundaryState {
  hasError: boolean;
  error?: Error;
}

/**
 * 错误边界组件
 */
export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('ErrorBoundary caught an error:', error, errorInfo);
    this.props.onError?.(error, errorInfo);
  }

  render() {
    if (this.state.hasError) {
      return (
        this.props.fallback || (
          <div className="error-fallback">
            <h3>组件加载失败</h3>
            <p>{this.state.error?.message}</p>
            <button onClick={() => this.setState({ hasError: false })}>
              重试
            </button>
          </div>
        )
      );
    }

    return this.props.children;
  }
}

/**
 * 加载中组件
 */
export function LoadingFallback() {
  return (
    <div className="loading-fallback">
      <div className="spinner"></div>
      <p>加载中...</p>
    </div>
  );
}

/**
 * 懒加载组件包装器
 */
export function lazyLoad(
  importFunc: () => Promise<{ default: React.ComponentType<any> }>,
  fallback?: ReactNode
) {
  const LazyComponent = React.lazy(importFunc);

  return function LazyLoadWrapper(props: any) {
    return (
      <ErrorBoundary>
        <Suspense fallback={fallback || <LoadingFallback />}>
          <LazyComponent {...props} />
        </Suspense>
      </ErrorBoundary>
    );
  };
}

/**
 * 预加载组件
 */
export function preloadComponent(
  importFunc: () => Promise<{ default: React.ComponentType<any> }>
) {
  // 在空闲时间预加载
  if ('requestIdleCallback' in window) {
    (window as any).requestIdleCallback(() => {
      importFunc();
    });
  } else {
    // 降级方案：延迟预加载
    setTimeout(() => {
      importFunc();
    }, 1000);
  }
}

/**
 * 路由级别代码分割
 */
export function createRouteComponent(
  importFunc: () => Promise<{ default: React.ComponentType<any> }>,
  options?: {
    fallback?: ReactNode;
    preload?: boolean;
  }
) {
  const { fallback, preload = false } = options || {};

  // 如果需要预加载
  if (preload) {
    preloadComponent(importFunc);
  }

  return lazyLoad(importFunc, fallback);
}

/**
 * 组件级别代码分割
 */
export function createComponent(
  importFunc: () => Promise<{ default: React.ComponentType<any> }>,
  options?: {
    fallback?: ReactNode;
    displayName?: string;
  }
) {
  const { fallback, displayName } = options || {};

  const LazyComponent = lazyLoad(importFunc, fallback);

  if (displayName) {
    LazyComponent.displayName = displayName;
  }

  return LazyComponent;
}

export default {
  ErrorBoundary,
  LoadingFallback,
  lazyLoad,
  preloadComponent,
  createRouteComponent,
  createComponent,
};
