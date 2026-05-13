use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use axum::extract::ws::Message;
use uuid::Uuid;

use crate::models::WSMessage;

/// WebSocket连接ID (唯一标识每个连接)
pub type ConnectionId = Uuid;

/// WebSocket连接
#[derive(Debug, Clone)]
pub struct WSConnection {
    /// 连接ID (唯一标识)
    pub connection_id: ConnectionId,
    /// 用户ID
    pub user_id: Uuid,
    /// 当前会话ID (可选)
    pub conversation_id: Option<Uuid>,
    /// 客户端地址描述
    pub addr: String,
    /// 消息发送通道
    pub sender: tokio::sync::mpsc::UnboundedSender<Message>,
    /// 连接时间戳
    pub connected_at: i64,
    /// 最后活跃时间戳 (用于心跳检测)
    pub last_active_at: i64,
}

/// WebSocket连接管理器
/// 支持同一用户多设备连接
#[derive(Clone)]
pub struct WSConnectionManager {
    /// 所有活跃连接: connection_id -> connection
    connections: Arc<RwLock<HashMap<ConnectionId, WSConnection>>>,
    /// 用户的连接列表: user_id -> vec<connection_id>
    user_connections: Arc<RwLock<HashMap<Uuid, Vec<ConnectionId>>>>,
    /// 会话连接映射: conversation_id -> vec<connection_id>
    conversation_connections: Arc<RwLock<HashMap<Uuid, Vec<ConnectionId>>>>,
}

impl WSConnectionManager {
    /// 创建新的连接管理器
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            user_connections: Arc::new(RwLock::new(HashMap::new())),
            conversation_connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 添加连接
    pub async fn add_connection(&self, connection: WSConnection) {
        let connection_id = connection.connection_id;
        let user_id = connection.user_id;

        // 添加到总连接表
        let mut connections = self.connections.write().await;
        connections.insert(connection_id, connection.clone());

        // 添加到用户连接表
        let mut user_connections = self.user_connections.write().await;
        user_connections
            .entry(user_id)
            .or_insert_with(Vec::new)
            .push(connection_id);

        tracing::info!(
            "User {} connected with connection {} (addr: {})",
            user_id,
            connection_id,
            connection.addr
        );
    }

    /// 移除连接
    pub async fn remove_connection(&self, connection_id: ConnectionId) -> Option<WSConnection> {
        // 从总连接表获取并移除
        let mut connections = self.connections.write().await;
        let connection = connections.remove(&connection_id);

        if let Some(conn) = connection {
            // 从用户连接表中移除
            let mut user_connections = self.user_connections.write().await;
            if let Some(conn_ids) = user_connections.get_mut(&conn.user_id) {
                conn_ids.retain(|id| *id != connection_id);
                if conn_ids.is_empty() {
                    user_connections.remove(&conn.user_id);
                }
            }

            // 从会话连接表中移除
            if let Some(conversation_id) = conn.conversation_id {
                let mut conv_connections = self.conversation_connections.write().await;
                if let Some(conn_ids) = conv_connections.get_mut(&conversation_id) {
                    conn_ids.retain(|id| *id != connection_id);
                    if conn_ids.is_empty() {
                        conv_connections.remove(&conversation_id);
                    }
                }
            }

            tracing::info!(
                "User {} disconnected (connection: {}, addr: {})",
                conn.user_id,
                connection_id,
                conn.addr
            );

            return Some(conn);
        }

        None
    }

    /// 获取连接
    pub async fn get_connection(&self, connection_id: ConnectionId) -> Option<WSConnection> {
        let connections = self.connections.read().await;
        connections.get(&connection_id).cloned()
    }

