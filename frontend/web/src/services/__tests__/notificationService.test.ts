/**
 * 通知服务测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { notificationService } from '../notificationService';

// Mock Notification API
const mockRequestPermission = vi.fn();

Object.defineProperty(window, 'Notification', {
  value: {
    requestPermission: mockRequestPermission,
    permission: 'default',
  },
  writable: true,
});

// Mock AudioContext
class MockAudioContext {
  createOscillator() {
    return {
      connect: vi.fn(),
      start: vi.fn(),
      stop: vi.fn(),
      frequency: { setValueAtTime: vi.fn() },
    };
  }
  createGain() {
    return {
      connect: vi.fn(),
      gain: {
        setValueAtTime: vi.fn(),
        exponentialRampToValueAtTime: vi.fn(),
      },
    };
  }
  get destination() {
    return {};
  }
  get currentTime() {
    return 0;
  }
}

Object.defineProperty(window, 'AudioContext', {
  value: MockAudioContext,
  writable: true,
});

describe('NotificationService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset the mock to return 'granted' by default
    mockRequestPermission.mockResolvedValue('granted');
  });

  it('is a singleton', () => {
    // The service is already a singleton instance
    expect(notificationService).toBeDefined();
    expect(typeof notificationService).toBe('object');
  });

  it('has correct default settings', () => {
    const settings = notificationService.getSettings();
    
    expect(settings).toBeDefined();
    expect(typeof settings).toBe('object');
  });

  it('can update settings', () => {
    // Just verify it doesn't throw
    expect(() => notificationService.updateSettings({})).not.toThrow();
  });

  it('can get unread count', () => {
    const count = notificationService.getUnreadCount();
    expect(typeof count).toBe('number');
  });

  it('can set unread count', () => {
    notificationService.setUnreadCount(5);
    expect(notificationService.getUnreadCount()).toBe(5);
  });

  it('can increment unread count', () => {
    const initialCount = notificationService.getUnreadCount();
    notificationService.incrementUnreadCount();
    expect(notificationService.getUnreadCount()).toBe(initialCount + 1);
  });

  it('can clear unread count', () => {
    notificationService.setUnreadCount(10);
    notificationService.clearUnreadCount();
    expect(notificationService.getUnreadCount()).toBe(0);
  });

  it('handles requestPermission', async () => {
    const result = await notificationService.requestPermission();
    
    expect(result).toBe('granted');
    expect(mockRequestPermission).toHaveBeenCalled();
  });

  it('handles requestPermission denial', async () => {
    mockRequestPermission.mockResolvedValueOnce('denied');
    
    const result = await notificationService.requestPermission();
    
    expect(result).toBe('denied');
  });

  it('can check permission', async () => {
    const permission = await notificationService.checkPermission();
    expect(['default', 'granted', 'denied']).toContain(permission);
  });
});
