/**
 * 无障碍访问工具测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  generateAriaProps,
  getAriaLabel,
  getKeyboardHandler,
  handleKeyboardNavigation,
  createFocusTrap,
  announceToScreenReader,
  addSkipLink,
  prefersReducedMotion,
  prefersHighContrast,
  prefersDarkMode,
} from '../accessibility';

// Mock DOM methods
const mockAddEventListener = vi.fn();
const mockRemoveEventListener = vi.fn();
const mockAppendChild = vi.fn();
const mockRemove = vi.fn();
const mockQuerySelector = vi.fn();
const mockGetAttribute = vi.fn();

Object.defineProperty(document, 'addEventListener', { value: mockAddEventListener });
Object.defineProperty(document, 'removeEventListener', { value: mockRemoveEventListener });
Object.defineProperty(document.body, 'appendChild', { value: mockAppendChild });
Object.defineProperty(document, 'querySelector', { value: mockQuerySelector });

describe('Accessibility Utils', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('generateAriaProps', () => {
    it('generates correct aria props for button', () => {
      const props = generateAriaProps('button', { label: 'Submit', disabled: true });
      
      expect(props).toEqual({
        'aria-label': 'Submit',
        'aria-disabled': 'true',
        role: 'button',
      });
    });

    it('generates correct aria props for dialog', () => {
      const props = generateAriaProps('dialog', { label: 'Settings', expanded: true });
      
      expect(props).toEqual({
        'aria-label': 'Settings',
        'aria-expanded': 'true',
        role: 'dialog',
      });
    });

    it('generates correct aria props for checkbox', () => {
      const props = generateAriaProps('checkbox', { label: 'Accept terms', checked: true });
      
      expect(props).toEqual({
        'aria-label': 'Accept terms',
        'aria-checked': 'true',
        role: 'checkbox',
      });
    });

    it('generates correct aria props for tab', () => {
      const props = generateAriaProps('tab', { label: 'Tab 1', selected: true });
      
      expect(props).toEqual({
        'aria-label': 'Tab 1',
        'aria-selected': 'true',
        role: 'tab',
      });
    });
  });

  describe('getAriaLabel', () => {
    it('returns label when provided', () => {
      const label = getAriaLabel({ label: 'Custom Label' });
      expect(label).toBe('Custom Label');
    });

    it('returns undefined when no label', () => {
      const label = getAriaLabel({});
      expect(label).toBeUndefined();
    });
  });

  describe('getKeyboardHandler', () => {
    it('returns a function', () => {
      const handler = getKeyboardHandler('button', vi.fn());
      expect(typeof handler).toBe('function');
    });

    it('calls callback on Enter key', () => {
      const callback = vi.fn();
      const handler = getKeyboardHandler('button', callback);
      
      const event = new KeyboardEvent('keydown', { key: 'Enter' });
      handler(event);
      
      expect(callback).toHaveBeenCalled();
    });

    it('calls callback on Space key for button', () => {
      const callback = vi.fn();
      const handler = getKeyboardHandler('button', callback);
      
      const event = new KeyboardEvent('keydown', { key: ' ' });
      handler(event);
      
      expect(callback).toHaveBeenCalled();
    });
  });

  describe('handleKeyboardNavigation', () => {
    it('returns a function', () => {
      const handler = handleKeyboardNavigation([]);
      expect(typeof handler).toBe('function');
    });
  });

  describe('createFocusTrap', () => {
    it('returns activate and deactivate functions', () => {
      const container = document.createElement('div');
      const trap = createFocusTrap(container);
      
      expect(trap).toHaveProperty('activate');
      expect(trap).toHaveProperty('deactivate');
      expect(typeof trap.activate).toBe('function');
      expect(typeof trap.deactivate).toBe('function');
    });
  });

  describe('announceToScreenReader', () => {
    it('creates live region element', () => {
      announceToScreenReader('Test message');
      
      expect(mockAppendChild).toHaveBeenCalled();
    });

    it('creates polite live region by default', () => {
      announceToScreenReader('Test message');
      
      const element = mockAppendChild.mock.calls[0][0];
      expect(element.getAttribute('aria-live')).toBe('polite');
    });

    it('creates assertive live region when specified', () => {
      announceToScreenReader('Urgent message', 'assertive');
      
      const element = mockAppendChild.mock.calls[0][0];
      expect(element.getAttribute('aria-live')).toBe('assertive');
    });
  });

  describe('addSkipLink', () => {
    it('adds skip link to document', () => {
      addSkipLink('#main-content', 'Skip to main content');
      
      expect(mockAppendChild).toHaveBeenCalled();
    });

    it('uses default text when not provided', () => {
      addSkipLink('#main-content');
      
      const element = mockAppendChild.mock.calls[0][0];
      expect(element.textContent).toBe('跳到主要内容');
    });
  });

  describe('prefersReducedMotion', () => {
    it('returns boolean', () => {
      const result = prefersReducedMotion();
      expect(typeof result).toBe('boolean');
    });
  });

  describe('prefersHighContrast', () => {
    it('returns boolean', () => {
      const result = prefersHighContrast();
      expect(typeof result).toBe('boolean');
    });
  });

  describe('prefersDarkMode', () => {
    it('returns boolean', () => {
      const result = prefersDarkMode();
      expect(typeof result).toBe('boolean');
    });
  });
});
