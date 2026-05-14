import { apiRequest } from './api'

// ============================================================
// 类型定义
// ============================================================

export interface FileItem {
  id: string
  filename: string
  original_filename: string
  mime_type: string
  size: number
  storage_path: string
  thumbnail_path?: string
  owner_id: string
  is_public: boolean
  metadata?: Record<string, any>
  created_at: string
  updated_at: string
}

export interface FileShare {
  id: string
  file_id: string
  share_token: string
  expires_at?: string
  max_downloads?: number
  download_count: number
  created_at: string
}

export interface StorageStats {
  total_files: number
  total_size: number
  by_type: Record<string, { count: number; size: number }>
}

export interface FileListParams {
  page?: number
  page_size?: number
  mime_type?: string
  sort_by?: 'created_at' | 'size' | 'filename'
  sort_order?: 'asc' | 'desc'
}

export interface PaginatedResponse<T> {
  items: T[]
  total: number
  page: number
  page_size: number
  total_pages: number
}

// ============================================================
// 文件服务
// ============================================================

export const fileService = {
  /** 上传文件 */
  async uploadFile(file: File, isPublic: boolean = false): Promise<FileItem> {
    const formData = new FormData()
    formData.append('file', file)
    formData.append('is_public', String(isPublic))

    const token = localStorage.getItem('token')
    const baseUrl = import.meta.env.VITE_API_BASE_URL || 'http://localhost:8002'

    const response = await fetch(`${baseUrl}/api/files/upload`, {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${token}`,
      },
      body: formData,
    })

    if (!response.ok) {
      const error = await response.json().catch(() => ({ message: 'Upload failed' }))
      throw new Error(error.message || 'Upload failed')
    }

    return response.json()
  },

  /** 批量上传文件 */
  async batchUpload(files: File[]): Promise<FileItem[]> {
    const formData = new FormData()
    files.forEach((file) => formData.append('files', file))

    const token = localStorage.getItem('token')
    const baseUrl = import.meta.env.VITE_API_BASE_URL || 'http://localhost:8002'

    const response = await fetch(`${baseUrl}/api/files/batch-upload`, {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${token}`,
      },
      body: formData,
    })

    if (!response.ok) {
      const error = await response.json().catch(() => ({ message: 'Upload failed' }))
      throw new Error(error.message || 'Upload failed')
    }

    return response.json()
  },

  /** 获取文件列表 */
  async listFiles(params: FileListParams = {}): Promise<PaginatedResponse<FileItem>> {
    const searchParams = new URLSearchParams()
    if (params.page) searchParams.set('page', String(params.page))
    if (params.page_size) searchParams.set('page_size', String(params.page_size))
    if (params.mime_type) searchParams.set('mime_type', params.mime_type)
    if (params.sort_by) searchParams.set('sort_by', params.sort_by)
    if (params.sort_order) searchParams.set('sort_order', params.sort_order)

    const query = searchParams.toString()
    return apiRequest<PaginatedResponse<FileItem>>(`/api/files${query ? `?${query}` : ''}`)
  },

  /** 获取单个文件信息 */
  async getFile(fileId: string): Promise<FileItem> {
    return apiRequest<FileItem>(`/api/files/${fileId}`)
  },

  /** 删除文件 */
  async deleteFile(fileId: string): Promise<void> {
    return apiRequest<void>(`/api/files/${fileId}`, {
      method: 'DELETE',
    })
  },

  /** 更新文件信息 */
  async updateFile(fileId: string, data: { filename?: string; is_public?: boolean }): Promise<FileItem> {
    return apiRequest<FileItem>(`/api/files/${fileId}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    })
  },

  /** 获取文件下载 URL */
  getDownloadUrl(fileId: string): string {
    const token = localStorage.getItem('token')
    const baseUrl = import.meta.env.VITE_API_BASE_URL || 'http://localhost:8002'
    return `${baseUrl}/api/files/${fileId}?token=${token}`
  },

  /** 获取文件预览 URL */
  getPreviewUrl(fileId: string): string {
    const token = localStorage.getItem('token')
    const baseUrl = import.meta.env.VITE_API_BASE_URL || 'http://localhost:8002'
    return `${baseUrl}/api/files/${fileId}/preview?token=${token}`
  },

  /** 获取缩略图 URL */
  getThumbnailUrl(fileId: string): string {
    const token = localStorage.getItem('token')
    const baseUrl = import.meta.env.VITE_API_BASE_URL || 'http://localhost:8002'
    return `${baseUrl}/api/files/${fileId}/thumbnail?token=${token}`
  },

  /** 创建文件分享 */
  async createShare(fileId: string, options?: { expires_at?: string; max_downloads?: number }): Promise<FileShare> {
    return apiRequest<FileShare>(`/api/files/${fileId}/shares`, {
      method: 'POST',
      body: JSON.stringify(options || {}),
    })
  },

  /** 获取文件的分享列表 */
  async getFileShares(fileId: string): Promise<FileShare[]> {
    return apiRequest<FileShare[]>(`/api/files/${fileId}/shares`)
  },

  /** 删除分享 */
  async deleteShare(shareId: string): Promise<void> {
    return apiRequest<void>(`/api/files/shares/${shareId}`, {
      method: 'DELETE',
    })
  },

  /** 获取存储统计 */
  async getStorageStats(): Promise<StorageStats> {
    return apiRequest<StorageStats>('/api/files/stats/storage')
  },

  /** 格式化文件大小 */
  formatFileSize(bytes: number): string {
    if (bytes === 0) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
  },

  /** 获取文件类型图标 */
  getFileIcon(mimeType: string): string {
    if (mimeType.startsWith('image/')) return '🖼️'
    if (mimeType.startsWith('video/')) return '🎬'
    if (mimeType.startsWith('audio/')) return '🎵'
    if (mimeType.includes('pdf')) return '📄'
    if (mimeType.includes('word') || mimeType.includes('document')) return '📝'
    if (mimeType.includes('sheet') || mimeType.includes('excel')) return '📊'
    if (mimeType.includes('presentation') || mimeType.includes('powerpoint')) return '📑'
    if (mimeType.includes('zip') || mimeType.includes('rar') || mimeType.includes('tar')) return '📦'
    if (mimeType.startsWith('text/')) return '📃'
    return '📁'
  },

  /** 判断是否为图片文件 */
  isImage(mimeType: string): boolean {
    return mimeType.startsWith('image/')
  },

  /** 判断是否为视频文件 */
  isVideo(mimeType: string): boolean {
    return mimeType.startsWith('video/')
  },

  /** 判断是否为音频文件 */
  isAudio(mimeType: string): boolean {
    return mimeType.startsWith('audio/')
  },

  /** 判断是否可预览 */
  isPreviewable(mimeType: string): boolean {
    return (
      mimeType.startsWith('image/') ||
      mimeType.startsWith('video/') ||
      mimeType.startsWith('audio/') ||
      mimeType === 'application/pdf' ||
      mimeType.startsWith('text/')
    )
  },
}
