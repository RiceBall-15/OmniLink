use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;

/// 用户状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    Online,
    Away,
    Busy,
    Offline,
}

impl UserStatus {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "away" => UserStatus::Away,
            "busy" => UserStatus::Busy,
            "offline" => UserStatus::Offline,
            _ => UserStatus::Online,
        }
    }
}

/// 用户状态信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserStatusInfo {
    pub user_id: Uuid,
    pub status: UserStatus,
    pub last_seen: i64,
    pub device_info: Option<String>,
}

/// 状态变更事件
#[derive(Debug, Clone)]
pub struct StatusChangeEvent {
    pub user_id: Uuid,
    pub old_status: Option<UserStatus>,
    pub new_status: UserStatus,
    pub timestamp: i64,
}

/// 状态变更监听器类型
pub type StatusChangeListener = Arc<dyn Fn(StatusChangeEvent) + Send + Sync + 'static>;

/// 在线状态管理器
/// 支持 Redis 持久化 + 内存缓存
#[derive(Clone)]
pub struct OnlineStatusManager {
    /// 内存缓存（快速读取）
    statuses: Arc<RwLock<HashMap<Uuid, UserStatusInfo>>>,
    /// Redis 连接
    redis: Option<ConnectionManager>,
    /// 状态变更监听器
    listeners: Arc<RwLock<Vec<StatusChangeListener>>>,
}

