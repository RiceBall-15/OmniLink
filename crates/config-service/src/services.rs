use anyhow::Result;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

use super::models::*;
use super::repository::ConfigRepository;

pub struct ConfigService {
    repository: ConfigRepository,
    cache: Arc<RwLock<HashMap<String, String>>>,
}

impl ConfigService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            repository: ConfigRepository::new(pool),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 获取配置（带缓存）
    pub async fn get_config(&self, key: &str) -> Result<Option<String>> {
        // 先从缓存读取
        {
            let cache = self.cache.read().await;
            if let Some(value) = cache.get(key) {
                return Ok(Some(value.clone()));
            }
        }

        // 缓存未命中，从数据库读取
        if let Some(config) = self.repository.get_config(key).await? {
            let value = config.value.clone();

            // 更新缓存
            let mut cache = self.cache.write().await;
            cache.insert(key.to_string(), value.clone());

            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// 设置配置
    pub async fn set_config(
        &self,
        key: &str,
        value: &str,
        updated_by: Option<uuid::Uuid>,
    ) -> Result<ConfigItem> {
        // 更新数据库
        let config = self.repository.upsert_config(key, value, updated_by).await?;

        // 更新缓存
        let mut cache = self.cache.write().await;
        cache.insert(key.to_string(), value.to_string());

        // 通知订阅者（异步）
        self._notify_subscribers(key, &config).await;

        Ok(config)
    }

    /// 删除配置
    pub async fn delete_config(&self, key: &str) -> Result<bool> {
        let deleted = self.repository.delete_config(key).await?;

        if deleted {
            // 从缓存中删除
            let mut cache = self.cache.write().await;
            cache.remove(key);
        }

        Ok(deleted)
    }

    /// 获取所有配置
    pub async fn list_configs(&self) -> Result<Vec<ConfigItem>> {
        self.repository.list_configs().await
    }

    /// 批量获取配置
    pub async fn batch_get_configs(&self, keys: &[String]) -> Result<BatchConfigResponse> {
        let configs = self.repository.batch_get_configs(keys).await?;
        let found_keys: Vec<String> = configs.iter().map(|c| c.key.clone()).collect();

        let not_found: Vec<String> = keys
            .iter()
            .filter(|k| !found_keys.contains(k))
            .cloned()
            .collect();

        let query_results: Vec<ConfigQueryResult> = configs
            .into_iter()
            .map(|c| ConfigQueryResult {
                key: c.key,
                value: c.value,
                version: c.version,
                updated_at: c.updated_at,
            })
            .collect();

        Ok(BatchConfigResponse {
            configs: query_results,
            not_found,
        })
    }

    /// 获取配置历史
    pub async fn get_config_history(&self, key: &str, limit: i64) -> Result<Vec<ConfigHistory>> {
        self.repository.get_config_history(key, limit).await
    }

    /// 恢复配置到指定版本
    pub async fn restore_config_version(
        &self,
        key: &str,
        version: i32,
        updated_by: Option<uuid::Uuid>,
    ) -> Result<Option<ConfigItem>> {
        self.repository.restore_config_version(key, version, updated_by).await
    }

    /// 添加配置订阅
    pub async fn add_subscription(&self, req: CreateSubscriptionRequest) -> Result<ConfigSubscription> {
        let id = uuid::Uuid::new_v4();
        let subscription = ConfigSubscription {
            id,
            key: req.key,
            subscriber: req.subscriber,
            callback_url: req.callback_url,
            created_at: chrono::Utc::now(),
            last_notified_at: None,
        };

        self.repository.add_subscription(&subscription).await
    }

    /// 获取配置订阅
    pub async fn get_subscriptions(&self, key: &str) -> Result<Vec<ConfigSubscription>> {
        self.repository.get_subscriptions(key).await
    }

    /// 删除订阅
    pub async fn remove_subscription(&self, id: uuid::Uuid) -> Result<bool> {
        self.repository.remove_subscription(id).await
    }

    /// 预热缓存
    pub async fn warmup_cache(&self) -> Result<()> {
        let configs = self.repository.list_configs().await?;
        let mut cache = self.cache.write().await;

        for config in configs {
            cache.insert(config.key, config.value);
        }

        Ok(())
    }

    /// 内部方法：通知订阅者
    async fn _notify_subscribers(&self, key: &str, config: &ConfigItem) {
        if let Ok(subs) = self.repository.get_subscriptions(key).await {
            for sub in subs {
                // 异步通知
                if let Some(callback_url) = sub.callback_url {
                    tokio::spawn(async move {
                        let _ = reqwest::post(&callback_url, serde_json::to_vec(config)).await;
                    });
                }
            }
        }
    }
}