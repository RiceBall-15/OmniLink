/**
 * LazyImage 组件测试
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import LazyImage from '../LazyImage';

// Mock IntersectionObserver with a class-based mock
class MockIntersectionObserver {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
  callback: IntersectionObserverCallback;
  
  constructor(callback: IntersectionObserverCallback) {
    this.callback = callback;
  }
  
  // Helper to simulate intersection
  simulateIntersection(isIntersecting: boolean) {
    this.callback([{ isIntersecting } as IntersectionObserverEntry], this as any);
  }
}

let mockObserver: MockIntersectionObserver;
let originalIntersectionObserver: typeof window.IntersectionObserver;

beforeEach(() => {
  vi.clearAllMocks();
  
  // Save original
  originalIntersectionObserver = window.IntersectionObserver;
  
  // Store reference to mock observer for test access
  // Use a function constructor (not arrow) so `new` works
  const MockIO = vi.fn().mockImplementation(function (callback: IntersectionObserverCallback) {
    mockObserver = new MockIntersectionObserver(callback);
    return mockObserver;
  });
  (window as any).IntersectionObserver = MockIO;
});

afterEach(() => {
  // Restore original
  window.IntersectionObserver = originalIntersectionObserver;
});

describe('LazyImage', () => {
  const defaultProps = {
    src: 'https://example.com/image.jpg',
    alt: 'Test image',
  };

  it('renders placeholder initially', () => {
    const { container } = render(<LazyImage {...defaultProps} />);
    
    // Should render a wrapper div
    expect(container.firstChild).toBeInTheDocument();
  });

  it('sets up IntersectionObserver', () => {
    render(<LazyImage {...defaultProps} />);
    
    expect(window.IntersectionObserver).toHaveBeenCalled();
    expect(mockObserver.observe).toHaveBeenCalled();
  });

  it('applies custom className', () => {
    const { container } = render(
      <LazyImage {...defaultProps} className="custom-class" />
    );
    
    expect(container.firstChild).toHaveClass('custom-class');
  });

  it('applies custom dimensions', () => {
    const { container } = render(
      <LazyImage {...defaultProps} width={200} height={150} />
    );
    
    const wrapper = container.firstChild as HTMLElement;
    expect(wrapper).toHaveStyle({ width: '200px', height: '150px' });
  });

  it('unobserve on unmount', () => {
    const { unmount } = render(<LazyImage {...defaultProps} />);
    
    unmount();
    
    expect(mockObserver.unobserve).toHaveBeenCalled();
  });
});
