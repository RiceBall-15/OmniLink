use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use std::net::SocketAddr;

use crate::models::{WSMessage, WSMessageType};

/// WebSocket连接
#[derive(Debug, Clone)]
pub struct WSConnection {
    pub user_id: Uuid,
    pub conversation_id: Option<Uuid>,
    pub addr: SocketAddr,
    pub sender: tokio::sync::mpsc::UnboundedSender<Message>,
    pub connected_at: i64,
}

/// WebSocket连接管理器
#[derive(Clone)]
pub struct WSConnectionManager {
    connections: Arc<RwLock<HashMap<Uuid, WSConnection>>>,
    conversation_connections: Arc<RwLock<HashMap<Uuid, Vec<Uuid>>>>, // conversation_id -> user_ids
}

impl WSConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            conversation_connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 添加连接
    pub async fn add_connection(&self, user_id: Uuid, connection: WSConnection) {
        let mut connections = self.connections.write().await;
        connections.insert(user_id, connection);
        tracing::info!("User {} connected", user_id);
    }

    /// 移除连接
    pub async fn remove_connection(&self, user_id: Uuid) {
        let mut connections = self.connections.write().await;
        if let Some(conn) = connections.remove(&user_id) {
            // 从对话连接中移除
            if let Some(conversation_id) = conn.conversation_id {
                let mut conv_connections = self.conversation_connections.write().await;
                if let Some(users) = conv_connections.get_mut(&conversation_id) {
                    users.retain(|uid| *uid != user_id);
                    if users.is_empty() {
                        conv_connections.remove(&conversation_id);
                    }
                }
            }
        }
        tracing::info!("User {} disconnected", user_id);
    }

    /// 获取连接
    pub async fn get_connection(&self, user_id: Uuid) -> Option<WSConnection> {
        let connections = self.connections.read().await;
        connections.get(&user_id).cloned()
    }

    /// 检查用户是否在线
    pub async fn is_online(&self, user_id: Uuid) -> bool {
        let connections = self.connections.read().await;
        connections.contains_key(&user_id)
    }

    /// 获取在线用户ID列表
    pub async fn get_online_users(&self) -> Vec<Uuid> {
        let connections = self.connections.read().await;
        connections.keys().cloned().collect()
    }

    /// 设置用户的当前对话
    pub async fn set_conversation(&self, user_id: Uuid, conversation_id: Uuid) {
        let mut connections = self.connections.write().await;
        if let Some(conn) = connections.get_mut(&user_id) {
            // 从旧对话中移除
            if let Some(old_conversation_id) = conn.conversation_id {
                let mut conv_connections = self.conversation_connections.write().await;
                if let Some(users) = conv_connections.get_mut(&old_conversation_id) {
                    users.retain(|uid| *uid != user_id);
                }
            }

            conn.conversation_id = Some(conversation_id);

            // 添加到新对话
            let mut conv_connections = self.conversation_connections.write().await;
            conv_connections
                .entry(conversation_id)
                .or_insert_with(Vec::new)
                .push(user_id);
        }
    }

    /// 向用户发送消息
    pub async fn send_to_user(&self, user_id: Uuid, message: WSMessage) -> bool {
        let connections = self.connections.read().await;
        if let Some(conn) = connections.get(&user_id) {
            if let Ok(json) = serde_json::to_string(&message) {
                let ws_message = Message::Text(json);
                let _ = conn.sender.send(ws_message);
                return true;
            }
        }
        false
    }

    /// 向对话中的所有用户发送消息
    pub async fn send_to_conversation(&self, conversation_id: Uuid, message: WSMessage) -> usize {
        let conv_connections = self.conversation_connections.read().await;
        let connections = self.connections.read().await;

        if let Some(user_ids) = conv_connections.get(&conversation_id) {
            if let Ok(json) = serde_json::to_string(&message) {
                let ws_message = Message::Text(json);
                let mut sent_count = 0;

                for user_id in user_ids {
                    if let Some(conn) = connections.get(user_id) {
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
        let connections = self.connections.read().await;
        connections.len()
    }

    /// 获取对话中的在线用户数
    pub async fn conversation_online_count(&self, conversation_id: Uuid) -> usize {
        let conv_connections = self.conversation_connections.read().await;
        conv_connections
            .get(&conversation_id)
            .map(|users| users.len())
            .unwrap_or(0)
    }
}

impl Default for WSConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}