import React, { useState, useEffect } from 'react'
import { useMessages } from '../hooks/useMessages'
import './MessageSearch.css'

interface MessageSearchProps {
  conversationId: string
  onMessageSelect: (messageId: string) => void
}

export function MessageSearch({ conversationId, onMessageSelect }: MessageSearchProps) {
  const [query, setQuery] = useState('')
  const [results, setResults] = useState<any[]>([])
  const [searching, setSearching] = useState(false)
  const [selectedIndex, setSelectedIndex] = useState(-1)

  // 模拟搜索功能
  const handleSearch = async (searchQuery: string) => {
    if (!searchQuery.trim()) {
      setResults([])
      return
    }

    setSearching(true)

    // 模拟搜索延迟
    await new Promise((resolve) => setTimeout(resolve, 300))

    // 这里应该调用API进行搜索
    // 暂时使用模拟数据
    const mockResults = [
      {
        id: '1',
        content: `${searchQuery} 相关的消息内容示例...`,
        createdAt: new Date().toISOString(),
        senderId: 'user',
      },
      {
        id: '2',
        content: `这是另一条包含 ${searchQuery} 的消息`,
        createdAt: new Date(Date.now() - 3600000).toISOString(),
        senderId: 'ai',
      },
    ]

    setResults(mockResults)
    setSearching(false)
  }

  useEffect(() => {
    const timeoutId = setTimeout(() => {
      handleSearch(query)
    }, 300)

    return () => clearTimeout(timeoutId)
  }, [query])

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (results.length === 0) return

    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setSelectedIndex((prev) => (prev < results.length - 1 ? prev + 1 : prev))
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setSelectedIndex((prev) => (prev > 0 ? prev - 1 : -1))
    } else if (e.key === 'Enter' && selectedIndex >= 0) {
      e.preventDefault()
      handleSelectMessage(results[selectedIndex])
    } else if (e.key === 'Escape') {
      setQuery('')
      setResults([])
      setSelectedIndex(-1)
    }
  }

  const handleSelectMessage = (message: any) => {
    onMessageSelect(message.id)
    setQuery('')
    setResults([])
    setSelectedIndex(-1)
  }

  const highlightText = (text: string, query: string) => {
    if (!query) return text

    const regex = new RegExp(`(${query})`, 'gi')
    const parts = text.split(regex)

    return parts.map((part, index) =>
      regex.test(part) ? (
        <mark key={index} className="search-highlight">
          {part}
        </mark>
      ) : (
        part
      )
    )
  }

  const formatTime = (dateString: string) => {
    const date = new Date(dateString)
    const now = new Date()
    const diffMs = now.getTime() - date.getTime()
    const diffMins = Math.floor(diffMs / 60000)

    if (diffMins < 1) return '刚刚'
    if (diffMins < 60) return `${diffMins}分钟前`
    if (diffMins < 1440) return `${Math.floor(diffMins / 60)}小时前`
    return date.toLocaleDateString()
  }

  return (
    <div className="message-search">
      <div className="search-container">
        <div className="search-input-wrapper">
          <span className="search-icon">🔍</span>
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="搜索消息..."
            className="search-input"
            autoComplete="off"
          />
          {query && (
            <button
              className="search-clear"
              onClick={() => {
                setQuery('')
                setResults([])
                setSelectedIndex(-1)
              }}
            >
              ✕
            </button>
          )}
        </div>

        {searching && (
          <div className="search-loading">
            <div className="loading-spinner"></div>
            <span>搜索中...</span>
          </div>
        )}
      </div>

      {results.length > 0 && !searching && (
        <div className="search-results">
          <div className="search-results-header">
            <span>找到 {results.length} 条结果</span>
          </div>

          <div className="search-results-list">
            {results.map((message, index) => (
              <div
                key={message.id}
                className={`search-result-item ${index === selectedIndex ? 'selected' : ''}`}
                onClick={() => handleSelectMessage(message)}
                onMouseEnter={() => setSelectedIndex(index)}
              >
                <div className="result-content">
                  <div className="result-text">
                    {highlightText(message.content, query)}
                  </div>
                  <div className="result-meta">
                    <span className="result-sender">
                      {message.senderId === 'user' ? '👤 你' : '🤖 AI助手'}
                    </span>
                    <span className="result-time">
                      {formatTime(message.createdAt)}
                    </span>
                  </div>
                </div>
              </div>
            ))}
          </div>

          <div className="search-results-footer">
            <span className="keyboard-hint">
              <kbd>↑</kbd> <kbd>↓</kbd> 导航
              <kbd>Enter</kbd> 选择
              <kbd>Esc</kbd> 关闭
            </span>
          </div>
        </div>
      )}

      {query && !searching && results.length === 0 && (
        <div className="search-no-results">
          <div className="no-results-icon">🔎</div>
          <p>未找到包含 "{query}" 的消息</p>
          <span className="no-results-hint">尝试使用其他关键词</span>
        </div>
      )}
    </div>
  )
}

interface SearchHistoryProps {
  history: string[]
  onSelect: (query: string) => void
  onClear: () => void
}

export function SearchHistory({ history, onSelect, onClear }: SearchHistoryProps) {
  if (history.length === 0) return null

  return (
    <div className="search-history">
      <div className="history-header">
        <span>搜索历史</span>
        <button className="history-clear" onClick={onClear}>
          清空
        </button>
      </div>

      <div className="history-list">
        {history.map((item, index) => (
          <div
            key={index}
            className="history-item"
            onClick={() => onSelect(item)}
          >
            <span className="history-icon">🕐</span>
            <span className="history-text">{item}</span>
          </div>
        ))}
      </div>
    </div>
  )
}
