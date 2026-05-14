-- 修复导出任务表列名：file_url → file_path
-- 统一列名与 Rust 模型字段名一致
ALTER TABLE export_jobs RENAME COLUMN file_url TO file_path;

-- 添加 CSV 格式支持的注释
COMMENT ON COLUMN export_jobs.format IS '导出格式：json, csv, txt';
