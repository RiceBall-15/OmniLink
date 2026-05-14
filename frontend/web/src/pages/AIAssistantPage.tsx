import React, { useState, useEffect, useRef, useCallback } from 'react'
import { aiService, AIAssistant, ChatMessage, ModelInfo } from '../services/aiService'
import './AIAssistantPage.css'

// ============================================================
// 子组件：助手选择器
// ============================================================

function AssistantSelector({
  assistants,
  selectedId,
  onSelect,
  onCreate,
}: {
  assistants: AIAssistant[]
  selectedId?: string
  onSelect: (id: string | undefined) => void
  onCreate: () => void
}) {
  return (
    <div className="assistant-selector">
      <div className="assistant-selector__header">
        <h3>AI 助手</h3>
        <button className="btn btn--small btn--primary" onClick={onCreate}>
          + 新建
        </button>
      </div>

      <div className="assistant-list">
        <button
          className={`assistant-item ${!selectedId ? 'active' : ''}`}
          onClick={() => onSelect(undefined)}
        >
          <span className="assistant-item__avatar">🤖</span>
          <div className="assistant-item__info">
            <div className="assistant-item__name">默认助手</div>
            <div className="assistant-item__desc">通用 AI 助手</div>
          </div>
        </button>

        {assistants.map((a) => (
          <button
            key={a.id}
            className={`assistant-item ${selectedId === a.id ? 'active' : ''}`}
            onClick={() => onSelect(a.id)}
          >
            <span className="assistant-item__avatar">{a.avatar || '🧠'}</span>
            <div className="assistant-item__info">
              <div className="assistant-item__name">{a.name}</div>
              <div className="assistant-item__desc">{a.description || a.model}</div>
            </div>
          </button>
        ))}
      </div>
    </div>
  )
}

// ============================================================
// 子组件：聊天消息
// ============================================================

function ChatMessageBubble({ message }: { message: ChatMessage }) {
  return (
    <div className={`chat-message chat-message--${message.role}`}>
      <div className="chat-message__avatar">
        {message.role === 'user' ? '👤' : '🤖'}
      </div>
      <div className="chat-message__content">
        <div className="chat-message__text">{message.content}</div>
        {message.timestamp && (
          <div className="chat-message__time">
            {new Date(message.timestamp).toLocaleTimeString()}
          </div>
        )}
      </div>
    </div>
  )
}

// ============================================================
// 子组件：助手编辑对话框
// ============================================================

function AssistantDialog({
  assistant,
  onSave,
  onClose,
}: {
  assistant?: AIAssistant
  onSave: (data: any) => void
  onClose: () => void
}) {
  const [name, setName] = useState(assistant?.name || '')
  const [description, setDescription] = useState(assistant?.description || '')
  const [systemPrompt, setSystemPrompt] = useState(assistant?.system_prompt || '')
  const [model, setModel] = useState(assistant?.model || 'gpt-3.5-turbo')
  const [temperature, setTemperature] = useState(assistant?.temperature ?? 0.7)
  const [maxTokens, setMaxTokens] = useState(assistant?.max_tokens ?? 2048)

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    onSave({
      name,
      description,
      system_prompt: systemPrompt,
      model,
      temperature,
      max_tokens: maxTokens,
    })
  }

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog" onClick={(e) => e.stopPropagation()}>
        <div className="dialog__header">
          <h3>{assistant ? '编辑助手' : '创建助手'}</h3>
          <button className="dialog__close" onClick={onClose}>✕</button>
        </div>

        <form onSubmit={handleSubmit} className="dialog__body">
          <div className="form-group">
            <label>名称 *</label>
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="助手名称"
              required
            />
          </div>

          <div className="form-group">
            <label>描述</label>
            <input
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="助手描述"
            />
          </div>

          <div className="form-group">
            <label>系统提示词 *</label>
            <textarea
              value={systemPrompt}
              onChange={(e) => setSystemPrompt(e.target.value)}
              placeholder="设定助手的行为和角色..."
              rows={4}
              required
            />
          </div>

          <div className="form-row">
            <div className="form-group">
              <label>模型</label>
              <input
                value={model}
                onChange={(e) => setModel(e.target.value)}
                placeholder="gpt-3.5-turbo"
              />
            </div>

            <div className="form-group">
              <label>温度 ({temperature})</label>
              <input
                type="range"
                min="0"
                max="2"
                step="0.1"
                value={temperature}
                onChange={(e) => setTemperature(parseFloat(e.target.value))}
              />
            </div>

            <div className="form-group">
              <label>最大 Token</label>
              <input
                type="number"
                value={maxTokens}
                onChange={(e) => setMaxTokens(parseInt(e.target.value))}
                min={1}
                max={128000}
              />
            </div>
          </div>

          <div className="dialog__actions">
            <button type="button" className="btn btn--secondary" onClick={onClose}>
              取消
            </button>
            <button type="submit" className="btn btn--primary">
              {assistant ? '保存' : '创建'}
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}

