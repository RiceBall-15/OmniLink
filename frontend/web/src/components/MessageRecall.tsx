import { useState, useEffect, useCallback } from 'react'
import { messageService } from '../services/messageService'
import './MessageRecall.css'

/**
 * 消息撤回组件属性
 */
interface MessageRecallProps {
  /** 消息ID */
  messageId: string
  /** 消息创建时间 */
  messageCreatedAt: string
  /** 当前用户ID */
  currentUserId: string
  /** 消息发送者ID */
  senderId: string
  /** 撤回时间限制（分钟） */
  recallTimeLimit?: number
  /** 撤回成功回调 */
  onRecallSuccess?: (messageId: string) => void
  /** 撤回失败回调 */
  onRecallError?: (error: Error) => void
}

/**
 * 消息撤回组件
 * 提供消息撤回功能的独立组件
 */
export function MessageRecall({
  messageId,
  messageCreatedAt,
  currentUserId,
  senderId,
  recallTimeLimit = 2,
  onRecallSuccess,
  onRecallError,
}: MessageRecallProps) {
  const [isRecalling, setIsRecalling] = useState(false)
  const [canRecall, setCanRecall] = useState(false)
  const [showConfirm, setShowConfirm] = useState(false)
  const [timeRemaining, setTimeRemaining] = useState<number | null>(null)

  /**
   * 检查是否可以撤回消息
   */
  const checkCanRecall = useCallback(() => {
    // 只能撤回自己的消息
    if (currentUserId !== senderId) {
      setCanRecall(false)
      setTimeRemaining(null)
      return
    }

    const now = Date.now()
    const createdAt = new Date(messageCreatedAt).getTime()
    const elapsedMs = now - createdAt
    const limitMs = recallTimeLimit * 60 * 1000
    const remainingMs = limitMs - elapsedMs

    if (remainingMs <= 0) {
      setCanRecall(false)
      setTimeRemaining(null)
    } else {
      setCanRecall(true)
      setTimeRemaining(Math.floor(remainingMs / 1000))
    }
  }, [messageCreatedAt, recallTimeLimit, currentUserId, senderId])

  /**
   * 更新剩余时间
   */
  useEffect(() => {
    // 初始检查
    checkCanRecall()

    // 每秒更新一次剩余时间
    const interval = setInterval(() => {
      checkCanRecall()
    }, 1000)

    return () => clearInterval(interval)
  }, [checkCanRecall])

  /**
   * 格式化剩余时间
   */
  const formatTimeRemaining = useCallback((seconds: number) => {
    if (seconds < 60) {
      return `${seconds}秒`
    }
    const minutes = Math.floor(seconds / 60)
    const remainingSeconds = seconds % 60
    if (remainingSeconds > 0) {
      return `${minutes}分${remainingSeconds}秒`
    }
    return `${minutes}分钟`
  }, [])

  /**
   * 处理撤回按钮点击
   */
  const handleRecallClick = useCallback(() => {
    if (!canRecall) return
    setShowConfirm(true)
  }, [canRecall])

  /**
   * 确认撤回
   */
  const handleConfirmRecall = useCallback(async () => {
    if (!canRecall || isRecalling) return

    setIsRecalling(true)
    try {
      await messageService.recallMessage(messageId)
      onRecallSuccess?.(messageId)
      setShowConfirm(false)
    } catch (error) {
      const err = error instanceof Error ? error : new Error('撤回消息失败')
      onRecallError?.(err)
    } finally {
      setIsRecalling(false)
    }
  }, [messageId, canRecall, isRecalling, onRecallSuccess, onRecallError])

  /**
   * 取消撤回
   */
  const handleCancelRecall = useCallback(() => {
    setShowConfirm(false)
  }, [])

  // 如果不能撤回，不渲染任何内容
  if (!canRecall) {
    return null
  }

  return (
    <div className="message-recall">
      {/* 撤回按钮 */}
      <button
        className="message-recall-btn"
        onClick={handleRecallClick}
        disabled={isRecalling}
        title={timeRemaining ? `${formatTimeRemaining(timeRemaining)}内可撤回` : undefined}
      >
        {isRecalling ? (
          <>
            <span className="message-recall-spinner" />
            撤回中...
          </>
        ) : (
          <>
            <span className="message-recall-icon">↩️</span>
            撤回
          </>
        )}
      </button>

      {/* 剩余时间提示 */}
      {timeRemaining !== null && (
        <div className="message-recall-time-hint">
          {formatTimeRemaining(timeRemaining)}后不可撤回
        </div>
      )}

      {/* 确认对话框 */}
      {showConfirm && (
        <div className="message-recall-confirm-overlay" onClick={handleCancelRecall}>
          <div
            className="message-recall-confirm-dialog"
            onClick={(e) => e.stopPropagation()}
            role="dialog"
            aria-modal="true"
            aria-labelledby="recall-dialog-title"
          >
            <div className="message-recall-confirm-header">
              <h3 id="recall-dialog-title" className="message-recall-confirm-title">
                确认撤回消息
              </h3>
            </div>

            <div className="message-recall-confirm-body">
              <p className="message-recall-confirm-message">
                确定要撤回这条消息吗？
              </p>
              <p className="message-recall-confirm-warning">
                撤回后，消息将被隐藏，对方将无法看到消息内容。此操作不可恢复。
              </p>
              <div className="message-recall-confirm-info">
                <span className="message-recall-confirm-info-icon">⏱️</span>
                <span>仅在消息发送后 {recallTimeLimit} 分钟内可以撤回</span>
              </div>
            </div>

            <div className="message-recall-confirm-footer">
              <button
                className="message-recall-confirm-btn message-recall-confirm-btn-cancel"
                onClick={handleCancelRecall}
                disabled={isRecalling}
              >
                取消
              </button>
              <button
                className="message-recall-confirm-btn message-recall-confirm-btn-confirm"
                onClick={handleConfirmRecall}
                disabled={isRecalling}
              >
                {isRecalling ? (
                  <>
                    <span className="message-recall-spinner" />
                    撤回中...
                  </>
                ) : (
                  '确认撤回'
                )}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
