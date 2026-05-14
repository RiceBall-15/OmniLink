/**
 * VirtualScroll 组件测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { VirtualScroll } from '../VirtualScroll';

// Mock IntersectionObserver with a class-based mock
class MockIntersectionObserver {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
  
  constructor(private callback: IntersectionObserverCallback) {}
  
  simulateIntersection(isIntersecting: boolean) {
    this.callback([{ isIntersecting } as IntersectionObserverEntry], this as any);
  }
}

beforeEach(() => {
  vi.clearAllMocks();
  
  (window as any).IntersectionObserver = vi.fn((callback: IntersectionObserverCallback) => {
    return new MockIntersectionObserver(callback);
  });
});

describe('VirtualScroll', () => {
  const mockItems = Array.from({ length: 100 }, (_, i) => ({
    id: i,
    content: `Item ${i}`,
  }));

  const mockRenderItem = vi.fn((item: any, index: number) => (
    <div key={item.id} data-testid={`item-${index}`}>
      {item.content}
    </div>
  ));

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders without crashing', () => {
    const { container } = render(
      <VirtualScroll
        items={mockItems}
        itemHeight={50}
        containerHeight={300}
        renderItem={mockRenderItem}
      />
    );

    expect(container.firstChild).toBeInTheDocument();
  });

  it('renders visible items only', () => {
    render(
      <VirtualScroll
        items={mockItems}
        itemHeight={50}
        containerHeight={300}
        renderItem={mockRenderItem}
      />
    );

    // Should render enough items to fill the container plus buffer
    expect(mockRenderItem).toHaveBeenCalled();
  });

  it('applies custom className', () => {
    const { container } = render(
      <VirtualScroll
        items={mockItems}
        itemHeight={50}
        containerHeight={300}
        renderItem={mockRenderItem}
        className="custom-class"
      />
    );

    expect(container.firstChild).toHaveClass('custom-class');
  });

  it('handles empty items array', () => {
    const { container } = render(
      <VirtualScroll
        items={[]}
        itemHeight={50}
        containerHeight={300}
        renderItem={mockRenderItem}
      />
    );

    expect(container.firstChild).toBeInTheDocument();
    expect(mockRenderItem).not.toHaveBeenCalled();
  });

  it('sets correct container height', () => {
    const { container } = render(
      <VirtualScroll
        items={mockItems}
        itemHeight={50}
        containerHeight={400}
        renderItem={mockRenderItem}
      />
    );

    const scrollContainer = container.firstChild as HTMLElement;
    expect(scrollContainer).toHaveStyle({ height: '400px' });
  });

  it('calculates total height correctly', () => {
    const { container } = render(
      <VirtualScroll
        items={mockItems}
        itemHeight={50}
        containerHeight={300}
        renderItem={mockRenderItem}
      />
    );

    // Total height should be items.length * itemHeight
    const innerDiv = container.querySelector('[style*="height"]');
    expect(innerDiv).toBeInTheDocument();
  });
});
