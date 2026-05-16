//! 消息表情回复（Reactions）模块
//!
//! 支持对消息添加/移除表情回复，常用于 Slack/微信 风格的交互。

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// 单条消息的所有 reaction 汇总
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageReactions {
    pub message_id: Uuid,
    /// emoji -> (user_id, added_at)
    pub reactions: HashMap<String, Vec<ReactionEntry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionEntry {
    pub user_id: Uuid,
    pub added_at: i64,
}

/// reaction 存储管理器
pub struct ReactionStore {
    /// message_id -> emoji -> Vec<ReactionEntry>
    store: Arc<RwLock<HashMap<Uuid, HashMap<String, Vec<ReactionEntry>>>>>,
}

impl ReactionStore {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 添加 reaction
    pub async fn add_reaction(&self, message_id: Uuid, emoji: String, user_id: Uuid) -> bool {
        let mut store = self.store.write().await;
        let message_reactions = store.entry(message_id).or_default();
        let emoji_reactions = message_reactions.entry(emoji).or_default();

        // 防止重复
        if emoji_reactions.iter().any(|r| r.user_id == user_id) {
            return false;
        }

        emoji_reactions.push(ReactionEntry {
            user_id,
            added_at: Utc::now().timestamp(),
        });
        true
    }

    /// 移除 reaction
    pub async fn remove_reaction(&self, message_id: Uuid, emoji: String, user_id: Uuid) -> bool {
        let mut store = self.store.write().await;
        if let Some(message_reactions) = store.get_mut(&message_id) {
            if let Some(emoji_reactions) = message_reactions.get_mut(&emoji) {
                let before = emoji_reactions.len();
                emoji_reactions.retain(|r| r.user_id != user_id);
                if emoji_reactions.is_empty() {
                    message_reactions.remove(&emoji);
                }
                return emoji_reactions.len() < before;
            }
        }
        false
    }

    /// 获取消息的所有 reactions
    pub async fn get_reactions(&self, message_id: Uuid) -> MessageReactions {
        let store = self.store.read().await;
        let reactions = store
            .get(&message_id)
            .cloned()
            .unwrap_or_default();
        MessageReactions {
            message_id,
            reactions,
        }
    }

    /// 获取消息 reaction 统计摘要（emoji -> count）
    pub async fn get_reaction_summary(&self, message_id: Uuid) -> HashMap<String, usize> {
        let store = self.store.read().await;
        match store.get(&message_id) {
            Some(reactions) => reactions
                .iter()
                .map(|(emoji, entries)| (emoji.clone(), entries.len()))
                .collect(),
            None => HashMap::new(),
        }
    }

    /// 用户是否对某消息添加了指定 emoji
    pub async fn has_user_reacted(
        &self,
        message_id: Uuid,
        emoji: &str,
        user_id: Uuid,
    ) -> bool {
        let store = self.store.read().await;
        store
            .get(&message_id)
            .and_then(|m| m.get(emoji))
            .map(|entries| entries.iter().any(|r| r.user_id == user_id))
            .unwrap_or(false)
    }
}

impl Default for ReactionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_and_get_reaction() {
        let store = ReactionStore::new();
        let msg_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let added = store.add_reaction(msg_id, "👍".to_string(), user_id).await;
        assert!(added);

        let reactions = store.get_reactions(msg_id).await;
        assert_eq!(reactions.reactions.len(), 1);
        assert!(reactions.reactions.contains_key("👍"));
        assert_eq!(reactions.reactions["👍"].len(), 1);
    }

    #[tokio::test]
    async fn test_duplicate_reaction_prevented() {
        let store = ReactionStore::new();
        let msg_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        assert!(store.add_reaction(msg_id, "❤️".to_string(), user_id).await);
        assert!(!store.add_reaction(msg_id, "❤️".to_string(), user_id).await);

        let summary = store.get_reaction_summary(msg_id).await;
        assert_eq!(summary["❤️"], 1);
    }

    #[tokio::test]
    async fn test_remove_reaction() {
        let store = ReactionStore::new();
        let msg_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        store.add_reaction(msg_id, "😂".to_string(), user_id).await;
        let removed = store.remove_reaction(msg_id, "😂".to_string(), user_id).await;
        assert!(removed);

        let reactions = store.get_reactions(msg_id).await;
        assert!(reactions.reactions.is_empty());
    }

    #[tokio::test]
    async fn test_multiple_users_same_emoji() {
        let store = ReactionStore::new();
        let msg_id = Uuid::new_v4();
        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();

        store.add_reaction(msg_id, "🔥".to_string(), user1).await;
        store.add_reaction(msg_id, "🔥".to_string(), user2).await;

        let summary = store.get_reaction_summary(msg_id).await;
        assert_eq!(summary["🔥"], 2);
    }

    #[tokio::test]
    async fn test_has_user_reacted() {
        let store = ReactionStore::new();
        let msg_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        assert!(!store.has_user_reacted(msg_id, "👍", user_id).await);
        store.add_reaction(msg_id, "👍".to_string(), user_id).await;
        assert!(store.has_user_reacted(msg_id, "👍", user_id).await);
    }
}
