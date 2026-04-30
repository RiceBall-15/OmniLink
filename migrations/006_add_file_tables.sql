-- 文件表
CREATE TABLE IF NOT EXISTS files (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    filename VARCHAR(255) NOT NULL,
    original_name VARCHAR(255) NOT NULL,
    file_path TEXT NOT NULL,
    file_size BIGINT NOT NULL,
    mime_type VARCHAR(100) NOT NULL,
    file_type VARCHAR(20) NOT NULL, -- 'image', 'video', 'audio', 'document', 'other'
    width INTEGER, -- 图片宽度
    height INTEGER, -- 图片高度
    duration INTEGER, -- 视频时长(秒)
    thumbnail_path TEXT, -- 缩略图路径
    storage_type VARCHAR(20) NOT NULL DEFAULT 'local', -- 'local', 'minio', 's3'
    is_public BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    CONSTRAINT fk_files_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_files_user_id ON files(user_id);
CREATE INDEX IF NOT EXISTS idx_files_file_type ON files(file_type);
CREATE INDEX IF NOT EXISTS idx_files_created_at ON files(created_at);
CREATE INDEX IF NOT EXISTS idx_files_storage_type ON files(storage_type);

-- 添加注释
COMMENT ON TABLE files IS '文件信息表';
COMMENT ON COLUMN files.file_type IS '文件类型：image/video/audio/document/other';
COMMENT ON COLUMN files.storage_type IS '存储类型：local/minio/s3';
COMMENT ON COLUMN files.width IS '图片宽度(像素)';
COMMENT ON COLUMN files.height IS '图片高度(像素)';
COMMENT ON COLUMN files.duration IS '视频时长(秒)';
COMMENT ON COLUMN files.thumbnail_path IS '缩略图路径';