    /// 获取用户的所有连接
    pub async fn get_user_connections(&self, user_id: Uuid) -> Vec<WSConnection> {
        let connections = self.connections.read().await;
        let user_connections = self.user_connections.read().await;

        if let Some(conn_ids) = user_connections.get(&user_id) {
            conn_ids
                .iter()
                .filter_map(|id| connections.get(id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// 更新连接的会话ID
    pub async fn set_conversation(
        &self,
        connection_id: ConnectionId,
        conversation_id: Uuid,
    ) -> Option<WSConnection> {
        let mut connections = self.connections.write().await;

        if let Some(conn) = connections.get_mut(&connection_id) {
            // 从旧会话中移除
            if let Some(old_conversation_id) = conn.conversation_id {
                let mut conv_connections = self.conversation_connections.write().await;
                if let Some(conn_ids) = conv_connections.get_mut(&old_conversation_id) {
                    conn_ids.retain(|id| *id != connection_id);
                    if conn_ids.is_empty() {
                        conv_connections.remove(&old_conversation_id);
                    }
                }
            }

            conn.conversation_id = Some(conversation_id);

            // 添加到新会话
            let mut conv_connections = self.conversation_connections.write().await;
            conv_connections
                .entry(conversation_id)
                .or_insert_with(Vec::new)
                .push(connection_id);

            return Some(conn.clone());
        }

        None
    }

    /// 更新连接的最后活跃时间
    pub async fn update_last_active(&self, connection_id: ConnectionId) {
        let mut connections = self.connections.write().await;
        if let Some(conn) = connections.get_mut(&connection_id) {
            conn.last_active_at = chrono::Utc::now().timestamp();
        }
    }

    /// 检查用户是否在线
    pub async fn is_online(&self, user_id: Uuid) -> bool {
        let user_connections = self.user_connections.read().await;
        user_connections
            .get(&user_id)
            .map(|ids| !ids.is_empty())
            .unwrap_or(false)
    }

    /// 获取在线用户ID列表
    pub async fn get_online_users(&self) -> Vec<Uuid> {
        let user_connections = self.user_connections.read().await;
        user_connections.keys().cloned().collect()
    }

    /// 获取所有活跃连接ID
    pub async fn get_all_connection_ids(&self) -> Vec<ConnectionId> {
        let connections = self.connections.read().await;
        connections.keys().cloned().collect()
    }

    /// 向用户的所有设备发送消息
    pub async fn send_to_user(&self, user_id: Uuid, message: WSMessage) -> usize {
        let user_connections = self.user_connections.read().await;
        let connections = self.connections.read().await;

        if let Some(conn_ids) = user_connections.get(&user_id) {
            if let Ok(json) = serde_json::to_string(&message) {
                let ws_message = Message::Text(json);
                let mut sent_count = 0;

                for conn_id in conn_ids {
                    if let Some(conn) = connections.get(conn_id) {
                        if conn.sender.send(ws_message.clone()).is_ok() {
                            sent_count += 1;
                        } else {
                            tracing::warn!(
                                "Failed to send message to connection {} of user {}",
                                conn_id,
                                user_id
                            );
                        }
                    }
                }

                return sent_count;
            }
        }

        0
    }

    /// 向会话中的所有连接发送消息
    pub async fn send_to_conversation(&self, conversation_id: Uuid, message: WSMessage) -> usize {
        let conv_connections = self.conversation_connections.read().await;
        let connections = self.connections.read().await;

        if let Some(conn_ids) = conv_connections.get(&conversation_id) {
            if let Ok(json) = serde_json::to_string(&message) {
                let ws_message = Message::Text(json);
                let mut sent_count = 0;

                for conn_id in conn_ids {
                    if let Some(conn) = connections.get(conn_id) {
                        if conn.sender.send(ws_message.clone()).is_ok() {
                            sent_count += 1;
                        }
                    }
                }

                return sent_count;
            }
        }

        0
    }

    /// 向会话中的所有连接发送消息（排除指定用户）
    pub async fn send_to_conversation_except(
        &self,
        conversation_id: Uuid,
        exclude_user_id: Uuid,
        message: WSMessage,
    ) -> usize {
        let conv_connections = self.conversation_connections.read().await;
        let connections = self.connections.read().await;
        let user_connections = self.user_connections.read().await;

        if let Some(conn_ids) = conv_connections.get(&conversation_id) {
            if let Ok(json) = serde_json::to_string(&message) {
                let ws_message = Message::Text(json);
                let mut sent_count = 0;

                // 获取排除用户的连接ID集合
                let exclude_conns: std::collections::HashSet<ConnectionId> = user_connections
                    .get(&exclude_user_id)
                    .map(|c| c.iter().cloned().collect())
                    .unwrap_or_default();

                for conn_id in conn_ids {
                    // 跳过排除用户的连接
                    if exclude_conns.contains(conn_id) {
                        continue;
                    }
                    if let Some(conn) = connections.get(conn_id) {
                        if conn.sender.send(ws_message.clone()).is_ok() {
                            sent_count += 1;
                        }
                    }
                }

                return sent_count;
            }
        }

        0
    }

    /// 向会话中的所有连接发送消息（过滤屏蔽用户）
    ///
    /// 排除 sender_id 屏蔽的用户以及屏蔽了 sender_id 的用户的连接。
    /// blocked_by_sender: 被发送者屏蔽的用户ID集合
    /// blocked_senders: 屏蔽了发送者的用户ID集合
    pub async fn send_to_conversation_filtered(
        &self,
        conversation_id: Uuid,
        sender_id: Uuid,
        blocked_by_sender: &std::collections::HashSet<Uuid>,
        blocked_senders: &std::collections::HashSet<Uuid>,
        message: WSMessage,
    ) -> usize {
        let conv_connections = self.conversation_connections.read().await;
        let connections = self.connections.read().await;

        if let Some(conn_ids) = conv_connections.get(&conversation_id) {
            if let Ok(json) = serde_json::to_string(&message) {
                let ws_message = Message::Text(json);
                let mut sent_count = 0;

                for conn_id in conn_ids {
                    if let Some(conn) = connections.get(conn_id) {
                        // 跳过发送者自己
                        if conn.user_id == sender_id {
                            continue;
                        }
                        // 跳过被发送者屏蔽的用户
                        if blocked_by_sender.contains(&conn.user_id) {
                            continue;
                        }
                        // 跳过屏蔽了发送者的用户
                        if blocked_senders.contains(&conn.user_id) {
                            continue;
                        }
                        if conn.sender.send(ws_message.clone()).is_ok() {
                            sent_count += 1;
                        }
                    }
                }

                return sent_count;
            }
        }

        0
    }

    /// 向特定连接发送消息
    pub async fn send_to_connection(
        &self,
        connection_id: ConnectionId,
        message: WSMessage,
    ) -> bool {
        let connections = self.connections.read().await;

        if let Some(conn) = connections.get(&connection_id) {
            if let Ok(json) = serde_json::to_string(&message) {
                let ws_message = Message::Text(json);
                return conn.sender.send(ws_message).is_ok();
            }
        }

        false
    }

    /// 广播消息给所有在线用户
    pub async fn broadcast(&self, message: WSMessage) -> usize {
        let connections = self.connections.read().await;

        if let Ok(json) = serde_json::to_string(&message) {
            let ws_message = Message::Text(json);
            let mut sent_count = 0;

            for conn in connections.values() {
                if conn.sender.send(ws_message.clone()).is_ok() {
                    sent_count += 1;
                }
            }

            return sent_count;
        }

        0
    }

    /// 获取在线用户数
    pub async fn online_count(&self) -> usize {
        let user_connections = self.user_connections.read().await;
        user_connections.len()
    }

    /// 获取总连接数
    pub async fn connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    /// 获取会话中的在线连接数
    pub async fn conversation_online_count(&self, conversation_id: Uuid) -> usize {
        let conv_connections = self.conversation_connections.read().await;
        conv_connections
            .get(&conversation_id)
            .map(|ids| ids.len())
            .unwrap_or(0)
    }

    /// 获取超时的连接 (超过指定秒数未活动)
    pub async fn get_inactive_connections(
        &self,
        timeout_seconds: i64,
    ) -> Vec<ConnectionId> {
        let connections = self.connections.read().await;
        let now = chrono::Utc::now().timestamp();
        let threshold = now - timeout_seconds;

        connections
            .iter()
            .filter(|(_, conn)| conn.last_active_at < threshold)
            .map(|(id, _)| *id)
            .collect()
    }

    /// 清理不活跃的连接
    ///
    /// 移除超过指定时间未活动的连接，返回被移除的连接列表。
    /// 用于定期心跳检测，防止僵尸连接占用资源。
    ///
    /// # 参数
    /// - `timeout_seconds`: 超时时间（秒），默认建议 300 秒（5分钟）
    ///
    /// # 返回
    /// 被移除的连接ID列表，可用于通知前端连接已断开
    pub async fn cleanup_stale_connections(&self, timeout_seconds: i64) -> Vec<ConnectionId> {
        let stale_ids = self.get_inactive_connections(timeout_seconds).await;

        for conn_id in &stale_ids {
            self.remove_connection(*conn_id).await;
        }

        if !stale_ids.is_empty() {
            tracing::info!(
                "Cleaned up {} stale WebSocket connections (timeout: {}s)",
                stale_ids.len(),
                timeout_seconds
            );
        }

        stale_ids
    }

    /// 启动连接池心跳清理后台任务
    ///
    /// 每隔 `check_interval` 秒检查一次，移除超过 `timeout_seconds` 未活动的连接。
    /// 返回 JoinHandle 以便在需要时取消任务。
    ///
    /// # 参数
    /// - `check_interval`: 检查间隔（秒），建议 30-60 秒
    /// - `timeout_seconds`: 连接超时时间（秒），建议 300 秒（5分钟）
    pub fn start_heartbeat_task(
        self: &Arc<Self>,
        check_interval: u64,
        timeout_seconds: i64,
    ) -> tokio::task::JoinHandle<()> {
        let manager = Arc::clone(self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                tokio::time::Duration::from_secs(check_interval),
            );
            loop {
                interval.tick().await;
                let stale = manager.cleanup_stale_connections(timeout_seconds).await;
                if !stale.is_empty() {
                    tracing::info!(
                        "Heartbeat: cleaned {} stale connections, {} active connections remain",
                        stale.len(),
                        manager.connection_count().await
                    );
                }
            }
        })
    }

    /// 获取连接池状态摘要
    ///
    /// 返回 (total_connections, online_users, active_conversations)
    pub async fn pool_status(&self) -> (usize, usize, usize) {
        let connections = self.connections.read().await;
        let user_connections = self.user_connections.read().await;
        let conv_connections = self.conversation_connections.read().await;

        (connections.len(), user_connections.len(), conv_connections.len())
    }
}

impl Default for WSConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}
