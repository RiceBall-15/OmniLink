import { useEffect, useCallback } from 'react'
import './RecallConfirmDialog.css'

/**
 * 撤回确认对话框组件属性
 */
interface RecallConfirmDialogProps {
  /** 是否显示对话框 */
  visible: boolean
  /** 关闭对话框回调 */
  onClose: () => void
  /** 确认撤回回调 */
  onConfirm: () => void
  /** 是否正在撤回 */
  isRecalling?: boolean
  /** 撤回时间限制（分钟） */
  recallTimeLimit?: number
}

/**
 * 撤回确认对话框组件
 * 显示确认提示，用户可以确认或取消撤回消息
 */
export function RecallConfirmDialog({
  visible,
  onClose,
  onConfirm,
  isRecalling = false,
  recallTimeLimit = 2,
}: RecallConfirmDialogProps) {
  /**
   * 处理 Esc 键关闭对话框
   */
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === 'Escape' && visible) {
        onClose()
      }
    },
    [visible, onClose]
  )

  /**
   * 处理点击外部关闭对话框
   */
  const handleOverlayClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) {
      onClose()
    }
  }

  /**
   * 注册和注销键盘事件监听
   */
  useEffect(() => {
    if (visible) {
      document.addEventListener('keydown', handleKeyDown)
      return () => {
        document.removeEventListener('keydown', handleKeyDown)
      }
    }
  }, [visible, handleKeyDown])

  /**
   * 处理确认撤回
   */
  const handleConfirm = () => {
    if (!isRecalling) {
      onConfirm()
    }
  }

  /**
   * 处理取消
   */
  const handleCancel = () => {
    if (!isRecalling) {
      onClose()
    }
  }

  if (!visible) {
    return null
  }

  return (
    <div
      className="recall-confirm-overlay"
      onClick={handleOverlayClick}
      role="dialog"
      aria-modal="true"
      aria-labelledby="recall-confirm-title"
    >
      <div className="recall-confirm-container">
        {/* 头部 */}
        <div className="recall-confirm-header">
          <h2 id="recall-confirm-title" className="recall-confirm-title">
            ⚠️ 撤回消息
          </h2>
        </div>

        {/* 内容 */}
        <div className="recall-confirm-body">
          <p className="recall-confirm-message">确定要撤回这条消息吗？</p>

          {/* 警告信息 */}
          <div className="recall-confirm-warning">
            撤回后，此消息将从对话中移除，所有用户都将无法查看。
          </div>

          {/* 提示信息 */}
          <div className="recall-confirm-info">
            <span className="recall-confirm-icon">ℹ️</span>
            <span className="recall-confirm-text">
              只能在消息发送后 {recallTimeLimit} 分钟内撤回
            </span>
          </div>
        </div>

        {/* 底部按钮 */}
        <div className="recall-confirm-footer">
          <button
            className="recall-confirm-btn recall-confirm-btn-cancel"
            onClick={handleCancel}
            disabled={isRecalling}
          >
            取消
          </button>
          <button
            className="recall-confirm-btn recall-confirm-btn-confirm"
            onClick={handleConfirm}
            disabled={isRecalling}
          >
            {isRecalling ? (
              <>
                <span className="recall-confirm-spinner"></span>
                <span>撤回中...</span>
              </>
            ) : (
              '确认撤回'
            )}
          </button>
        </div>
      </div>
    </div>
  )
}
