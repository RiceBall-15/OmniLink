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
                if matches!(status.status, UserStatus::Online | UserStatus::Away | UserStatus::Busy) {
                    if now - status.last_seen > expiration_threshold {
                        to_remove.push(*user_id);
                    }
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
