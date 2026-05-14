/**
 * MessageService 测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { messageService } from '../messageService';

// Mock the api module's default export
vi.mock('../api', () => ({
  default: vi.fn(),
  ApiResponse: {},
}));

import request from '../api';
const mockRequest = vi.mocked(request);

describe('MessageService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('getMessages', () => {
    it('calls correct endpoint with conversation_id', async () => {
      mockRequest.mockResolvedValue({
        success: true,
        data: [],
      });

      await messageService.getMessages('conv-123', 1, 20);

      expect(mockRequest).toHaveBeenCalledWith(
        expect.stringContaining('/api/im/conversations/conv-123/messages')
      );
    });
  });

  describe('sendMessage', () => {
    it('calls POST with correct params', async () => {
      mockRequest.mockResolvedValue({
        success: true,
        data: { id: 'msg-1' },
      });

      await messageService.sendMessage('conv-123', '你好');

      expect(mockRequest).toHaveBeenCalledWith(
        expect.stringContaining('/api/im/conversations/conv-123/messages'),
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({ content: '你好', type: 'text' }),
        })
      );
    });
  });

  describe('editMessage', () => {
    it('calls PUT with message ID', async () => {
      mockRequest.mockResolvedValue({
        success: true,
        data: { id: 'msg-1', content: '已编辑' },
      });

      await messageService.editMessage('msg-1', '已编辑');

      expect(mockRequest).toHaveBeenCalledWith(
        expect.stringContaining('/api/im/messages/msg-1'),
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify({ content: '已编辑' }),
        })
      );
    });
  });

  describe('recallMessage', () => {
    it('calls PUT to recall endpoint', async () => {
      mockRequest.mockResolvedValue({
        success: true,
        data: {},
      });

      await messageService.recallMessage('msg-1');

      expect(mockRequest).toHaveBeenCalledWith(
        expect.stringContaining('/api/im/messages/msg-1/recall'),
        expect.objectContaining({ method: 'PUT' })
      );
    });
  });

  describe('markAsRead', () => {
    it('calls PUT to read endpoint', async () => {
      mockRequest.mockResolvedValue({
        success: true,
        data: undefined,
      });

      await messageService.markAsRead('conv-123');

      expect(mockRequest).toHaveBeenCalledWith(
        expect.stringContaining('/api/im/conversations/conv-123/read'),
        expect.objectContaining({ method: 'PUT' })
      );
    });
  });
});