// ============================================================
// 主页面组件
// ============================================================

export default function AIAssistantPage() {
  // 状态
  const [assistants, setAssistants] = useState<AIAssistant[]>([])
  const [selectedAssistantId, setSelectedAssistantId] = useState<string | undefined>()
  const [messages, setMessages] = useState<ChatMessage[]>([])
  const [inputText, setInputText] = useState('')
  const [isStreaming, setIsStreaming] = useState(false)
  const [streamingContent, setStreamingContent] = useState('')
  const [models, setModels] = useState<ModelInfo[]>([])
  const [showDialog, setShowDialog] = useState(false)
  const [editingAssistant, setEditingAssistant] = useState<AIAssistant | undefined>()
  const [error, setError] = useState<string | null>(null)

  const messagesEndRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLTextAreaElement>(null)
  const eventSourceRef = useRef<EventSource | null>(null)

  // 滚动到底部
  const scrollToBottom = useCallback(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [])

  // 加载助手列表
  useEffect(() => {
    const loadAssistants = async () => {
      try {
        const data = await aiService.listAssistants()
        setAssistants(data)
      } catch (err) {
        console.error('Failed to load assistants:', err)
      }
    }
    loadAssistants()
  }, [])

  // 加载模型列表
  useEffect(() => {
    const loadModels = async () => {
      try {
        const data = await aiService.listModels()
        setModels(data)
      } catch (err) {
        console.error('Failed to load models:', err)
      }
    }
    loadModels()
  }, [])

  // 消息更新时滚动
  useEffect(() => {
    scrollToBottom()
  }, [messages, streamingContent, scrollToBottom])

  // 清理 EventSource
  useEffect(() => {
    return () => {
      if (eventSourceRef.current) {
        eventSourceRef.current.close()
      }
    }
  }, [])

  // 发送消息（流式）
  const handleSend = async () => {
    const text = inputText.trim()
    if (!text || isStreaming) return

    const userMessage: ChatMessage = {
      role: 'user',
      content: text,
      timestamp: new Date().toISOString(),
    }

    setMessages((prev) => [...prev, userMessage])
    setInputText('')
    setIsStreaming(true)
    setStreamingContent('')
    setError(null)

    try {
      const request = {
        messages: [...messages, userMessage].map((m) => ({
          role: m.role,
          content: m.content,
        })),
        assistant_id: selectedAssistantId,
        stream: true,
      }

      const eventSource = aiService.createChatStream(request)
      eventSourceRef.current = eventSource
      let fullContent = ''

      eventSource.onmessage = (event) => {
        if (event.data === '[DONE]') {
          eventSource.close()
          const assistantMessage: ChatMessage = {
            role: 'assistant',
            content: fullContent,
            timestamp: new Date().toISOString(),
          }
          setMessages((prev) => [...prev, assistantMessage])
          setStreamingContent('')
          setIsStreaming(false)
          return
        }

        try {
          const data = JSON.parse(event.data)
          if (data.content) {
            fullContent += data.content
            setStreamingContent(fullContent)
          }
        } catch {
          // 忽略解析错误
        }
      }

      eventSource.onerror = () => {
        eventSource.close()
        if (!fullContent) {
          setError('连接失败，请检查 AI 服务是否运行')
        }
        setIsStreaming(false)
      }
    } catch (err: any) {
      setError(err.message || '发送失败')
      setIsStreaming(false)
    }
  }

  // 键盘快捷键
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
  }

  // 创建助手
  const handleCreateAssistant = async (data: any) => {
    try {
      const newAssistant = await aiService.createAssistant(data)
      setAssistants((prev) => [...prev, newAssistant])
      setShowDialog(false)
    } catch (err: any) {
      setError(err.message || '创建失败')
    }
  }

  // 更新助手
  const handleUpdateAssistant = async (data: any) => {
    if (!editingAssistant) return
    try {
      const updated = await aiService.updateAssistant(editingAssistant.id, data)
      setAssistants((prev) => prev.map((a) => (a.id === updated.id ? updated : a)))
      setEditingAssistant(undefined)
      setShowDialog(false)
    } catch (err: any) {
      setError(err.message || '更新失败')
    }
  }

  // 删除助手
  const handleDeleteAssistant = async (id: string) => {
    if (!confirm('确定删除这个助手吗？')) return
    try {
      await aiService.deleteAssistant(id)
      setAssistants((prev) => prev.filter((a) => a.id !== id))
      if (selectedAssistantId === id) {
        setSelectedAssistantId(undefined)
      }
    } catch (err: any) {
      setError(err.message || '删除失败')
    }
  }

  // 清空对话
  const handleClearChat = () => {
    setMessages([])
    setStreamingContent('')
  }

  return (
    <div className="ai-assistant-page">
      {/* 左侧面板 */}
      <div className="ai-sidebar">
        <AssistantSelector
          assistants={assistants}
          selectedId={selectedAssistantId}
          onSelect={setSelectedAssistantId}
          onCreate={() => {
            setEditingAssistant(undefined)
            setShowDialog(true)
          }}
        />

        {/* 助手操作 */}
        {selectedAssistantId && (
          <div className="ai-sidebar__actions">
            <button
              className="btn btn--small btn--secondary"
              onClick={() => {
                const a = assistants.find((a) => a.id === selectedAssistantId)
                if (a) {
                  setEditingAssistant(a)
                  setShowDialog(true)
                }
              }}
            >
              ✏️ 编辑
            </button>
            <button
              className="btn btn--small btn--danger"
              onClick={() => handleDeleteAssistant(selectedAssistantId)}
            >
              🗑️ 删除
            </button>
          </div>
        )}

        {/* 模型列表 */}
        {models.length > 0 && (
          <div className="ai-models">
            <h4>可用模型</h4>
            {models.map((m) => (
              <div key={m.id} className="ai-model-item">
                <span className="ai-model-item__name">{m.name}</span>
                <span className="ai-model-item__provider">{m.provider}</span>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* 聊天区域 */}
      <div className="ai-chat">
        {/* 消息列表 */}
        <div className="ai-chat__messages">
          {messages.length === 0 && !isStreaming && (
            <div className="ai-chat__welcome">
              <h2>🤖 AI 助手</h2>
              <p>选择一个助手或直接开始对话</p>
              {selectedAssistantId && (
                <p className="ai-chat__assistant-info">
                  当前助手: {assistants.find((a) => a.id === selectedAssistantId)?.name}
                </p>
              )}
            </div>
          )}

          {messages.map((msg, i) => (
            <ChatMessageBubble key={i} message={msg} />
          ))}

          {/* 流式消息 */}
          {isStreaming && streamingContent && (
            <div className="chat-message chat-message--assistant streaming">
              <div className="chat-message__avatar">🤖</div>
              <div className="chat-message__content">
                <div className="chat-message__text">{streamingContent}</div>
                <div className="chat-message__typing">正在输入...</div>
              </div>
            </div>
          )}

          <div ref={messagesEndRef} />
        </div>

        {/* 错误提示 */}
        {error && (
          <div className="ai-chat__error">
            <span>{error}</span>
            <button onClick={() => setError(null)}>✕</button>
          </div>
        )}

        {/* 输入区域 */}
        <div className="ai-chat__input">
          <button
            className="ai-chat__clear"
            onClick={handleClearChat}
            title="清空对话"
            disabled={messages.length === 0}
          >
            🗑️
          </button>
          <textarea
            ref={inputRef}
            value={inputText}
            onChange={(e) => setInputText(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="输入消息... (Enter 发送，Shift+Enter 换行)"
            disabled={isStreaming}
            rows={1}
          />
          <button
            className="ai-chat__send"
            onClick={handleSend}
            disabled={!inputText.trim() || isStreaming}
          >
            {isStreaming ? '⏳' : '📤'}
          </button>
        </div>
      </div>

      {/* 对话框 */}
      {showDialog && (
        <AssistantDialog
          assistant={editingAssistant}
          onSave={editingAssistant ? handleUpdateAssistant : handleCreateAssistant}
          onClose={() => {
            setShowDialog(false)
            setEditingAssistant(undefined)
          }}
        />
      )}
    </div>
  )
}
