import React from 'react'
import './Input.css'

interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  label?: string
  error?: string
  hint?: string
  icon?: React.ReactNode
}

export function Input({
  label,
  error,
  hint,
  icon,
  className = '',
  id,
  ...props
}: InputProps) {
  const inputId = id || `input-${Math.random().toString(36).substr(2, 9)}`
  const hasError = !!error

  return (
    <div className={`input-group ${hasError ? 'input-group-error' : ''} ${className}`}>
      {label && (
        <label htmlFor={inputId} className="input-label">
          {icon && <span className="input-icon">{icon}</span>}
          {label}
        </label>
      )}
      <div className="input-wrapper">
        {icon && !label && <span className="input-icon-prefix">{icon}</span>}
        <input id={inputId} className={`input ${hasError ? 'input-error' : ''}`} {...props} />
      </div>
      {error && <div className="input-error">⚠️ {error}</div>}
      {hint && !error && <div className="input-hint">{hint}</div>}
    </div>
  )
}

interface TextareaProps extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {
  label?: string
  error?: string
  hint?: string
}

export function Textarea({
  label,
  error,
  hint,
  className = '',
  id,
  ...props
}: TextareaProps) {
  const textareaId = id || `textarea-${Math.random().toString(36).substr(2, 9)}`
  const hasError = !!error

  return (
    <div className={`input-group ${hasError ? 'input-group-error' : ''} ${className}`}>
      {label && <label htmlFor={textareaId} className="input-label">{label}</label>}
      <textarea
        id={textareaId}
        className={`textarea ${hasError ? 'input-error' : ''}`}
        {...props}
      />
      {error && <div className="input-error">⚠️ {error}</div>}
      {hint && !error && <div className="input-hint">{hint}</div>}
    </div>
  )
}
