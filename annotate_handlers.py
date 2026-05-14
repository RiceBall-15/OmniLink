#!/usr/bin/env python3
"""Batch-add utoipa::path annotations to handler functions."""

import sys

# Define annotations for each handler file: list of (old_text, new_text) pairs
annotations = {
    "crates/im-api/src/handlers/contact.rs": [
        ("""/// 添加联系人
///
/// POST /api/users/contacts
pub async fn add_contact_handler(""",
         """/// 添加联系人
///
/// POST /api/users/contacts
#[utoipa::path(
    post,
    path = "/api/users/contacts",
    tag = "contacts",
    responses(
        (status = 201, description = "添加成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn add_contact_handler("""),
        ("""/// 获取联系人列表
///
/// GET /api/users/contacts
pub async fn get_contacts_handler(""",
         """/// 获取联系人列表
///
/// GET /api/users/contacts
#[utoipa::path(
    get,
    path = "/api/users/contacts",
    tag = "contacts",
    responses(
        (status = 200, description = "获取成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn get_contacts_handler("""),
        ("""/// 搜索用户
///
/// GET /api/users/search?q=keyword
pub async fn search_users_handler(""",
         """/// 搜索用户
///
/// GET /api/users/search?q=keyword
#[utoipa::path(
    get,
    path = "/api/users/search",
    tag = "contacts",
    params(("q" = String, Query, description = "搜索关键词")),
    responses(
        (status = 200, description = "搜索成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn search_users_handler("""),
    ],
    "crates/im-api/src/handlers/announcement.rs": [
        ("""/// 创建系统公告（管理员）
pub async fn create_announcement_handler(""",
         """/// 创建系统公告（管理员）
#[utoipa::path(
    post,
    path = "/api/announcements",
    tag = "announcements",
    responses(
        (status = 201, description = "创建成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn create_announcement_handler("""),
        ("""/// 获取公告列表（管理员视图）
pub async fn get_all_announcements_handler(""",
         """/// 获取公告列表（管理员视图）
#[utoipa::path(
    get,
    path = "/api/announcements/all",
    tag = "announcements",
    responses(
        (status = 200, description = "获取成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn get_all_announcements_handler("""),
        ("""/// 获取活跃公告列表（用户视图，含已读状态）
pub async fn get_active_announcements_handler(""",
         """/// 获取活跃公告列表（用户视图，含已读状态）
#[utoipa::path(
    get,
    path = "/api/announcements",
    tag = "announcements",
    responses(
        (status = 200, description = "获取成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn get_active_announcements_handler("""),
        ("""/// 标记公告为已读
pub async fn mark_announcement_read_handler(""",
         """/// 标记公告为已读
#[utoipa::path(
    post,
    path = "/api/announcements/{id}/read",
    tag = "announcements",
    params(("id" = String, Path, description = "公告ID")),
    responses(
        (status = 200, description = "标记成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn mark_announcement_read_handler("""),
    ],
    "crates/im-api/src/handlers/quick_reply.rs": [
        ("""/// 创建快捷回复
pub async fn create_quick_reply_handler(""",
         """/// 创建快捷回复
#[utoipa::path(
    post,
    path = "/api/im/quick-replies",
    tag = "quick-replies",
    responses(
        (status = 201, description = "创建成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn create_quick_reply_handler("""),
        ("""/// 获取快捷回复列表
pub async fn get_quick_replies_handler(""",
         """/// 获取快捷回复列表
#[utoipa::path(
    get,
    path = "/api/im/quick-replies",
    tag = "quick-replies",
    responses(
        (status = 200, description = "获取成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn get_quick_replies_handler("""),
    ],
    "crates/im-api/src/handlers/feedback.rs": [
        ("""/// 提交反馈
pub async fn submit_feedback_handler(""",
         """/// 提交反馈
#[utoipa::path(
    post,
    path = "/api/feedbacks",
    tag = "feedbacks",
    responses(
        (status = 201, description = "提交成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn submit_feedback_handler("""),
        ("""/// 获取所有反馈（管理员）
pub async fn get_all_feedbacks_handler(""",
         """/// 获取所有反馈（管理员）
#[utoipa::path(
    get,
    path = "/api/feedbacks/all",
    tag = "feedbacks",
    responses(
        (status = 200, description = "获取成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn get_all_feedbacks_handler("""),
    ],
    "crates/im-api/src/handlers/chat_export.rs": [
        ("""/// 创建导出任务
pub async fn create_export_job_handler(""",
         """/// 创建导出任务
#[utoipa::path(
    post,
    path = "/api/im/exports",
    tag = "chat-export",
    responses(
        (status = 201, description = "创建成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn create_export_job_handler("""),
        ("""/// 下载导出文件
pub async fn download_export_file_handler(""",
         """/// 下载导出文件
#[utoipa::path(
    get,
    path = "/api/im/exports/{id}/download",
    tag = "chat-export",
    params(("id" = String, Path, description = "导出任务ID")),
    responses(
        (status = 200, description = "下载成功"),
    )
)]
pub async fn download_export_file_handler("""),
    ],
    "crates/im-api/src/handlers/message_retry.rs": [
        ("""/// 手动重试失败消息
///
/// POST /api/im/messages/:id/retry
pub async fn retry_message_handler(""",
         """/// 手动重试失败消息
///
/// POST /api/im/messages/:id/retry
#[utoipa::path(
    post,
    path = "/api/im/messages/{id}/retry",
    tag = "message-retry",
    params(("id" = String, Path, description = "消息ID")),
    responses(
        (status = 200, description = "重试成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn retry_message_handler("""),
        ("""/// 获取用户失败消息列表
///
/// GET /api/im/messages/failed
pub async fn get_failed_messages_handler(""",
         """/// 获取用户失败消息列表
///
/// GET /api/im/messages/failed
#[utoipa::path(
    get,
    path = "/api/im/messages/failed",
    tag = "message-retry",
    responses(
        (status = 200, description = "获取成功", body = ApiResponse<serde_json::Value>),
    )
)]
pub async fn get_failed_messages_handler("""),
    ],
}

total = 0
success = 0
for filepath, pairs in annotations.items():
    try:
        with open(filepath, 'r') as f:
            content = f.read()
    except FileNotFoundError:
        print(f"❌ File not found: {filepath}")
        continue
    
    modified = False
    for old, new in pairs:
        total += 1
        if old in content:
            content = content.replace(old, new, 1)
            success += 1
            print(f"  ✅ Annotated in {filepath}")
            modified = True
        else:
            print(f"  ❌ Pattern not found in {filepath}: {repr(old[:60])}...")
    
    if modified:
        with open(filepath, 'w') as f:
            f.write(content)

print(f"\nDone: {success}/{total} annotations applied.")
