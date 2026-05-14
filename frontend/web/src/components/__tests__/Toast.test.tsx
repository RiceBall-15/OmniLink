/**
 * Toast 组件测试
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen, act } from '@testing-library/react';
import Toast from '../components/Toast';

describe('Toast Component', () => {
  it('renders toast message', () => {
    render(<Toast message="测试消息" type="success" onClose={vi.fn()} />);
    expect(screen.getByText('测试消息')).toBeInTheDocument();
  });

  it('renders success type', () => {
    render(<Toast message="成功" type="success" onClose={vi.fn()} />);
    const toast = screen.getByText('成功').closest('div');
    expect(toast).toBeTruthy();
  });

  it('renders error type', () => {
    render(<Toast message="错误" type="error" onClose={vi.fn()} />);
    expect(screen.getByText('错误')).toBeInTheDocument();
  });

  it('renders warning type', () => {
    render(<Toast message="警告" type="warning" onClose={vi.fn()} />);
    expect(screen.getByText('警告')).toBeInTheDocument();
  });

  it('renders info type', () => {
    render(<Toast message="信息" type="info" onClose={vi.fn()} />);
    expect(screen.getByText('信息')).toBeInTheDocument();
  });

  it('calls onClose after timeout', async () => {
    vi.useFakeTimers();
    const onClose = vi.fn();
    render(<Toast message="自动关闭" type="success" onClose={onClose} />);
    
    await act(async () => {
      vi.advanceTimersByTime(3000);
    });
    
    expect(onClose).toHaveBeenCalled();
    vi.useRealTimers();
  });
});
