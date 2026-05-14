import React, { useState, useEffect, useRef, useCallback } from 'react'
import { fileService, FileItem, StorageStats, FileListParams } from '../services/fileService'
import './FileManagerPage.css'

// ============================================================
// 子组件：存储统计卡片
// ============================================================

function StorageStatsCard({ stats }: { stats: StorageStats | null }) {
  if (!stats) return null

  const totalSizeFormatted = fileService.formatFileSize(stats.total_size)
  const topTypes = Object.entries(stats.by_type)
    .sort(([, a], [, b]) => b.size - a.size)
    .slice(0, 5)

  return (
    <div className="storage-stats">
      <div className="storage-stats__main">
        <div className="storage-stats__icon">💾</div>
        <div className="storage-stats__info">
          <div className="storage-stats__size">{totalSizeFormatted}</div>
          <div className="storage-stats__count">{stats.total_files} 个文件</div>
        </div>
      </div>

      {topTypes.length > 0 && (
        <div className="storage-stats__breakdown">
          {topTypes.map(([type, data]) => (
            <div key={type} className="storage-stats__type">
              <span className="storage-stats__type-icon">
                {fileService.getFileIcon(type + '/generic')}
              </span>
              <span className="storage-stats__type-name">
                {type.split('/')[1] || type}
              </span>
              <span className="storage-stats__type-size">
                {fileService.formatFileSize(data.size)}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

// ============================================================
// 子组件：文件卡片
// ============================================================

function FileCard({
  file,
  onPreview,
  onShare,
  onDelete,
}: {
  file: FileItem
  onPreview: (file: FileItem) => void
  onShare: (file: FileItem) => void
  onDelete: (file: FileItem) => void
}) {
  const isImage = fileService.isImage(file.mime_type)
  const isPreviewable = fileService.isPreviewable(file.mime_type)

  return (
    <div className="file-card">
      {/* 缩略图/图标 */}
      <div className="file-card__preview" onClick={() => isPreviewable && onPreview(file)}>
        {isImage ? (
          <img
            src={fileService.getThumbnailUrl(file.id)}
            alt={file.original_filename}
            className="file-card__thumbnail"
            loading="lazy"
          />
        ) : (
          <div className="file-card__icon">
            {fileService.getFileIcon(file.mime_type)}
          </div>
        )}
        {isPreviewable && (
          <div className="file-card__overlay">
            <span>👁️ 预览</span>
          </div>
        )}
      </div>

      {/* 文件信息 */}
      <div className="file-card__info">
        <div className="file-card__name" title={file.original_filename}>
          {file.original_filename}
        </div>
        <div className="file-card__meta">
          <span>{fileService.formatFileSize(file.size)}</span>
          <span>•</span>
          <span>{new Date(file.created_at).toLocaleDateString()}</span>
        </div>
      </div>

      {/* 操作按钮 */}
      <div className="file-card__actions">
        <button
          className="file-card__action"
          onClick={() => {
            const url = fileService.getDownloadUrl(file.id)
            window.open(url, '_blank')
          }}
          title="下载"
        >
          ⬇️
        </button>
        <button
          className="file-card__action"
          onClick={() => onShare(file)}
          title="分享"
        >
          🔗
        </button>
        <button
          className="file-card__action file-card__action--danger"
          onClick={() => onDelete(file)}
          title="删除"
        >
          🗑️
        </button>
      </div>
    </div>
  )
}

// ============================================================
// 子组件：文件预览对话框
// ============================================================

function FilePreviewDialog({
  file,
  onClose,
}: {
  file: FileItem
  onClose: () => void
}) {
  const isImage = fileService.isImage(file.mime_type)
  const isVideo = fileService.isVideo(file.mime_type)
  const isAudio = fileService.isAudio(file.mime_type)

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="preview-dialog" onClick={(e) => e.stopPropagation()}>
        <div className="preview-dialog__header">
          <h3>{file.original_filename}</h3>
          <div className="preview-dialog__actions">
            <a
              href={fileService.getDownloadUrl(file.id)}
              target="_blank"
              rel="noopener noreferrer"
              className="btn btn--small btn--secondary"
            >
              ⬇️ 下载
            </a>
            <button className="dialog__close" onClick={onClose}>✕</button>
          </div>
        </div>

        <div className="preview-dialog__content">
          {isImage && (
            <img
              src={fileService.getPreviewUrl(file.id)}
              alt={file.original_filename}
              className="preview-image"
            />
          )}
          {isVideo && (
            <video
              src={fileService.getPreviewUrl(file.id)}
              controls
              className="preview-video"
            />
          )}
          {isAudio && (
            <audio
              src={fileService.getPreviewUrl(file.id)}
              controls
              className="preview-audio"
            />
          )}
          {!isImage && !isVideo && !isAudio && (
            <div className="preview-unsupported">
              <span className="preview-unsupported__icon">
                {fileService.getFileIcon(file.mime_type)}
              </span>
              <p>此文件类型不支持在线预览</p>
              <a
                href={fileService.getDownloadUrl(file.id)}
                target="_blank"
                rel="noopener noreferrer"
                className="btn btn--primary"
              >
                下载文件
              </a>
            </div>
          )}
        </div>

        <div className="preview-dialog__info">
          <div className="preview-info__item">
            <strong>大小:</strong> {fileService.formatFileSize(file.size)}
          </div>
          <div className="preview-info__item">
            <strong>类型:</strong> {file.mime_type}
          </div>
          <div className="preview-info__item">
            <strong>上传时间:</strong> {new Date(file.created_at).toLocaleString()}
          </div>
        </div>
      </div>
    </div>
  )
}

// ============================================================
// 子组件：分享对话框
// ============================================================

function ShareDialog({
  file,
  onClose,
}: {
  file: FileItem
  onClose: () => void
}) {
  const [shares, setShares] = useState<any[]>([])
  const [loading, setLoading] = useState(true)
  const [creating, setCreating] = useState(false)
  const [expiresIn, setExpiresIn] = useState<string>('never')
  const [maxDownloads, setMaxDownloads] = useState<string>('')
  const [copiedToken, setCopiedToken] = useState<string | null>(null)

  useEffect(() => {
    loadShares()
  }, [])

  const loadShares = async () => {
    try {
      const data = await fileService.getFileShares(file.id)
      setShares(data)
    } catch (err) {
      console.error('Failed to load shares:', err)
    } finally {
      setLoading(false)
    }
  }

  const handleCreateShare = async () => {
    setCreating(true)
    try {
      const options: any = {}
      if (expiresIn !== 'never') {
        const hours = parseInt(expiresIn)
        options.expires_at = new Date(Date.now() + hours * 3600000).toISOString()
      }
      if (maxDownloads) {
        options.max_downloads = parseInt(maxDownloads)
      }
      const newShare = await fileService.createShare(file.id, options)
      setShares((prev) => [newShare, ...prev])
    } catch (err: any) {
      alert(err.message || '创建分享失败')
    } finally {
      setCreating(false)
    }
  }

  const handleCopyLink = (shareToken: string) => {
    const baseUrl = window.location.origin
    const link = `${baseUrl}/shared/${shareToken}`
    navigator.clipboard.writeText(link)
    setCopiedToken(shareToken)
    setTimeout(() => setCopiedToken(null), 2000)
  }

  const handleDeleteShare = async (shareId: string) => {
    try {
      await fileService.deleteShare(shareId)
      setShares((prev) => prev.filter((s) => s.id !== shareId))
    } catch (err: any) {
      alert(err.message || '删除分享失败')
    }
  }

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog share-dialog" onClick={(e) => e.stopPropagation()}>
        <div className="dialog__header">
          <h3>分享文件: {file.original_filename}</h3>
          <button className="dialog__close" onClick={onClose}>✕</button>
        </div>

        <div className="dialog__body">
          {/* 创建新分享 */}
          <div className="share-create">
            <h4>创建新分享链接</h4>
            <div className="share-create__options">
              <div className="form-group">
                <label>有效期</label>
                <select value={expiresIn} onChange={(e) => setExpiresIn(e.target.value)}>
                  <option value="never">永久有效</option>
                  <option value="1">1 小时</option>
                  <option value="24">24 小时</option>
                  <option value="168">7 天</option>
                  <option value="720">30 天</option>
                </select>
              </div>
              <div className="form-group">
                <label>最大下载次数</label>
                <input
                  type="number"
                  value={maxDownloads}
                  onChange={(e) => setMaxDownloads(e.target.value)}
                  placeholder="不限制"
                  min="1"
                />
              </div>
              <button
                className="btn btn--primary"
                onClick={handleCreateShare}
                disabled={creating}
              >
                {creating ? '创建中...' : '🔗 创建分享'}
              </button>
            </div>
          </div>

          {/* 已有分享列表 */}
          <div className="share-list">
            <h4>已创建的分享</h4>
            {loading ? (
              <div className="share-loading">加载中...</div>
            ) : shares.length === 0 ? (
              <div className="share-empty">暂无分享链接</div>
            ) : (
              shares.map((share) => (
                <div key={share.id} className="share-item">
                  <div className="share-item__info">
                    <div className="share-item__token">{share.share_token}</div>
                    <div className="share-item__meta">
                      下载次数: {share.download_count}
                      {share.max_downloads && ` / ${share.max_downloads}`}
                      {share.expires_at && (
                        <> • 过期时间: {new Date(share.expires_at).toLocaleString()}</>
                      )}
                    </div>
                  </div>
                  <div className="share-item__actions">
                    <button
                      className="btn btn--small btn--secondary"
                      onClick={() => handleCopyLink(share.share_token)}
                    >
                      {copiedToken === share.share_token ? '✅ 已复制' : '📋 复制链接'}
                    </button>
                    <button
                      className="btn btn--small btn--danger"
                      onClick={() => handleDeleteShare(share.id)}
                    >
                      🗑️
                    </button>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  )
}

// ============================================================
// 主页面组件
// ============================================================

export default function FileManagerPage() {
  const [files, setFiles] = useState<FileItem[]>([])
  const [stats, setStats] = useState<StorageStats | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [page, setPage] = useState(1)
  const [totalPages, setTotalPages] = useState(1)
  const [filterType, setFilterType] = useState<string>('')
  const [sortBy, setSortBy] = useState<'created_at' | 'size' | 'filename'>('created_at')
  const [sortOrder, setSortOrder] = useState<'asc' | 'desc'>('desc')
  const [previewFile, setPreviewFile] = useState<FileItem | null>(null)
  const [shareFile, setShareFile] = useState<FileItem | null>(null)
  const [uploading, setUploading] = useState(false)
  const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid')

  const fileInputRef = useRef<HTMLInputElement>(null)

  // 加载文件列表
  const loadFiles = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const params: FileListParams = {
        page,
        page_size: 20,
        sort_by: sortBy,
        sort_order: sortOrder,
      }
      if (filterType) params.mime_type = filterType

      const data = await fileService.listFiles(params)
      setFiles(data.items)
      setTotalPages(data.total_pages)
    } catch (err: any) {
      setError(err.message || '加载文件列表失败')
    } finally {
      setLoading(false)
    }
  }, [page, filterType, sortBy, sortOrder])

  // 加载存储统计
  const loadStats = useCallback(async () => {
    try {
      const data = await fileService.getStorageStats()
      setStats(data)
    } catch (err) {
      console.error('Failed to load stats:', err)
    }
  }, [])

  useEffect(() => {
    loadFiles()
  }, [loadFiles])

  useEffect(() => {
    loadStats()
  }, [loadStats])

  // 上传文件
  const handleUpload = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const selectedFiles = event.target.files
    if (!selectedFiles || selectedFiles.length === 0) return

    setUploading(true)
    try {
      if (selectedFiles.length === 1) {
        await fileService.uploadFile(selectedFiles[0])
      } else {
        await fileService.batchUpload(Array.from(selectedFiles))
      }
      loadFiles()
      loadStats()
    } catch (err: any) {
      setError(err.message || '上传失败')
    } finally {
      setUploading(false)
      if (fileInputRef.current) {
        fileInputRef.current.value = ''
      }
    }
  }

  // 删除文件
  const handleDelete = async (file: FileItem) => {
    if (!confirm(`确定删除文件 "${file.original_filename}" 吗？`)) return
    try {
      await fileService.deleteFile(file.id)
      setFiles((prev) => prev.filter((f) => f.id !== file.id))
      loadStats()
    } catch (err: any) {
      setError(err.message || '删除失败')
    }
  }

  // MIME 类型过滤选项
  const typeFilters = [
    { value: '', label: '全部文件' },
    { value: 'image/', label: '🖼️ 图片' },
    { value: 'video/', label: '🎬 视频' },
    { value: 'audio/', label: '🎵 音频' },
    { value: 'application/pdf', label: '📄 PDF' },
    { value: 'text/', label: '📃 文本' },
  ]

  return (
    <div className="file-manager-page">
      {/* 顶部工具栏 */}
      <div className="file-toolbar">
        <div className="file-toolbar__left">
          <h1>📁 文件管理</h1>
          <StorageStatsCard stats={stats} />
        </div>

        <div className="file-toolbar__right">
          <input
            ref={fileInputRef}
            type="file"
            multiple
            onChange={handleUpload}
            style={{ display: 'none' }}
          />
          <button
            className="btn btn--primary"
            onClick={() => fileInputRef.current?.click()}
            disabled={uploading}
          >
            {uploading ? '⏳ 上传中...' : '📤 上传文件'}
          </button>
        </div>
      </div>

      {/* 过滤和排序 */}
      <div className="file-filters">
        <div className="file-filters__types">
          {typeFilters.map((filter) => (
            <button
              key={filter.value}
              className={`file-filter-btn ${filterType === filter.value ? 'active' : ''}`}
              onClick={() => {
                setFilterType(filter.value)
                setPage(1)
              }}
            >
              {filter.label}
            </button>
          ))}
        </div>

        <div className="file-filters__sort">
          <select
            value={sortBy}
            onChange={(e) => setSortBy(e.target.value as any)}
          >
            <option value="created_at">上传时间</option>
            <option value="size">文件大小</option>
            <option value="filename">文件名</option>
          </select>
          <button
            className="sort-order-btn"
            onClick={() => setSortOrder((o) => (o === 'asc' ? 'desc' : 'asc'))}
            title={sortOrder === 'asc' ? '升序' : '降序'}
          >
            {sortOrder === 'asc' ? '↑' : '↓'}
          </button>
        </div>

        <div className="file-filters__view">
          <button
            className={`view-btn ${viewMode === 'grid' ? 'active' : ''}`}
            onClick={() => setViewMode('grid')}
            title="网格视图"
          >
            ▦
          </button>
          <button
            className={`view-btn ${viewMode === 'list' ? 'active' : ''}`}
            onClick={() => setViewMode('list')}
            title="列表视图"
          >
            ☰
          </button>
        </div>
      </div>

      {/* 错误提示 */}
      {error && (
        <div className="file-error">
          <span>{error}</span>
          <button onClick={() => setError(null)}>✕</button>
        </div>
      )}

      {/* 文件列表 */}
      {loading ? (
        <div className="file-loading">加载中...</div>
      ) : files.length === 0 ? (
        <div className="file-empty">
          <span className="file-empty__icon">📭</span>
          <p>暂无文件</p>
          <button
            className="btn btn--primary"
            onClick={() => fileInputRef.current?.click()}
          >
            上传第一个文件
          </button>
        </div>
      ) : (
        <div className={`file-grid file-grid--${viewMode}`}>
          {files.map((file) => (
            <FileCard
              key={file.id}
              file={file}
              onPreview={setPreviewFile}
              onShare={setShareFile}
              onDelete={handleDelete}
            />
          ))}
        </div>
      )}

      {/* 分页 */}
      {totalPages > 1 && (
        <div className="file-pagination">
          <button
            className="btn btn--small btn--secondary"
            onClick={() => setPage((p) => Math.max(1, p - 1))}
            disabled={page === 1}
          >
            ← 上一页
          </button>
          <span className="file-pagination__info">
            第 {page} / {totalPages} 页
          </span>
          <button
            className="btn btn--small btn--secondary"
            onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
            disabled={page === totalPages}
          >
            下一页 →
          </button>
        </div>
      )}

      {/* 预览对话框 */}
      {previewFile && (
        <FilePreviewDialog
          file={previewFile}
          onClose={() => setPreviewFile(null)}
        />
      )}

      {/* 分享对话框 */}
      {shareFile && (
        <ShareDialog
          file={shareFile}
          onClose={() => setShareFile(null)}
        />
      )}
    </div>
  )
}
