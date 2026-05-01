import React, { useRef, useState } from 'react'
import { useToast } from './Toast'
import './FileUploader.css'

interface FileUploaderProps {
  onUpload: (files: File[]) => Promise<void>
  maxSize?: number // MB
  accept?: string
  multiple?: boolean
  disabled?: boolean
}

export function FileUploader({
  onUpload,
  maxSize = 10,
  accept = '*/*',
  multiple = false,
  disabled = false,
}: FileUploaderProps) {
  const { showSuccess, showError } = useToast()
  const fileInputRef = useRef<HTMLInputElement>(null)
  const [uploading, setUploading] = useState(false)
  const [dragActive, setDragActive] = useState(false)

  const handleFileSelect = async (files: FileList | null) => {
    if (!files || files.length === 0) return

    const fileList = Array.from(files)
    const validFiles: File[] = []

    for (const file of fileList) {
      // 检查文件大小
      if (file.size > maxSize * 1024 * 1024) {
        showError(`${file.name} 超过 ${maxSize}MB 限制`)
        continue
      }

      validFiles.push(file)
    }

    if (validFiles.length === 0) return

    setUploading(true)
    try {
      await onUpload(validFiles)
      showSuccess(`成功上传 ${validFiles.length} 个文件`)
    } catch (error) {
      console.error('文件上传失败:', error)
      showError('文件上传失败，请稍后重试')
    } finally {
      setUploading(false)
      // 清空文件输入
      if (fileInputRef.current) {
        fileInputRef.current.value = ''
      }
    }
  }

  const handleDrag = (e: React.DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
    if (e.type === 'dragenter' || e.type === 'dragover') {
      setDragActive(true)
    } else if (e.type === 'dragleave') {
      setDragActive(false)
    }
  }

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
    setDragActive(false)
    handleFileSelect(e.dataTransfer.files)
  }

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    handleFileSelect(e.target.files)
  }

  const handleClick = () => {
    if (!disabled && !uploading) {
      fileInputRef.current?.click()
    }
  }

  const formatFileSize = (bytes: number) => {
    if (bytes === 0) return '0 Bytes'
    const k = 1024
    const sizes = ['Bytes', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i]
  }

  return (
    <div className="file-uploader">
      <input
        ref={fileInputRef}
        type="file"
        accept={accept}
        multiple={multiple}
        onChange={handleInputChange}
        disabled={disabled || uploading}
        className="file-input"
      />

      <div
        className={`file-dropzone ${dragActive ? 'active' : ''} ${disabled || uploading ? 'disabled' : ''}`}
        onClick={handleClick}
        onDragEnter={handleDrag}
        onDragLeave={handleDrag}
        onDragOver={handleDrag}
        onDrop={handleDrop}
      >
        <div className="dropzone-content">
          <div className="dropzone-icon">
            {uploading ? '⏳' : dragActive ? '📥' : '📁'}
          </div>
          <div className="dropzone-text">
            {uploading ? '上传中...' : '点击或拖拽文件到这里'}
          </div>
          <div className="dropzone-hint">
            最大 {maxSize}MB，支持 {multiple ? '多文件' : '单文件'}上传
          </div>
        </div>
      </div>
    </div>
  )
}

interface FilePreviewProps {
  file: File
  onRemove: () => void
  progress?: number
}

export function FilePreview({ file, onRemove, progress }: FilePreviewProps) {
  const [preview, setPreview] = useState<string | null>(null)

  React.useEffect(() => {
    // 为图片文件生成预览
    if (file.type.startsWith('image/')) {
      const reader = new FileReader()
      reader.onloadend = () => {
        setPreview(reader.result as string)
      }
      reader.readAsDataURL(file)
    }
  }, [file])

  const formatFileSize = (bytes: number) => {
    if (bytes === 0) return '0 Bytes'
    const k = 1024
    const sizes = ['Bytes', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i]
  }

  const getFileIcon = (type: string) => {
    if (type.startsWith('image/')) return '🖼️'
    if (type.startsWith('video/')) return '🎬'
    if (type.startsWith('audio/')) return '🎵'
    if (type.includes('pdf')) return '📄'
    if (type.includes('word') || type.includes('document')) return '📝'
    if (type.includes('excel') || type.includes('sheet')) return '📊'
    if (type.includes('zip') || type.includes('rar')) return '📦'
    if (type.includes('code') || type.includes('json') || type.includes('javascript')) return '💻'
    return '📎'
  }

  return (
    <div className="file-preview">
      {preview ? (
        <img src={preview} alt={file.name} className="file-preview-image" />
      ) : (
        <div className="file-preview-icon">{getFileIcon(file.type)}</div>
      )}

      <div className="file-preview-info">
        <div className="file-name" title={file.name}>
          {file.name}
        </div>
        <div className="file-size">{formatFileSize(file.size)}</div>

        {progress !== undefined && progress < 100 && (
          <div className="file-progress">
            <div
              className="file-progress-bar"
              style={{ width: `${progress}%` }}
            />
          </div>
        )}
      </div>

      <button className="file-remove" onClick={onRemove} title="移除">
        ✕
      </button>
    </div>
  )
}

interface FileListProps {
  files: Array<{ file: File; progress?: number }>
  onRemove: (index: number) => void
}

export function FileList({ files, onRemove }: FileListProps) {
  if (files.length === 0) return null

  return (
    <div className="file-list">
      {files.map((item, index) => (
        <FilePreview
          key={index}
          file={item.file}
          progress={item.progress}
          onRemove={() => onRemove(index)}
        />
      ))}
    </div>
  )
}