impl OnlineStatusManager {
    pub fn new() -> Self {
        Self {
            statuses: Arc::new(RwLock::new(HashMap::new())),
            redis: None,
            listeners: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 创建带 Redis 的实例
    pub fn with_redis(redis: ConnectionManager) -> Self {
        Self {
            statuses: Arc::new(RwLock::new(HashMap::new())),
            redis: Some(redis),
            listeners: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 注册状态变更监听器
    pub async fn on_status_change(&self, listener: StatusChangeListener) {
        let mut listeners = self.listeners.write().await;
        listeners.push(listener);
    }

    /// 通知状态变更
    async fn notify_change(&self, event: StatusChangeEvent) {
        let listeners = self.listeners.read().await;
        for listener in listeners.iter() {
            listener(event.clone());
        }
    }

    /// 设置用户在线
    pub async fn set_online(&self, user_id: Uuid, device_info: Option<String>) {
        let now = Utc::now().timestamp();
        let old_status = {
            let statuses = self.statuses.read().await;
            statuses.get(&user_id).map(|s| s.status.clone())
        };

        let status_info = UserStatusInfo {
            user_id,
            status: UserStatus::Online,
            last_seen: now,
            device_info: device_info.clone(),
        };

        // 更新内存缓存
        {
            let mut statuses = self.statuses.write().await;
            statuses.insert(user_id, status_info.clone());
        }

        // 持久化到 Redis
        if let Some(ref redis) = self.redis {
            let mut redis = redis.clone();
            let redis_key = format!("user:status:{}", user_id);
            let status_json = serde_json::to_string(&status_info).unwrap_or_default();
            let _: Result<(), _> = redis.set_ex(&redis_key, &status_json, 3600).await; // 1小时TTL
            // 添加到在线用户集合
            let _: Result<(), _> = redis.sadd("users:online", user_id.to_string()).await;
        }

        tracing::info!("User {} is now online", user_id);

        // 通知状态变更
        self.notify_change(StatusChangeEvent {
            user_id,
            old_status,
            new_status: UserStatus::Online,
            timestamp: now,
        }).await;
    }

    /// 设置用户离线
    pub async fn set_offline(&self, user_id: Uuid) {
        let now = Utc::now().timestamp();
        let old_status = {
            let statuses = self.statuses.read().await;
            statuses.get(&user_id).map(|s| s.status.clone())
        };

        // 更新内存缓存
        {
            let mut statuses = self.statuses.write().await;
            if let Some(status) = statuses.get_mut(&user_id) {
                status.status = UserStatus::Offline;
                status.last_seen = now;
            } else {
                statuses.insert(
                    user_id,
                    UserStatusInfo {
                        user_id,
                        status: UserStatus::Offline,
                        last_seen: now,
                        device_info: None,
                    },
                );
            }
        }

        // 持久化到 Redis
        if let Some(ref redis) = self.redis {
            let mut redis = redis.clone();
            let redis_key = format!("user:status:{}", user_id);
            let status_info = UserStatusInfo {
                user_id,
                status: UserStatus::Offline,
                last_seen: now,
                device_info: None,
            };
            let status_json = serde_json::to_string(&status_info).unwrap_or_default();
            let _: Result<(), _> = redis.set_ex(&redis_key, &status_json, 86400).await; // 24小时TTL（离线保留更久）
            // 从在线集合移除
            let _: Result<(), _> = redis.srem("users:online", user_id.to_string()).await;
        }

        tracing::info!("User {} is now offline", user_id);

        // 通知状态变更
        self.notify_change(StatusChangeEvent {
            user_id,
            old_status,
            new_status: UserStatus::Offline,
            timestamp: now,
        }).await;
    }

    /// 更新用户状态（Away / Busy 等）
    pub async fn update_status(&self, user_id: Uuid, status: UserStatus) {
        let now = Utc::now().timestamp();
        let old_status = {
            let statuses = self.statuses.read().await;
            statuses.get(&user_id).map(|s| s.status.clone())
        };

        // 更新内存缓存
        {
            let mut statuses = self.statuses.write().await;
            if let Some(user_status) = statuses.get_mut(&user_id) {
                user_status.status = status.clone();
                user_status.last_seen = now;
            }
        }

        // 持久化到 Redis
        if let Some(ref redis) = self.redis {
            let mut redis = redis.clone();
            let redis_key = format!("user:status:{}", user_id);
            let statuses = self.statuses.read().await;
            if let Some(info) = statuses.get(&user_id) {
                let status_json = serde_json::to_string(info).unwrap_or_default();
                let _: Result<(), _> = redis.set_ex(&redis_key, &status_json, 3600).await;
            }
        }

        // 通知状态变更
        self.notify_change(StatusChangeEvent {
            user_id,
            old_status,
            new_status: status,
            timestamp: now,
        }).await;
    }

    /// 获取用户状态（先查内存，再查 Redis）
    pub async fn get_status(&self, user_id: Uuid) -> Option<UserStatusInfo> {
        // 先查内存
        {
            let statuses = self.statuses.read().await;
            if let Some(info) = statuses.get(&user_id) {
                return Some(info.clone());
            }
        }

        // 查 Redis
        if let Some(ref redis) = self.redis {
            let mut redis = redis.clone();
            let redis_key = format!("user:status:{}", user_id);
            if let Ok(status_json) = redis.get::<_, String>(&redis_key).await {
                if let Ok(info) = serde_json::from_str::<UserStatusInfo>(&status_json) {
                    // 回填到内存缓存
                    let mut statuses = self.statuses.write().await;
                    statuses.insert(user_id, info.clone());
                    return Some(info);
                }
            }
        }

        None
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
            .iter()
            .filter(|(_, s)| matches!(s.status, UserStatus::Online))
            .map(|(uid, _)| *uid)
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

    /// 清理过期的在线状态 (超过指定秒数未活动)
    pub async fn cleanup_expired(&self) {
        let now = Utc::now().timestamp();
        let expiration_threshold = 60; // 60秒，匹配心跳超时时间

        let mut to_remove = Vec::new();
        {
            let statuses = self.statuses.read().await;
            for (user_id, status) in statuses.iter() {
                if matches!(status.status, UserStatus::Online | UserStatus::Away | UserStatus::Busy)
                    && now - status.last_seen > expiration_threshold {
                        to_remove.push(*user_id);
                    }
            }
        }

        for user_id in to_remove {
            tracing::info!(
                "Cleaning up expired status for user {} (inactive for > {} seconds)",
                user_id,
                expiration_threshold
            );
            self.set_offline(user_id).await;
        }
    }

    /// 检查空闲用户并自动切换为 Away 状态
    ///
    /// 当用户在指定时间内没有活动时，自动从 Online 切换为 Away
    /// idle_threshold: 空闲阈值（秒），建议 300 秒（5分钟）
    pub async fn check_idle_users(&self, idle_threshold: i64) {
        let now = Utc::now().timestamp();
        let mut to_set_away = Vec::new();

        {
            let statuses = self.statuses.read().await;
            for (user_id, status) in statuses.iter() {
                if matches!(status.status, UserStatus::Online)
                    && now - status.last_seen > idle_threshold
                {
                    to_set_away.push(*user_id);
                }
            }
        }

        for user_id in to_set_away {
            tracing::info!(
                "User {} idle for > {} seconds, switching to Away",
                user_id,
                idle_threshold
            );
            self.update_status(user_id, UserStatus::Away).await;
        }
    }

    /// 启动自动状态管理后台任务
    ///
    /// 每 60 秒检查一次：
    /// - 空闲 5 分钟以上的用户自动切换为 Away
    /// - 空闲 60 秒以上的用户（含 Away）自动切换为 Offline
    pub fn start_auto_status_task(self: Arc<Self>) {
        let manager = self.clone();
        tokio::spawn(async move {
            tracing::info!("自动状态管理任务已启动（每60秒检查一次）");

            loop {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;

                // 检查空闲用户（5分钟无活动 → Away）
                manager.check_idle_users(300).await;

                // 清理过期用户（60秒无活动 → Offline）
                manager.cleanup_expired().await;
            }
        });
    }

    /// 批量获取用户状态
    pub async fn get_batch_statuses(&self, user_ids: &[Uuid]) -> HashMap<Uuid, UserStatusInfo> {
        let mut result = HashMap::new();
        let statuses = self.statuses.read().await;

        for &uid in user_ids {
            if let Some(info) = statuses.get(&uid) {
                result.insert(uid, info.clone());
            }
        }

        // 对于内存中没有的，批量查询 Redis
        if let Some(ref redis) = self.redis {
            let missing: Vec<Uuid> = user_ids
                .iter()
                .filter(|uid| !result.contains_key(uid))
                .copied()
                .collect();

            if !missing.is_empty() {
                let mut redis = redis.clone();
                let keys: Vec<String> = missing.iter().map(|uid| format!("user:status:{}", uid)).collect();
                if let Ok(values) = redis.mget::<_, Vec<Option<String>>>(&keys).await {
                    let mut statuses_mut = self.statuses.write().await;
                    for (i, value) in values.into_iter().enumerate() {
                        if let Some(json) = value {
                            if let Ok(info) = serde_json::from_str::<UserStatusInfo>(&json) {
                                result.insert(missing[i], info.clone());
                                statuses_mut.insert(missing[i], info);
                            }
                        }
                    }
                }
            }
        }

        result
    }

    /// 从 Redis 初始化内存缓存（启动时调用）
    pub async fn load_from_redis(&self) {
        if let Some(ref redis) = self.redis {
            let mut redis = redis.clone();
            // 获取所有在线用户
            if let Ok(online_ids) = redis.smembers::<_, Vec<String>>("users:online").await {
                let mut statuses = self.statuses.write().await;
                for id_str in online_ids {
                    if let Ok(uid) = Uuid::parse_str(&id_str) {
                        let redis_key = format!("user:status:{}", uid);
                        if let Ok(json) = redis.get::<_, String>(&redis_key).await {
                            if let Ok(info) = serde_json::from_str::<UserStatusInfo>(&json) {
                                statuses.insert(uid, info);
                            }
                        }
                    }
                }
                tracing::info!("Loaded {} user statuses from Redis", statuses.len());
            }
        }
    }

    /// 更新最后活跃时间（心跳）
    pub async fn touch(&self, user_id: Uuid) {
        let now = Utc::now().timestamp();
        let mut statuses = self.statuses.write().await;
        if let Some(status) = statuses.get_mut(&user_id) {
            status.last_seen = now;
        }
    }
}

impl Default for OnlineStatusManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // === UserStatus::from_str 测试 ===

    #[test]
    fn test_user_status_from_str_online() {
        assert!(matches!(UserStatus::from_str("online"), UserStatus::Online));
        assert!(matches!(UserStatus::from_str("Online"), UserStatus::Online));
        assert!(matches!(UserStatus::from_str("ONLINE"), UserStatus::Online));
        assert!(matches!(UserStatus::from_str(""), UserStatus::Online)); // 默认为 Online
    }

    #[test]
    fn test_user_status_from_str_away() {
        assert!(matches!(UserStatus::from_str("away"), UserStatus::Away));
        assert!(matches!(UserStatus::from_str("Away"), UserStatus::Away));
        assert!(matches!(UserStatus::from_str("AWAY"), UserStatus::Away));
    }

    #[test]
    fn test_user_status_from_str_busy() {
        assert!(matches!(UserStatus::from_str("busy"), UserStatus::Busy));
        assert!(matches!(UserStatus::from_str("Busy"), UserStatus::Busy));
        assert!(matches!(UserStatus::from_str("BUSY"), UserStatus::Busy));
    }

    #[test]
    fn test_user_status_from_str_offline() {
        assert!(matches!(UserStatus::from_str("offline"), UserStatus::Offline));
        assert!(matches!(UserStatus::from_str("Offline"), UserStatus::Offline));
        assert!(matches!(UserStatus::from_str("OFFLINE"), UserStatus::Offline));
    }

    #[test]
    fn test_user_status_from_str_unknown_defaults_to_online() {
        assert!(matches!(UserStatus::from_str("unknown"), UserStatus::Online));
        assert!(matches!(UserStatus::from_str("dnd"), UserStatus::Online));
    }

    // === OnlineStatusManager 基础操作测试 ===

    #[tokio::test]
    async fn test_new_manager_is_empty() {
        let manager = OnlineStatusManager::new();
        assert_eq!(manager.online_count().await, 0);
        assert!(manager.get_online_user_ids().await.is_empty());
    }

    #[tokio::test]
    async fn test_set_online_and_check() {
        let manager = OnlineStatusManager::new();
        let user_id = Uuid::new_v4();

        manager.set_online(user_id, Some("test-device".to_string())).await;

        assert!(manager.is_online(user_id).await);
        assert_eq!(manager.online_count().await, 1);
    }

    #[tokio::test]
    async fn test_set_online_get_status() {
        let manager = OnlineStatusManager::new();
        let user_id = Uuid::new_v4();

        manager.set_online(user_id, Some("mobile".to_string())).await;

        let status = manager.get_status(user_id).await;
        assert!(status.is_some());
        let info = status.unwrap();
        assert_eq!(info.user_id, user_id);
        assert!(matches!(info.status, UserStatus::Online));
        assert_eq!(info.device_info, Some("mobile".to_string()));
    }

    #[tokio::test]
    async fn test_set_offline() {
        let manager = OnlineStatusManager::new();
        let user_id = Uuid::new_v4();

        manager.set_online(user_id, None).await;
        assert!(manager.is_online(user_id).await);

        manager.set_offline(user_id).await;
        assert!(!manager.is_online(user_id).await);
        assert_eq!(manager.online_count().await, 0);
    }

    #[tokio::test]
    async fn test_set_offline_without_being_online() {
        let manager = OnlineStatusManager::new();
        let user_id = Uuid::new_v4();

        // 设置一个从未上线的用户为离线
        manager.set_offline(user_id).await;
        assert!(!manager.is_online(user_id).await);

        // 验证状态被正确记录
        let status = manager.get_status(user_id).await;
        assert!(status.is_some());
        assert!(matches!(status.unwrap().status, UserStatus::Offline));
    }

    #[tokio::test]
    async fn test_update_status_away() {
        let manager = OnlineStatusManager::new();
        let user_id = Uuid::new_v4();

        manager.set_online(user_id, None).await;
        manager.update_status(user_id, UserStatus::Away).await;

        // Away 不算在线
        assert!(!manager.is_online(user_id).await);

        let status = manager.get_status(user_id).await.unwrap();
        assert!(matches!(status.status, UserStatus::Away));
    }

    #[tokio::test]
    async fn test_update_status_busy() {
        let manager = OnlineStatusManager::new();
        let user_id = Uuid::new_v4();

        manager.set_online(user_id, None).await;
        manager.update_status(user_id, UserStatus::Busy).await;

        assert!(!manager.is_online(user_id).await);

        let status = manager.get_status(user_id).await.unwrap();
        assert!(matches!(status.status, UserStatus::Busy));
    }

    #[tokio::test]
    async fn test_update_status_nonexistent_user() {
        let manager = OnlineStatusManager::new();
        let user_id = Uuid::new_v4();

        // 更新不存在的用户状态不应 panic
        manager.update_status(user_id, UserStatus::Away).await;
        assert!(!manager.is_online(user_id).await);
    }

    // === 多用户测试 ===

    #[tokio::test]
    async fn test_multiple_users_online() {
        let manager = OnlineStatusManager::new();
        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();
        let user3 = Uuid::new_v4();

        manager.set_online(user1, None).await;
        manager.set_online(user2, Some("desktop".to_string())).await;
        manager.set_online(user3, None).await;

        assert_eq!(manager.online_count().await, 3);

        let ids = manager.get_online_user_ids().await;
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&user1));
        assert!(ids.contains(&user2));
        assert!(ids.contains(&user3));
    }

    #[tokio::test]
    async fn test_partial_offline() {
        let manager = OnlineStatusManager::new();
        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();

        manager.set_online(user1, None).await;
        manager.set_online(user2, None).await;
        assert_eq!(manager.online_count().await, 2);

        manager.set_offline(user1).await;
        assert_eq!(manager.online_count().await, 1);
        assert!(!manager.is_online(user1).await);
        assert!(manager.is_online(user2).await);
    }

    // === get_online_users 测试 ===

    #[tokio::test]
    async fn test_get_online_users_filter() {
        let manager = OnlineStatusManager::new();
        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();
        let user3 = Uuid::new_v4();

        manager.set_online(user1, None).await;
        manager.set_online(user2, None).await;
        manager.set_online(user3, None).await;
        manager.update_status(user2, UserStatus::Away).await;

        let online = manager.get_online_users().await;
        // 只有 user1 和 user3 是 Online 状态
        assert_eq!(online.len(), 2);
        assert!(online.iter().any(|u| u.user_id == user1));
        assert!(online.iter().any(|u| u.user_id == user3));
        assert!(!online.iter().any(|u| u.user_id == user2));
    }

    // === batch 操作测试 ===

    #[tokio::test]
    async fn test_get_batch_statuses() {
        let manager = OnlineStatusManager::new();
        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();
        let user3 = Uuid::new_v4();

        manager.set_online(user1, None).await;
        manager.set_online(user2, None).await;
        // user3 不在线

        let statuses = manager.get_batch_statuses(&[user1, user2, user3]).await;
        assert_eq!(statuses.len(), 2); // 只有 user1 和 user2
        assert!(statuses.contains_key(&user1));
        assert!(statuses.contains_key(&user2));
        assert!(!statuses.contains_key(&user3));
    }

    #[tokio::test]
    async fn test_get_batch_statuses_empty() {
        let manager = OnlineStatusManager::new();
        let statuses = manager.get_batch_statuses(&[]).await;
        assert!(statuses.is_empty());
    }

    // === touch (心跳) 测试 ===

    #[tokio::test]
    async fn test_touch_updates_last_seen() {
        let manager = OnlineStatusManager::new();
        let user_id = Uuid::new_v4();

        manager.set_online(user_id, None).await;
        let before = manager.get_status(user_id).await.unwrap().last_seen;

        // 等待一小段时间确保时间戳变化
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        manager.touch(user_id).await;
        let after = manager.get_status(user_id).await.unwrap().last_seen;
        assert!(after >= before);
    }

    #[tokio::test]
    async fn test_touch_nonexistent_user() {
        let manager = OnlineStatusManager::new();
        let user_id = Uuid::new_v4();
        // 不应 panic
        manager.touch(user_id).await;
    }

    // === cleanup_expired 测试 ===

    #[tokio::test]
    async fn test_cleanup_expired_removes_stale_users() {
        let manager = OnlineStatusManager::new();
        let user_id = Uuid::new_v4();

        // 直接插入一个过期的状态记录
        {
            let mut statuses = manager.statuses.write().await;
            statuses.insert(user_id, UserStatusInfo {
                user_id,
                status: UserStatus::Online,
                last_seen: Utc::now().timestamp() - 120, // 2分钟前
                device_info: None,
            });
        }

        assert!(manager.is_online(user_id).await);
        manager.cleanup_expired().await;
        assert!(!manager.is_online(user_id).await);
    }

    #[tokio::test]
    async fn test_cleanup_expired_keeps_active_users() {
        let manager = OnlineStatusManager::new();
        let user_id = Uuid::new_v4();

        manager.set_online(user_id, None).await;
        manager.cleanup_expired().await;

        // 刚设置的用户不应该被清理
        assert!(manager.is_online(user_id).await);
    }

    // === check_idle_users 测试 ===

    #[tokio::test]
    async fn test_check_idle_users_switches_to_away() {
        let manager = OnlineStatusManager::new();
        let user_id = Uuid::new_v4();

        // 插入一个空闲超过阈值的在线用户
        {
            let mut statuses = manager.statuses.write().await;
            statuses.insert(user_id, UserStatusInfo {
                user_id,
                status: UserStatus::Online,
                last_seen: Utc::now().timestamp() - 400, // 超过 300 秒阈值
                device_info: None,
            });
        }

        manager.check_idle_users(300).await;

        let status = manager.get_status(user_id).await.unwrap();
        assert!(matches!(status.status, UserStatus::Away));
    }

    #[tokio::test]
    async fn test_check_idle_users_keeps_active() {
        let manager = OnlineStatusManager::new();
        let user_id = Uuid::new_v4();

        manager.set_online(user_id, None).await;
        manager.check_idle_users(300).await;

        let status = manager.get_status(user_id).await.unwrap();
        assert!(matches!(status.status, UserStatus::Online));
    }

    // === 状态变更通知测试 ===

    #[tokio::test]
    async fn test_status_change_notification() {
        let manager = OnlineStatusManager::new();
        let user_id = Uuid::new_v4();

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        manager.on_status_change(Arc::new(move |_event: StatusChangeEvent| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        })).await;

        manager.set_online(user_id, None).await;
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        manager.set_offline(user_id).await;
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_status_change_event_content() {
        let manager = OnlineStatusManager::new();
        let user_id = Uuid::new_v4();

        let event = Arc::new(tokio::sync::Mutex::new(None::<StatusChangeEvent>));
        let event_clone = event.clone();

        manager.on_status_change(Arc::new(move |e: StatusChangeEvent| {
            // 使用 try_lock 因为我们不在 async 上下文中
            if let Ok(mut guard) = event_clone.try_lock() {
                *guard = Some(e);
            }
        })).await;

        manager.set_online(user_id, None).await;

        let captured = event.lock().await;
        assert!(captured.is_some());
        let e = captured.as_ref().unwrap();
        assert_eq!(e.user_id, user_id);
        assert!(e.old_status.is_none()); // 首次上线没有旧状态
        assert!(matches!(e.new_status, UserStatus::Online));
    }

    // === Default trait 测试 ===

    #[test]
    fn test_default_trait() {
        let manager = OnlineStatusManager::default();
        // Default 应该创建空管理器
        assert!(manager.redis.is_none());
    }

    // === 大量用户测试 ===

    #[tokio::test]
    async fn test_many_users_performance() {
        let manager = OnlineStatusManager::new();
        let user_count = 100;

        for _ in 0..user_count {
            manager.set_online(Uuid::new_v4(), None).await;
        }

        assert_eq!(manager.online_count().await, user_count);
        assert_eq!(manager.get_online_user_ids().await.len(), user_count);
    }

    // === UserStatusInfo 序列化测试 ===

    #[test]
    fn test_user_status_info_serialization() {
        let info = UserStatusInfo {
            user_id: Uuid::new_v4(),
            status: UserStatus::Online,
            last_seen: 1234567890,
            device_info: Some("desktop".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"online\""));
        assert!(json.contains("desktop"));

        let deserialized: UserStatusInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.user_id, info.user_id);
        assert_eq!(deserialized.last_seen, 1234567890);
    }

    #[test]
    fn test_user_status_info_serialization_no_device() {
        let info = UserStatusInfo {
            user_id: Uuid::new_v4(),
            status: UserStatus::Offline,
            last_seen: 0,
            device_info: None,
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: UserStatusInfo = serde_json::from_str(&json).unwrap();
        assert!(deserialized.device_info.is_none());
    }
}
