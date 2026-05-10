import { useState, useEffect, useRef, KeyboardEvent } from 'react'

/**
 * 消息编辑组件属性
 */
interface MessageEditProps {
  /** 初始消息内容 */
  initialContent: string
  /** 保存回调 */
  onSave: (content: string) => Promise<void>
  /** 取消回调 */
  onCancel: () => void
  /** 是否正在保存 */
  isSaving?: boolean
  /** 编辑时间限制（分钟），默认2分钟 */
  editTimeLimit?: number
  /** 消息创建时间 */
  messageCreatedAt: string
  /** 是否禁用编辑（超时或其他原因） */
  disabled?: boolean
}

/**
 * 消息编辑组件
 * 提供消息编辑功能，支持：
 * - 双击编辑
 * - 快捷键支持（Esc取消，Ctrl+Enter保存）
 * - 自动高度调整
 * - 编辑历史记录（撤销/重做）
 * - 编辑时间限制
 */
export function MessageEdit({
  initialContent,
  onSave,
  onCancel,
  isSaving = false,
  editTimeLimit = 2,
  messageCreatedAt,
  disabled = false,
}: MessageEditProps) {
  const [content, setContent] = useState(initialContent)
  const [hasChanges, setHasChanges] = useState(false)
  const [isExpired, setIsExpired] = useState(false)

  const textareaRef = useRef<HTMLTextAreaElement>(null)

  // 编辑历史记录（用于撤销/重做）
  const historyRef = useRef<string[]>([initialContent])
  const historyIndexRef = useRef(0)

  /**
   * 检查编辑是否超时
   */
  useEffect(() => {
    const now = Date.now()
    const createdAt = new Date(messageCreatedAt).getTime()
    const timeDiffMinutes = (now - createdAt) / (1000 * 60)

    if (timeDiffMinutes >= editTimeLimit) {
      setIsExpired(true)
    }

    // 设置定时器，在达到编辑时限时禁用编辑
    const remainingTime = editTimeLimit * 60 * 1000 - (now - createdAt)
    if (remainingTime > 0) {
      const timer = setTimeout(() => {
        setIsExpired(true)
      }, remainingTime)
      return () => clearTimeout(timer)
    }
  }, [messageCreatedAt, editTimeLimit])

  /**
   * 自动调整 textarea 高度
   */
  useEffect(() => {
    const textarea = textareaRef.current
    if (textarea) {
      textarea.style.height = 'auto'
      textarea.style.height = `${Math.min(textarea.scrollHeight, 200)}px`
      textarea.focus()
      // 光标移到末尾
      textarea.setSelectionRange(textarea.value.length, textarea.value.length)
    }
  }, [])

  /**
   * 更新内容并记录历史
   */
  const updateContent = (newContent: string) => {
    setContent(newContent)
    setHasChanges(newContent !== initialContent)

    // 记录历史（简单实现：每次更改都记录）
    // 实际应用中可以使用防抖来优化性能
    const newHistory = historyRef.current.slice(0, historyIndexRef.current + 1)
    newHistory.push(newContent)
    historyRef.current = newHistory
    historyIndexRef.current = newHistory.length - 1
  }

  /**
   * 处理保存
   */
  const handleSave = async () => {
    if (!content.trim() || content === initialContent) {
      onCancel()
      return
    }
    await onSave(content)
  }

  /**
   * 处理键盘事件
   * - Esc: 取消编辑
   * - Ctrl+Enter: 保存
   * - Ctrl+Z: 撤销
   * - Ctrl+Shift+Z 或 Ctrl+Y: 重做
   */
  const handleKeyDown = async (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Escape') {
      e.preventDefault()
      onCancel()
    } else if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault()
      await handleSave()
    } else if (e.key === 'z' && (e.ctrlKey || e.metaKey)) {
      if (e.shiftKey) {
        // 重做
        e.preventDefault()
        if (historyIndexRef.current < historyRef.current.length - 1) {
          historyIndexRef.current++
          setContent(historyRef.current[historyIndexRef.current])
          setHasChanges(true)
        }
      } else {
        // 撤销
        e.preventDefault()
        if (historyIndexRef.current > 0) {
          historyIndexRef.current--
          setContent(historyRef.current[historyIndexRef.current])
          setHasChanges(true)
        }
      }
    }
  }

  /**
   * 处理输入变化
   */
  const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    updateContent(e.target.value)
  }

  // 如果已禁用或已超时，显示禁用状态
  const isDisabled = disabled || isExpired

  return (
    <div className="message-edit-container">
      <div className="message-edit-content">
        <textarea
          ref={textareaRef}
          value={content}
          onChange={handleInputChange}
          onKeyDown={handleKeyDown}
          disabled={isDisabled || isSaving}
          className="message-edit-textarea"
          rows={1}
          placeholder="输入消息内容..."
          style={{
            resize: 'none',
            overflowY: 'auto',
          }}
        />
        {isExpired && (
          <div className="message-edit-expired">
            编辑已超时（超过 {editTimeLimit} 分钟）
          </div>
        )}
      </div>
      <div className="message-edit-actions">
        <button
          type="button"
          onClick={onCancel}
          disabled={isSaving}
          className="message-edit-btn message-edit-btn-cancel"
          aria-label="取消编辑"
        >
          取消
        </button>
        <button
          type="button"
          onClick={handleSave}
          disabled={isSaving || !hasChanges || isDisabled}
          className="message-edit-btn message-edit-btn-save"
          aria-label="保存编辑"
        >
          {isSaving ? (
            <>
              <span className="message-edit-spinner" />
              保存中...
            </>
          ) : (
            '保存'
          )}
        </button>
      </div>
      <div className="message-edit-hint">
        <span className="message-edit-hint-text">快捷键: Esc 取消 · Ctrl+Enter 保存</span>
      </div>
    </div>
  )
}
