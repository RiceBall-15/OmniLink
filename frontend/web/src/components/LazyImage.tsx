/**
 * 图片懒加载组件
 * 支持 IntersectionObserver 和图片压缩
 */

import React, { useState, useEffect, useRef, useCallback } from 'react';

export interface LazyImageProps {
  src: string;
  alt: string;
  width?: number | string;
  height?: number | string;
  placeholder?: string;
  errorImage?: string;
  className?: string;
  style?: React.CSSProperties;
  onLoad?: () => void;
  onError?: (error: Error) => void;
  threshold?: number;
  rootMargin?: string;
  quality?: number;
  format?: 'webp' | 'jpeg' | 'png' | 'auto';
}

export function LazyImage({
  src,
  alt,
  width,
  height,
  placeholder = 'data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjAwIiBoZWlnaHQ9IjIwMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48cmVjdCB3aWR0aD0iMjAwIiBoZWlnaHQ9IjIwMCIgZmlsbD0iI2VlZSIvPjx0ZXh0IHg9IjUwJSIgeT0iNTAlIiBmb250LWZhbWlseT0iQXJpYWwiIGZvbnQtc2l6ZT0iMTQiIGZpbGw9IiM5OTkiIHRleHQtYW5jaG9yPSJtaWRkbGUiIGR5PSIuM2VtIj5Mb2FkaW5nLi4uPC90ZXh0Pjwvc3ZnPg==',
  errorImage = 'data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjAwIiBoZWlnaHQ9IjIwMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48cmVjdCB3aWR0aD0iMjAwIiBoZWlnaHQ9IjIwMCIgZmlsbD0iI2VlZSIvPjx0ZXh0IHg9IjUwJSIgeT0iNTAlIiBmb250LWZhbWlseT0iQXJpYWwiIGZvbnQtc2l6ZT0iMTQiIGZpbGw9IiM5OTkiIHRleHQtYW5jaG9yPSJtaWRkbGUiIGR5PSIuM2VtIj5JbWFnZSBFcnJvcjwvdGV4dD48L3N2Zz4=',
  className = '',
  style,
  onLoad,
  onError,
  threshold = 0.1,
  rootMargin = '100px',
  quality = 80,
  format = 'auto',
}: LazyImageProps) {
  const [imageSrc, setImageSrc] = useState(placeholder);
  const [imageStatus, setImageStatus] = useState<'loading' | 'loaded' | 'error'>('loading');
  const imageRef = useRef<HTMLImageElement>(null);
  const observerRef = useRef<IntersectionObserver | null>(null);

  // 生成优化后的图片URL
  const getOptimizedSrc = useCallback((originalSrc: string) => {
    // 如果是 data URL 或 SVG，直接返回
    if (originalSrc.startsWith('data:') || originalSrc.endsWith('.svg')) {
      return originalSrc;
    }

    // 尝试使用图片服务进行压缩
    // 这里假设后端有图片处理服务，可以根据实际情况调整
    const url = new URL(originalSrc, window.location.origin);
    
    // 添加质量参数
    if (quality < 100) {
      url.searchParams.set('q', quality.toString());
    }

    // 添加格式参数
    if (format !== 'auto') {
      url.searchParams.set('f', format);
    }

    // 添加尺寸参数（如果指定）
    if (width && typeof width === 'number') {
      url.searchParams.set('w', width.toString());
    }
    if (height && typeof height === 'number') {
      url.searchParams.set('h', height.toString());
    }

    return url.toString();
  }, [src, quality, format, width, height]);

  // 加载图片
  const loadImage = useCallback(() => {
    const optimizedSrc = getOptimizedSrc(src);
    
    const img = new Image();
    img.src = optimizedSrc;

    img.onload = () => {
      setImageSrc(optimizedSrc);
      setImageStatus('loaded');
      onLoad?.();
    };

    img.onerror = () => {
      setImageSrc(errorImage);
      setImageStatus('error');
      onError?.(new Error(`Failed to load image: ${src}`));
    };
  }, [src, getOptimizedSrc, onLoad, onError, errorImage]);

  // 设置 IntersectionObserver
  useEffect(() => {
    const imageElement = imageRef.current;
    if (!imageElement) return;

    observerRef.current = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            loadImage();
            observerRef.current?.unobserve(imageElement);
          }
        });
      },
      {
        threshold,
        rootMargin,
      }
    );

    observerRef.current.observe(imageElement);

    return () => {
      observerRef.current?.unobserve(imageElement);
    };
  }, [loadImage, threshold, rootMargin]);

  return (
    <img
      ref={imageRef}
      src={imageSrc}
      alt={alt}
      width={width}
      height={height}
      className={`lazy-image ${imageStatus} ${className}`}
      style={{
        ...style,
        opacity: imageStatus === 'loaded' ? 1 : 0.5,
        transition: 'opacity 0.3s ease',
      }}
      loading="lazy"
    />
  );
}

export default LazyImage;
