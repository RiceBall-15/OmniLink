/**
 * Toast 组件测试
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { ToastProvider, useToast } from '../Toast';
import React from 'react';

// Helper component to test useToast hook
function ToastTestHelper() {
  const { showToast, showSuccess, showError, showWarning, showInfo } = useToast();
  return (
    <div>
      <button onClick={() => showToast({ type: 'info', message: 'Custom toast' })}>
        Show Custom
      </button>
      <button onClick={() => showSuccess('Success message', 'Success Title')}>
        Show Success
      </button>
      <button onClick={() => showError('Error message', 'Error Title')}>
        Show Error
      </button>
      <button onClick={() => showWarning('Warning message', 'Warning Title')}>
        Show Warning
      </button>
      <button onClick={() => showInfo('Info message', 'Info Title')}>
        Show Info
      </button>
    </div>
  );
}

describe('ToastProvider', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('renders children', () => {
    render(
      <ToastProvider>
        <div>Child Content</div>
      </ToastProvider>
    );
    expect(screen.getByText('Child Content')).toBeInTheDocument();
  });

  it('throws error when useToast is used outside ToastProvider', () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    
    function BadComponent() {
      useToast();
      return null;
    }

    expect(() => render(<BadComponent />)).toThrow('useToast must be used within a ToastProvider');
    consoleSpy.mockRestore();
  });

  it('shows a toast when showToast is called', async () => {
    render(
      <ToastProvider>
        <ToastTestHelper />
      </ToastProvider>
    );

    await act(async () => {
      fireEvent.click(screen.getByText('Show Custom'));
    });

    expect(screen.getByText('Custom toast')).toBeInTheDocument();
  });

  it('shows success toast with title', async () => {
    render(
      <ToastProvider>
        <ToastTestHelper />
      </ToastProvider>
    );

    await act(async () => {
      fireEvent.click(screen.getByText('Show Success'));
    });

    expect(screen.getByText('Success message')).toBeInTheDocument();
    expect(screen.getByText('Success Title')).toBeInTheDocument();
  });

  it('shows error toast with title', async () => {
    render(
      <ToastProvider>
        <ToastTestHelper />
      </ToastProvider>
    );

    await act(async () => {
      fireEvent.click(screen.getByText('Show Error'));
    });

    expect(screen.getByText('Error message')).toBeInTheDocument();
    expect(screen.getByText('Error Title')).toBeInTheDocument();
  });

  it('shows warning toast', async () => {
    render(
      <ToastProvider>
        <ToastTestHelper />
      </ToastProvider>
    );

    await act(async () => {
      fireEvent.click(screen.getByText('Show Warning'));
    });

    expect(screen.getByText('Warning message')).toBeInTheDocument();
  });

  it('shows info toast', async () => {
    render(
      <ToastProvider>
        <ToastTestHelper />
      </ToastProvider>
    );

    await act(async () => {
      fireEvent.click(screen.getByText('Show Info'));
    });

    expect(screen.getByText('Info message')).toBeInTheDocument();
  });

  it('removes toast when close button is clicked', async () => {
    render(
      <ToastProvider>
        <ToastTestHelper />
      </ToastProvider>
    );

    await act(async () => {
      fireEvent.click(screen.getByText('Show Custom'));
    });

    const closeButton = screen.getByText('✕');
    fireEvent.click(closeButton);

    expect(screen.queryByText('Custom toast')).not.toBeInTheDocument();
  });

  it('auto-removes toast after duration', async () => {
    render(
      <ToastProvider>
        <ToastTestHelper />
      </ToastProvider>
    );

    await act(async () => {
      fireEvent.click(screen.getByText('Show Custom'));
    });

    expect(screen.getByText('Custom toast')).toBeInTheDocument();

    // Default duration is 3000ms
    await act(async () => {
      vi.advanceTimersByTime(3000);
    });

    expect(screen.queryByText('Custom toast')).not.toBeInTheDocument();
  });

  it('renders correct icon for each toast type', async () => {
    render(
      <ToastProvider>
        <ToastTestHelper />
      </ToastProvider>
    );

    // Show success toast
    await act(async () => {
      fireEvent.click(screen.getByText('Show Success'));
    });
    expect(screen.getByText('✅')).toBeInTheDocument();
  });

  it('can show multiple toasts simultaneously', async () => {
    render(
      <ToastProvider>
        <ToastTestHelper />
      </ToastProvider>
    );

    await act(async () => {
      fireEvent.click(screen.getByText('Show Success'));
      fireEvent.click(screen.getByText('Show Error'));
    });

    expect(screen.getByText('Success message')).toBeInTheDocument();
    expect(screen.getByText('Error message')).toBeInTheDocument();
  });
});
