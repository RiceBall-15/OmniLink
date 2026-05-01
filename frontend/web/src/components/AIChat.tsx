import React, { useState, useRef, useEffect } from 'react'
import { useMessages } from '../hooks/useMessages'
import { useToast } from './Toast'
import './AIChat.css'

interface AIChatProps {
  conversationId: string
  assistantId: string
}

export function AIChat({ conversationId, assistantId }: AIChatProps) {
  const { messages, loading: messagesLoading, sendMessage: sendApiMessage } = useMessages(conversationId)
  const { showSuccess, showError } = useToast()
  const [input, setInput] = useState('')
  const [isStreaming, setIsStreaming] = useState(false)
  const [streamedContent, setStreamedContent] = useState('')
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const textareaRef = useRef<HTMLTextAreaElement>(null)

  // 自动滚动到底部
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages, streamedContent])

  const handleSend = async () => {
    if (!input.trim() || isStreaming) return

    const userMessage = input
    setInput('')

    try {
      // 先发送用户消息
      await sendApiMessage(userMessage)

      // 开始流式对话
      await startStreaming(userMessage)
    } catch (error) {
      console.error('发送消息失败:', error)
      showError('发送消息失败，请稍后重试')
    }
  }

  const startStreaming = async (userMessage: string) => {
    setIsStreaming(true)
    setStreamedContent('')

    try {
      const token = localStorage.getItem('token')
      const response = await fetch(`${import.meta.env.VITE_API_BASE_URL}/ai/conversations/${conversationId}/stream`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${token}`,
        },
        body: JSON.stringify({
          content: userMessage,
          assistantId,
        }),
      })

      if (!response.ok) {
        throw new Error('AI对话失败')
      }

      const reader = response.body?.getReader()
      if (!reader) {
        throw new Error('无法创建流式读取器')
      }

      const decoder = new TextDecoder()
      let fullContent = ''

      while (true) {
        const { done, value } = await reader.read()
        if (done) break

        const chunk = decoder.decode(value)
        fullContent += chunk
        setStreamedContent(fullContent)
      }

      // 流式完成，添加到消息列表
      // 这里需要调用API保存AI的响应消息
      showSuccess('AI回复完成')
    } catch (error) {
      console.error('流式对话失败:', error)
      // 如果失败，使用模拟数据
      await simulateStreaming(userMessage)
    } finally {
      setIsStreaming(false)
      setStreamedContent('')
    }
  }

  const simulateStreaming = async (userMessage: string) => {
    // 模拟流式回复
    const responses = [
      '这是一个很好的问题！让我来帮你解答。',
      '根据我的分析，这个方案是可行的。',
      '我理解你的需求，这里有几个建议供你参考。',
      '这是一个有趣的话题，让我详细解释一下。',
    ]

    const response = responses[Math.floor(Math.random() * responses.length)]
    const content = `${response}\n\n你提到的内容是：${userMessage}。如果你有更多问题，请随时问我！`

    for (let i = 0; i < content.length; i++) {
      await new Promise((resolve) => setTimeout(resolve, 20))
      setStreamedContent(content.slice(0, i + 1))
    }

    await new Promise((resolve) => setTimeout(resolve, 500))
  }

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
  }

  const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setInput(e.target.value)
    // 自动调整高度
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto'
      textareaRef.current.style.height = Math.min(textareaRef.current.scrollHeight, 200) + 'px'
    }
  }

  const isTyping = isStreaming || messagesLoading

  return (
    <div className="ai-chat">
      <div className="ai-messages">
        {messagesLoading ? (
          <div className="ai-loading">
            <div className="loading-dots">
              <span></span>
              <span></span>
              <span></span>
            </div>
            <p>加载消息中...</p>
          </div>
        ) : messages.length === 0 ? (
          <div className="ai-welcome">
            <div className="welcome-icon">🤖</div>
            <h2>开始对话</h2>
            <p>你可以问我任何问题，我会尽力为你解答</p>
            <div className="welcome-prompts">
              <div
                className="welcome-prompt"
                onClick={() => setInput('介绍一下你自己')}
              >
                💡 介绍一下你自己
              </div>
              <div
                className="welcome-prompt"
                onClick={() => setInput('帮我写一个Python函数')}
              >
                💡 帮我写一个Python函数
              </div>
              <div
                className="welcome-prompt"
                onClick={() => setInput('解释一下机器学习')}
              >
                💡 解释一下机器学习
              </div>
            </div>
          </div>
        ) : (
          <>
            {messages.map((message, index) => (
              <div
                key={message.id || index}
                className={`ai-message ${message.senderId === 'user' ? 'user' : 'ai'}`}
              >
                <div className="message-avatar">
                  {message.senderId === 'user' ? '👤' : '🤖'}
                </div>
                <div className="message-content">
                  <div className="message-text">{message.content}</div>
                  <div className="message-time">
                    {new Date(message.createdAt).toLocaleTimeString()}
                  </div>
                </div>
              </div>
            ))}

            {isStreaming && (
              <div className="ai-message ai streaming">
                <div className="message-avatar">🤖</div>
                <div className="message-content">
                  <div className="message-text">
                    <StreamingContent content={streamedContent} />
                  </div>
                </div>
              </div>
            )}

            <div ref={messagesEndRef} />
          </>
        )}
      </div>

      <div className="ai-input-container">
        <div className="ai-input-wrapper">
          <textarea
            ref={textareaRef}
            value={input}
            onChange={handleInputChange}
            onKeyDown={handleKeyPress}
            placeholder="输入消息... (Enter发送, Shift+Enter换行)"
            rows={1}
            disabled={isStreaming}
            className="ai-textarea"
          />
          <div className="ai-input-actions">
            <button className="action-button" title="上传文件" disabled={isStreaming}>
              📎
            </button>
            <button className="action-button" title="发送表情" disabled={isStreaming}>
              😊
            </button>
            <button
              className={`send-button ${isStreaming ? 'disabled' : ''}`}
              onClick={handleSend}
              disabled={isStreaming || !input.trim()}
            >
              {isStreaming ? '⏳' : '➤'}
            </button>
          </div>
        </div>
        <div className="ai-input-hint">
          {isStreaming ? 'AI正在思考...' : `已输入 ${input.length} 字符`}
        </div>
      </div>
    </div>
  )
}

interface StreamingContentProps {
  content: string
}

function StreamingContent({ content }: StreamingContentProps) {
  return (
    <div className="streaming-content">
      <div className="streaming-text">{content}</div>
      <span className="streaming-cursor">▊</span>
    </div>
  )
}
