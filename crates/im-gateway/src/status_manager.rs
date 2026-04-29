use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;

/// 用户状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    Online,
    Away,
    Busy,
    Offline,
}

/// 用户状态信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct UserStatusInfo {
    pub user_id: Uuid,
    pub status: UserStatus,
    pub last_seen: i64,
    pub device_info: Option<String>,
}

/// 在线状态管理器
#[derive(Clone)]
pub struct OnlineStatusManager {
    statuses: Arc<RwLock<HashMap<Uuid, UserStatusInfo>>>,
}

impl OnlineStatusManager {
    pub fn new() -> Self {
        Self {
            statuses: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 设置用户在线
    pub async fn set_online(&self, user_id: Uuid, device_info: Option<String>) {
        let now = Utc::now().timestamp();
        let mut statuses = self.statuses.write().await;

        statuses.insert(
            user_id,
            UserStatusInfo {
                user_id,
                status: UserStatus::Online,
                last_seen: now,
                device_info,
            },
        );

        tracing::info!("User {} is now online", user_id);
    }

    /// 设置用户离线
    pub async fn set_offline(&self, user_id: Uuid) {
        let now = Utc::now().timestamp();
        let mut statuses = self.statuses.write().await;

        if let Some(status) = statuses.get_mut(&user_id) {
            status.status = UserStatus::Offline;
            status.last_seen = now;
        }

        tracing::info!("User {} is now offline", user_id);
    }

    /// 更新用户状态
    pub async fn update_status(&self, user_id: Uuid, status: UserStatus) {
        let now = Utc::now().timestamp();
        let mut statuses = self.statuses.write().await;

        if let Some(user_status) = statuses.get_mut(&user_id) {
            user_status.status = status;
            user_status.last_seen = now;
        }
    }

    /// 获取用户状态
    pub async fn get_status(&self, user_id: Uuid) -> Option<UserStatusInfo> {
        let statuses = self.statuses.read().await;
        statuses.get(&user_id).cloned()
    }

    /// 检查用户是否在线
    pub async fn is_online(&self, user_id: Uuid) -> bool {
        let statuses = self.statuses.read().await;
        statuses
            .get(&user_id)
            .map(|s| matches!(s.status, UserStatus::Online))
            .unwrap_or(false)
    }

    /// 获取所有在线用户
    pub async fn get_online_users(&self) -> Vec<UserStatusInfo> {
        let statuses = self.statuses.read().await;
        statuses
            .values()
            .filter(|s| matches!(s.status, UserStatus::Online))
            .cloned()
            .collect()
    }

    /// 获取在线用户ID列表
    pub async fn get_online_user_ids(&self) -> Vec<Uuid> {
        let statuses = self.statuses.read().await;
        statuses
            .keys()
            .filter(|uid| {
                if let Some(status) = statuses.get(uid) {
                    matches!(status.status, UserStatus::Online)
                } else {
                    false
                }
            })
            .cloned()
            .collect()
    }

    /// 获取在线用户数
    pub async fn online_count(&self) -> usize {
        let statuses = self.statuses.read().await;
        statuses
            .values()
            .filter(|s| matches!(s.status, UserStatus::Online))
            .count()
    }

    /// 清理过期的在线状态 (超过5分钟未活动)
    pub async fn cleanup_expired(&self) {
        let now = Utc::now().timestamp();
        let expiration_threshold = 5 * 60; // 5分钟

        let mut statuses = self.statuses.write().await;
        let mut to_remove = Vec::new();

        for (user_id, status) in statuses.iter() {
            if matches!(status.status, UserStatus::Online | UserStatus::Away | UserStatus::Busy) {
                if now - status.last_seen > expiration_threshold {
                    to_remove.push(*user_id);
                }
            }
        }

        for user_id in to_remove {
            tracing::info!("Cleaning up expired status for user {}", user_id);
            statuses.remove(&user_id);
        }
    }

    /// 批量获取用户状态
    pub async fn get_batch_statuses(&self, user_ids: Vec<Uuid>) -> HashMap<Uuid, UserStatusInfo> {
        let statuses = self.statuses.read().await;
        user_ids
            .into_iter()
            .filter_map(|uid| statuses.get(&uid).map(|s| (uid, s.clone())))
            .collect()
    }
}

impl Default for OnlineStatusManager {
    fn default() -> Self {
        Self::new()
    }
}