//! 模型路由策略
//!
//! 支持多种路由策略：
//! - 直接路由：使用指定模型
//! - 回退路由：主模型失败时自动切换备选模型
//! - 成本优化：选择最低成本的可用模型
//! - 负载均衡：在多个同级模型间轮询

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::models::ModelConfig;

/// 路由策略类型
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RoutingStrategy {
    /// 直接使用指定模型
    Direct,
    /// 主模型失败时回退到备选模型
    Fallback,
    /// 选择成本最低的可用模型
    CostOptimized,
    /// 在同级模型间轮询
    RoundRobin,
}

/// 模型路由配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelRoute {
    /// 主模型ID
    pub primary: String,
    /// 备选模型列表（按优先级排序）
    pub fallbacks: Vec<String>,
    /// 路由策略
    pub strategy: RoutingStrategy,
    /// 是否启用
    pub enabled: bool,
}

/// 模型路由器
pub struct ModelRouter {
    /// 路由配置表：model_id -> route config
    routes: Arc<RwLock<HashMap<String, ModelRoute>>>,
    /// 模型配置列表
    model_configs: Arc<RwLock<Vec<ModelConfig>>>,
    /// Round-robin 计数器
    rr_counters: Arc<RwLock<HashMap<String, usize>>>,
    /// 不可用模型列表（临时标记）
    unavailable_models: Arc<RwLock<HashMap<String, chrono::DateTime<chrono::Utc>>>>,
}

