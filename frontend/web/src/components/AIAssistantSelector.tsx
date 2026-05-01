import React, { useState } from 'react'
import type { AIAssistant } from '../types/ai'
import { aiService } from '../services/aiService'
import { mockApi } from '../services/mockApi'
import './AIAssistantSelector.css'

const USE_MOCK_DATA = import.meta.env.VITE_USE_MOCK_DATA === 'true'

interface AIAssistantSelectorProps {
  onSelect: (assistant: AIAssistant) => void
  currentAssistant?: AIAssistant
}

export function AIAssistantSelector({ onSelect, currentAssistant }: AIAssistantSelectorProps) {
  const [assistants, setAssistants] = useState<AIAssistant[]>([])
  const [loading, setLoading] = useState(true)
  const [showCreate, setShowCreate] = useState(false)
  const [formData, setFormData] = useState({
    name: '',
    modelProvider: 'openai',
    modelName: 'gpt-4',
    systemPrompt: '',
    temperature: 0.7,
    maxTokens: 2048,
  })

  React.useEffect(() => {
    loadAssistants()
  }, [])

  const loadAssistants = async () => {
    try {
      if (USE_MOCK_DATA) {
        setAssistants(mockAssistants)
      } else {
        const response = await aiService.getAssistants()
        if (response.success && response.data) {
          setAssistants(response.data)
        }
      }
    } catch (error) {
      console.error('加载AI助手失败:', error)
    } finally {
      setLoading(false)
    }
  }

  const handleCreate = async () => {
    if (!formData.name.trim()) return

    try {
      if (USE_MOCK_DATA) {
        const newAssistant: AIAssistant = {
          id: 'ai-' + Date.now(),
          userId: '1',
          name: formData.name,
          modelProvider: formData.modelProvider as any,
          modelName: formData.modelName,
          systemPrompt: formData.systemPrompt,
          temperature: formData.temperature,
          maxTokens: formData.maxTokens,
          isPublic: false,
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        }
        setAssistants([...assistants, newAssistant])
        setShowCreate(false)
      } else {
        const response = await aiService.createAssistant(formData)
        if (response.success && response.data) {
          setAssistants([...assistants, response.data])
          setShowCreate(false)
        }
      }
    } catch (error) {
      console.error('创建AI助手失败:', error)
    }
  }

  return (
    <div className="ai-assistant-selector">
      <div className="ai-header">
        <h3 className="ai-title">🤖 AI助手</h3>
        <button
          className="btn btn-primary btn-sm"
          onClick={() => setShowCreate(true)}
        >
          + 新建
        </button>
      </div>

      {loading ? (
        <div className="ai-loading">加载中...</div>
      ) : (
        <div className="ai-list">
          {assistants.map((assistant) => (
            <div
              key={assistant.id}
              className={`ai-item ${currentAssistant?.id === assistant.id ? 'active' : ''}`}
              onClick={() => onSelect(assistant)}
            >
              <div className="ai-avatar">
                {assistant.avatar ? (
                  <img src={assistant.avatar} alt={assistant.name} />
                ) : (
                  <div className="ai-avatar-placeholder">
                    {assistant.name?.charAt(0).toUpperCase() || 'AI'}
                  </div>
                )}
              </div>
              <div className="ai-info">
                <div className="ai-name">{assistant.name}</div>
                <div className="ai-model">{assistant.modelProvider} / {assistant.modelName}</div>
              </div>
              {assistant.isPublic && (
                <div className="ai-badge">公开</div>
              )}
            </div>
          ))}
        </div>
      )}

      {showCreate && (
        <div className="ai-create-form">
          <h4>创建新AI助手</h4>
          <div className="form-group">
            <label>助手名称</label>
            <input
              type="text"
              value={formData.name}
              onChange={(e) => setFormData({ ...formData, name: e.target.value })}
              placeholder="例如：代码助手、写作助手..."
            />
          </div>
          <div className="form-group">
            <label>系统提示词</label>
            <textarea
              value={formData.systemPrompt}
              onChange={(e) => setFormData({ ...formData, systemPrompt: e.target.value })}
              placeholder="定义AI助手的角色和行为..."
              rows={3}
            />
          </div>
          <div className="form-row">
            <div className="form-group">
              <label>模型提供商</label>
              <select
                value={formData.modelProvider}
                onChange={(e) => setFormData({ ...formData, modelProvider: e.target.value })}
              >
                <option value="openai">OpenAI</option>
                <option value="anthropic">Anthropic</option>
                <option value="google">Google</option>
                <option value="local">本地模型</option>
              </select>
            </div>
            <div className="form-group">
              <label>模型名称</label>
              <input
                type="text"
                value={formData.modelName}
                onChange={(e) => setFormData({ ...formData, modelName: e.target.value })}
                placeholder="gpt-4, claude-3..."
              />
            </div>
          </div>
          <div className="form-row">
            <div className="form-group">
              <label>温度: {formData.temperature}</label>
              <input
                type="range"
                min="0"
                max="2"
                step="0.1"
                value={formData.temperature}
                onChange={(e) => setFormData({ ...formData, temperature: parseFloat(e.target.value) })}
              />
            </div>
            <div className="form-group">
              <label>最大令牌</label>
              <input
                type="number"
                value={formData.maxTokens}
                onChange={(e) => setFormData({ ...formData, maxTokens: parseInt(e.target.value) })}
                min="1"
                max="32000"
              />
            </div>
          </div>
          <div className="form-actions">
            <button
              className="btn btn-secondary"
              onClick={() => setShowCreate(false)}
            >
              取消
            </button>
            <button className="btn btn-primary" onClick={handleCreate}>
              创建
            </button>
          </div>
        </div>
      )}
    </div>
  )
}

const mockAssistants: AIAssistant[] = [
  {
    id: 'ai-1',
    userId: '1',
    name: '通用助手',
    modelProvider: 'openai',
    modelName: 'gpt-4',
    systemPrompt: '你是一个友好的AI助手，帮助用户解决问题。',
    temperature: 0.7,
    maxTokens: 2048,
    isPublic: true,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  },
  {
    id: 'ai-2',
    userId: '1',
    name: '代码助手',
    modelProvider: 'openai',
    modelName: 'gpt-4',
    systemPrompt: '你是一个专业的编程助手，帮助用户解决编程问题。',
    temperature: 0.3,
    maxTokens: 4096,
    isPublic: true,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  },
]