impl ModelRouter {
    pub fn new(model_configs: Arc<RwLock<Vec<ModelConfig>>>) -> Self {
        Self {
            routes: Arc::new(RwLock::new(HashMap::new())),
            model_configs,
            rr_counters: Arc::new(RwLock::new(HashMap::new())),
            unavailable_models: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册模型路由
    pub async fn register_route(&self, model_id: &str, route: ModelRoute) {
        let mut routes = self.routes.write().await;
        routes.insert(model_id.to_string(), route);
    }

    /// 设置默认路由（直接策略）
    pub async fn set_default_routes(&self) {
        let configs = self.model_configs.read().await;

        // 为每个模型创建默认的回退路由
        let openai_models: Vec<&str> = configs.iter()
            .filter(|c| c.provider == "openai")
            .map(|c| c.id.as_str())
            .collect();
        let qwen_models: Vec<&str> = configs.iter()
            .filter(|c| c.provider == "qwen")
            .map(|c| c.id.as_str())
            .collect();
        let zhipu_models: Vec<&str> = configs.iter()
            .filter(|c| c.provider == "zhipu")
            .map(|c| c.id.as_str())
            .collect();

        // OpenAI → Qwen fallback
        for model_id in &openai_models {
            self.register_route(model_id, ModelRoute {
                primary: model_id.to_string(),
                fallbacks: qwen_models.iter().map(|s| s.to_string()).collect(),
                strategy: RoutingStrategy::Fallback,
                enabled: true,
            }).await;
        }

        // Qwen → Zhipu fallback
        for model_id in &qwen_models {
            self.register_route(model_id, ModelRoute {
                primary: model_id.to_string(),
                fallbacks: zhipu_models.iter().map(|s| s.to_string()).collect(),
                strategy: RoutingStrategy::Fallback,
                enabled: true,
            }).await;
        }

        tracing::info!(
            "Registered {} model routes with fallback chains",
            openai_models.len() + qwen_models.len() + zhipu_models.len()
        );
    }

    /// 解析模型路由，返回最终要使用的模型ID
    pub async fn resolve_model(&self, requested_model_id: &str) -> String {
        let routes = self.routes.read().await;

        if let Some(route) = routes.get(requested_model_id) {
            if !route.enabled {
                return requested_model_id.to_string();
            }

            match route.strategy {
                RoutingStrategy::Direct => requested_model_id.to_string(),
                RoutingStrategy::Fallback => {
                    self.resolve_fallback(route).await
                }
                RoutingStrategy::CostOptimized => {
                    self.resolve_cost_optimized(requested_model_id).await
                }
                RoutingStrategy::RoundRobin => {
                    self.resolve_round_robin(route).await
                }
            }
        } else {
            // No route configured, use as-is
            requested_model_id.to_string()
        }
    }

    /// 回退策略解析
    async fn resolve_fallback(&self, route: &ModelRoute) -> String {
        // Check if primary is available
        if self.is_model_available(&route.primary).await {
            return route.primary.clone();
        }

        // Try fallbacks in order
        for fallback in &route.fallbacks {
            if self.is_model_available(fallback).await {
                tracing::info!(
                    "Model '{}' unavailable, falling back to '{}'",
                    route.primary, fallback
                );
                return fallback.clone();
            }
        }

        // All models unavailable, return primary (will fail but with proper error)
        tracing::warn!(
            "All models in route unavailable, returning primary '{}'",
            route.primary
        );
        route.primary.clone()
    }

    /// 成本优化策略解析
    async fn resolve_cost_optimized(&self, requested_model_id: &str) -> String {
        let configs = self.model_configs.read().await;
        let unavailable = self.unavailable_models.read().await;

        // Find all available models sorted by input cost
        let mut candidates: Vec<&ModelConfig> = configs.iter()
            .filter(|c| !unavailable.contains_key(&c.id))
            .collect();
        candidates.sort_by(|a, b| {
            a.input_price_per_1k.partial_cmp(&b.input_price_per_1k).unwrap()
        });

        if let Some(cheapest) = candidates.first() {
            if cheapest.id != requested_model_id {
                tracing::info!(
                    "Cost optimization: switching from '{}' to '{}' (${:.4}/1k vs ${:.4}/1k)",
                    requested_model_id, cheapest.id,
                    configs.iter().find(|c| c.id == requested_model_id)
                        .map(|c| c.input_price_per_1k).unwrap_or(0.0),
                    cheapest.input_price_per_1k
                );
            }
            return cheapest.id.clone();
        }

        requested_model_id.to_string()
    }

    /// 轮询策略解析
    async fn resolve_round_robin(&self, route: &ModelRoute) -> String {
        let mut all_models = vec![route.primary.clone()];
        all_models.extend(route.fallbacks.clone());

        let key = route.primary.clone();
        let mut counters = self.rr_counters.write().await;
        let counter = counters.entry(key).or_insert(0);
        let idx = *counter % all_models.len();
        *counter = counter.wrapping_add(1);

        all_models[idx].clone()
    }

    /// 检查模型是否可用
    async fn is_model_available(&self, model_id: &str) -> bool {
        let unavailable = self.unavailable_models.read().await;
        if let Some(unavailable_since) = unavailable.get(model_id) {
            // Auto-recover after 5 minutes
            let elapsed = chrono::Utc::now() - *unavailable_since;
            if elapsed.num_minutes() >= 5 {
                return true; // Allow retry
            }
            return false;
        }
        true
    }

    /// 标记模型为不可用（调用失败时）
    pub async fn mark_unavailable(&self, model_id: &str) {
        let mut unavailable = self.unavailable_models.write().await;
        unavailable.insert(model_id.to_string(), chrono::Utc::now());
        tracing::warn!("Marked model '{}' as temporarily unavailable", model_id);
    }

    /// 恢复模型可用性（手动恢复或自动恢复后）
    pub async fn mark_available(&self, model_id: &str) {
        let mut unavailable = self.unavailable_models.write().await;
        unavailable.remove(model_id);
        tracing::info!("Marked model '{}' as available", model_id);
    }

    /// 获取所有路由配置
    pub async fn get_routes(&self) -> HashMap<String, ModelRoute> {
        let routes = self.routes.read().await;
        routes.clone()
    }

    /// 获取不可用模型列表
    pub async fn get_unavailable_models(&self) -> Vec<String> {
        let unavailable = self.unavailable_models.read().await;
        unavailable.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(id: &str, provider: &str, cost: f64) -> ModelConfig {
        ModelConfig {
            id: id.to_string(),
            name: id.to_string(),
            provider: provider.to_string(),
            api_base: String::new(),
            max_tokens: 4096,
            input_price_per_1k: cost,
            output_price_per_1k: cost * 2.0,
        }
    }

    #[tokio::test]
    async fn test_direct_routing() {
        let configs = Arc::new(RwLock::new(vec![
            make_config("gpt-4", "openai", 0.03),
        ]));
        let router = ModelRouter::new(configs);

        router.register_route("gpt-4", ModelRoute {
            primary: "gpt-4".to_string(),
            fallbacks: vec![],
            strategy: RoutingStrategy::Direct,
            enabled: true,
        }).await;

        let result = router.resolve_model("gpt-4").await;
        assert_eq!(result, "gpt-4");
    }

    #[tokio::test]
    async fn test_fallback_routing() {
        let configs = Arc::new(RwLock::new(vec![
            make_config("gpt-4", "openai", 0.03),
            make_config("qwen-turbo", "qwen", 0.002),
        ]));
        let router = ModelRouter::new(configs);

        router.register_route("gpt-4", ModelRoute {
            primary: "gpt-4".to_string(),
            fallbacks: vec!["qwen-turbo".to_string()],
            strategy: RoutingStrategy::Fallback,
            enabled: true,
        }).await;

        // Primary available
        let result = router.resolve_model("gpt-4").await;
        assert_eq!(result, "gpt-4");

        // Mark primary unavailable
        router.mark_unavailable("gpt-4").await;
        let result = router.resolve_model("gpt-4").await;
        assert_eq!(result, "qwen-turbo");
    }

    #[tokio::test]
    async fn test_cost_optimized_routing() {
        let configs = Arc::new(RwLock::new(vec![
            make_config("gpt-4", "openai", 0.03),
            make_config("qwen-turbo", "qwen", 0.002),
            make_config("glm-4-flash", "zhipu", 0.0001),
        ]));
        let router = ModelRouter::new(configs);

        router.register_route("gpt-4", ModelRoute {
            primary: "gpt-4".to_string(),
            fallbacks: vec![],
            strategy: RoutingStrategy::CostOptimized,
            enabled: true,
        }).await;

        let result = router.resolve_model("gpt-4").await;
        assert_eq!(result, "glm-4-flash"); // cheapest
    }

    #[tokio::test]
    async fn test_round_robin_routing() {
        let configs = Arc::new(RwLock::new(vec![
            make_config("gpt-4", "openai", 0.03),
            make_config("qwen-turbo", "qwen", 0.002),
            make_config("glm-4-flash", "zhipu", 0.0001),
        ]));
        let router = ModelRouter::new(configs);

        router.register_route("gpt-4", ModelRoute {
            primary: "gpt-4".to_string(),
            fallbacks: vec!["qwen-turbo".to_string(), "glm-4-flash".to_string()],
            strategy: RoutingStrategy::RoundRobin,
            enabled: true,
        }).await;

        let r1 = router.resolve_model("gpt-4").await;
        let r2 = router.resolve_model("gpt-4").await;
        let r3 = router.resolve_model("gpt-4").await;
        let r4 = router.resolve_model("gpt-4").await;

        assert_eq!(r1, "gpt-4");
        assert_eq!(r2, "qwen-turbo");
        assert_eq!(r3, "glm-4-flash");
        assert_eq!(r4, "gpt-4"); // wraps around
    }

    #[tokio::test]
    async fn test_no_route_passthrough() {
        let configs = Arc::new(RwLock::new(vec![]));
        let router = ModelRouter::new(configs);

        let result = router.resolve_model("unknown-model").await;
        assert_eq!(result, "unknown-model");
    }
}
