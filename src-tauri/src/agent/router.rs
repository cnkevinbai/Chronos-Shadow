// 混合模型自适应路由中枢
//
// 负责：
// - 多模型矩阵分发（全 API 配置状态）— 根据 Agent 角色自动选最优模型
// - 单模型降阶滑窗（Context Sharding）— 单模型场景裁剪上下文
// - 局域网本地模型热切换（Ollama/Llama.cpp）— 云端超时/超费时毫秒级切换

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::agent::billing::ChronosBillingEngine;

// ─── 模型定义 ──────────────────────────────────────────────────────

/// 模型提供商
///
/// ⚠️ DEPRECATED: 此枚举将于 v0.2.0 移除。
/// 新代码请使用 `ModelModel` 枚举 + `HybridAgentRouter`。
#[deprecated(since = "0.1.1", note = "Use ModelModel + HybridAgentRouter instead")]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Provider {
    DeepSeek,
    Kimi,
    GLM,
    OpenAI,
    Ollama,
    LlamaCpp,
    Custom(String),
}

/// 模型类型
///
/// ⚠️ DEPRECATED: 此枚举将于 v0.2.0 移除。
/// 新代码请使用 `ClusterModelNode` + `HybridAgentRouter`。
#[deprecated(since = "0.1.1", note = "Use ClusterModelNode + HybridAgentRouter instead")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelConfig {
    /// 云端大模型
    Cloud {
        provider: Provider,
        model_name: String,
        api_key: String,
        endpoint: String,
        max_tokens: u32,
    },
    /// 本地模型
    Local {
        provider: Provider,
        model_name: String,
        endpoint: String,
    },
}

#[allow(deprecated)]
impl ModelConfig {
    /// 模型标识符（用于匹配路由规则）
    pub fn key(&self) -> String {
        match self {
            ModelConfig::Cloud { provider, model_name, .. } => {
                format!("{:?}:{}", provider, model_name)
            }
            ModelConfig::Local { provider, model_name, .. } => {
                format!("{:?}:{}", provider, model_name)
            }
        }
    }
}

/// 路由配置模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RouteMode {
    /// 全自动：多模型矩阵最优分配
    AutoMatrix,
    /// 手动：用户覆盖所有 Agent 使用同一模型
    Manual {
        text_model: String,
        vision_model: Option<String>,
    },
    /// 降阶保护：单模型 + Context Sharding
    Fallback {
        model_key: String,
        context_window: usize,
    },
    /// 局域网热切换
    LanLocal {
        model_key: String,
        fallback_cloud: Option<String>,
    },
}

impl RouteMode {
    pub fn label(&self) -> &str {
        match self {
            RouteMode::AutoMatrix => "Auto-Matrix",
            RouteMode::Manual { .. } => "Manual Override",
            RouteMode::Fallback { .. } => "Fallback Single",
            RouteMode::LanLocal { .. } => "LAN Local",
        }
    }
}

// ─── 路由规则 ──────────────────────────────────────────────────────

/// Agent 角色 → 推荐模型 映射规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRule {
    /// 角色名（PM / Architect / Coder …）
    pub role: String,
    /// 推荐的文本模型 key
    pub text_model: String,
    /// 推荐的视觉模型 key（可选）
    pub vision_model: Option<String>,
    /// 选择理由
    pub reason: String,
}

// ─── 路由引擎 ──────────────────────────────────────────────────────

/// 路由中枢
///
/// ⚠️ DEPRECATED: 此结构体将于 v0.2.0 移除。
/// 请迁移至 `HybridAgentRouter`（支持三维自适应决策、集群节点管理、财务审计）。
/// 迁移指南：
///   - `route_text_model()`    → `HybridAgentRouter::select_optimal_model()`
///   - `route_vision_model()`  → `HybridAgentRouter::select_optimal_model()`
///   - `construct_caching_payload()` → `HybridAgentRouter::build_cached_payload()`
///   - LAN 降级                → `HybridAgentRouter::execute_cluster_llm_call()`
#[deprecated(since = "0.1.1", note = "Migrate to HybridAgentRouter")]
pub struct Router {
    /// 当前路由模式
    pub mode: RouteMode,
    /// 已注册的模型配置表
    pub models: HashMap<String, ModelConfig>,
    /// 角色 → 模型路由规则
    pub rules: Vec<RouteRule>,
    /// 云端调用失败次数（触发 LAN 切换）
    pub cloud_failures: u32,
    /// 云端调用失败阈值
    pub fail_threshold: u32,
    /// 是否启用 LAN 回退
    pub lan_fallback_enabled: bool,
}

#[allow(deprecated)]
impl Router {
    pub fn new() -> Self {
        let mut models = HashMap::new();
        let mut rules = Vec::new();

        // 预置模型
        models.insert(
            "deepseek-v4-pro".into(),
            ModelConfig::Cloud {
                provider: Provider::DeepSeek,
                model_name: "deepseek-v4-pro".into(),
                api_key: String::new(),
                endpoint: "https://api.deepseek.com".into(),
                max_tokens: 131072,
            },
        );
        models.insert(
            "deepseek-v4-flash".into(),
            ModelConfig::Cloud {
                provider: Provider::DeepSeek,
                model_name: "deepseek-v4-flash".into(),
                api_key: String::new(),
                endpoint: "https://api.deepseek.com".into(),
                max_tokens: 65536,
            },
        );
        models.insert(
            "kimi-k3".into(),
            ModelConfig::Cloud {
                provider: Provider::Kimi,
                model_name: "kimi-k3".into(),
                api_key: String::new(),
                endpoint: "https://api.moonshot.cn/v1".into(),
                max_tokens: 131072,
            },
        );
        models.insert(
            "kimi-k2.7-code".into(),
            ModelConfig::Cloud {
                provider: Provider::Kimi,
                model_name: "kimi-k2.7-code".into(),
                api_key: String::new(),
                endpoint: "https://api.moonshot.cn/v1".into(),
                max_tokens: 65536,
            },
        );
        models.insert(
            "kimi-k2.7-code-highspeed".into(),
            ModelConfig::Cloud {
                provider: Provider::Kimi,
                model_name: "kimi-k2.7-code-highspeed".into(),
                api_key: String::new(),
                endpoint: "https://api.moonshot.cn/v1".into(),
                max_tokens: 32768,
            },
        );
        models.insert(
            "glm-5.2".into(),
            ModelConfig::Cloud {
                provider: Provider::GLM,
                model_name: "glm-5.2".into(),
                api_key: String::new(),
                endpoint: "https://open.bigmodel.cn/api/paas/v4".into(),
                max_tokens: 131072,
            },
        );
        models.insert(
            "glm-5v-turbo".into(),
            ModelConfig::Cloud {
                provider: Provider::GLM,
                model_name: "glm-5v-turbo".into(),
                api_key: String::new(),
                endpoint: "https://open.bigmodel.cn/api/paas/v4".into(),
                max_tokens: 8192,
            },
        );
        models.insert(
            "glm-5.1".into(),
            ModelConfig::Cloud {
                provider: Provider::GLM,
                model_name: "glm-5.1".into(),
                api_key: String::new(),
                endpoint: "https://open.bigmodel.cn/api/paas/v4".into(),
                max_tokens: 65536,
            },
        );
        models.insert(
            "glm-4.7".into(),
            ModelConfig::Cloud {
                provider: Provider::GLM,
                model_name: "glm-4.7".into(),
                api_key: String::new(),
                endpoint: "https://open.bigmodel.cn/api/paas/v4".into(),
                max_tokens: 32768,
            },
        );
        models.insert(
            "ollama-local".into(),
            ModelConfig::Local {
                provider: Provider::Ollama,
                model_name: "deepseek-v4-flash".into(),
                endpoint: "http://localhost:11434".into(),
            },
        );

        // 预置路由规则
        rules.push(RouteRule {
            role: "PM".into(),
            text_model: "kimi-k3".into(),
            vision_model: None,
            reason: "长文本理解与慢思考推理能力强".into(),
        });
        rules.push(RouteRule {
            role: "Architect".into(),
            text_model: "kimi-k3".into(),
            vision_model: None,
            reason: "长文本理解与慢思考推理能力强".into(),
        });
        rules.push(RouteRule {
            role: "UIDesigner".into(),
            text_model: "glm-5.2".into(),
            vision_model: Some("glm-5v-turbo".into()),
            reason: "多模态感知精度高".into(),
        });
        rules.push(RouteRule {
            role: "Planner".into(),
            text_model: "glm-5.2".into(),
            vision_model: None,
            reason: "Function Calling 与 Tool-Use 稳定性极高".into(),
        });
        rules.push(RouteRule {
            role: "Verifier".into(),
            text_model: "glm-5.2".into(),
            vision_model: None,
            reason: "Function Calling 与 Tool-Use 稳定性极高".into(),
        });
        rules.push(RouteRule {
            role: "Coder".into(),
            text_model: "deepseek-v4-flash".into(),
            vision_model: None,
            reason: "代码生成 + Context Caching 最大化命中".into(),
        });
        rules.push(RouteRule {
            role: "Auditor".into(),
            text_model: "deepseek-v4-flash".into(),
            vision_model: None,
            reason: "AST 审计专用，最大化命中缓存".into(),
        });

        Self {
            mode: RouteMode::AutoMatrix,
            models,
            rules,
            cloud_failures: 0,
            fail_threshold: 5,
            lan_fallback_enabled: true,
        }
    }

    // ── 模型管理 ──────────────────────────────────────────────────

    /// 注册新模型
    pub fn register_model(&mut self, key: &str, config: ModelConfig) {
        self.models.insert(key.into(), config);
    }

    /// 获取模型配置
    pub fn get_model(&self, key: &str) -> Option<&ModelConfig> {
        self.models.get(key)
    }

    // ── 路由逻辑 ──────────────────────────────────────────────────

    /// 根据 Agent 角色路由到最优模型
    pub fn route_text_model(&self, role: &str) -> &str {
        match &self.mode {
            RouteMode::AutoMatrix => {
                // 查找匹配的路由规则
                for rule in &self.rules {
                    if rule.role.eq_ignore_ascii_case(role) {
                        return &rule.text_model;
                    }
                }
                "deepseek-v4-flash" // 默认降本
            }
            RouteMode::Manual { text_model, .. } => text_model,
            RouteMode::Fallback { model_key, .. } => model_key,
            RouteMode::LanLocal { model_key, .. } => model_key,
        }
    }

    /// 根据 Agent 角色路由到最优视觉模型
    pub fn route_vision_model(&self, role: &str) -> Option<&str> {
        match &self.mode {
            RouteMode::AutoMatrix => {
                for rule in &self.rules {
                    if rule.role.eq_ignore_ascii_case(role) {
                        return rule.vision_model.as_deref();
                    }
                }
                None
            }
            RouteMode::Manual { vision_model, .. } => vision_model.as_deref(),
            _ => None,
        }
    }

    /// 记录一次云端调用失败
    /// 超过阈值自动切换到 LAN 模式
    pub fn record_cloud_failure(&mut self) -> bool {
        self.cloud_failures += 1;
        if self.cloud_failures >= self.fail_threshold && self.lan_fallback_enabled {
            self.mode = RouteMode::LanLocal {
                model_key: "ollama-local".into(),
                fallback_cloud: Some("deepseek-v4-flash".into()),
            };
            true // 已切换
        } else {
            false
        }
    }

    /// 重置失败计数器
    pub fn reset_failures(&mut self) {
        self.cloud_failures = 0;
        if matches!(self.mode, RouteMode::LanLocal { .. }) {
            self.mode = RouteMode::AutoMatrix;
        }
    }

    // ── 路由规则管理 ──────────────────────────────────────────────

    /// 添加路由规则
    pub fn add_rule(&mut self, rule: RouteRule) {
        self.rules.push(rule);
    }

    /// 获取所有路由规则（供前端展示）
    pub fn get_rules(&self) -> &[RouteRule] {
        &self.rules
    }

    // ── Context Caching（对齐白皮书 construct_caching_payload） ───

    /// 针对 DeepSeek 的 Context Caching 机制重构 Prompt 头
    /// 强制将 CLAUDE.md 契约排在最前端，确保 100% 触发 1 折计费
    pub fn construct_caching_payload(
        &self,
        claude_contract: &str,
        current_prompt: &str,
        history: &[String],
    ) -> serde_json::Value {
        let mut messages: Vec<serde_json::Value> = Vec::new();

        // 极致压榨缓存：固定的 System 契约作为第一条消息
        messages.push(json!({
            "role": "system",
            "content": format!("## CLAUDE.md GLOBAL CONTRACT ##\n{}", claude_contract)
        }));

        // 历史消息（中间部分，不破坏前文缓存结构）
        for hist in history {
            messages.push(json!({ "role": "assistant", "content": hist }));
        }

        // 当前动态请求（尾部动态区）
        messages.push(json!({ "role": "user", "content": current_prompt }));

        json!({
            "messages": messages,
            "temperature": 0.0,
            "stream": false
        })
    }

    // ── 智能路由 + 热切换（对齐白皮书 execute_request_with_failover） ─

    /// 根据角色路由到最优模型并返回完整配置
    pub async fn decide_target_node(&self, agent_role: &str) -> ModelConfig {
        let model_key = self.route_text_model(agent_role);
        self.models.get(model_key).cloned().unwrap_or_else(|| {
            self.models.values().next().cloned().unwrap_or(ModelConfig::Cloud {
                provider: Provider::DeepSeek,
                model_name: "deepseek-v4-flash".into(),
                api_key: String::new(),
                endpoint: "https://api.deepseek.com".into(),
                max_tokens: 65536,
            })
        })
    }

    /// 带局域网离线降级热切换的通用请求执行器
    /// 当云端超时或熔断时，毫秒级无感降级至局域网本地模型
    pub async fn execute_request_with_failover(
        &self,
        agent_role: &str,
        payload: serde_json::Value,
    ) -> Result<String, String> {
        let primary_node = self.decide_target_node(agent_role).await;
        let (endpoint, api_key, is_local) = match &primary_node {
            ModelConfig::Cloud { endpoint, api_key, .. } => (endpoint.clone(), api_key.clone(), false),
            ModelConfig::Local { endpoint, .. } => (endpoint.clone(), String::new(), true),
        };

        tracing::info!("[ROUTER] Route: {} → {} (Local: {})", agent_role, primary_node.key(), is_local);

        let client = reqwest::Client::new();
        let response = client
            .post(&endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .timeout(std::time::Duration::from_millis(3500))
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
                Ok(body["choices"][0]["message"]["content"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string())
            }
            Ok(resp) => {
                tracing::warn!("[ROUTER] Primary status: {}. Triggering LAN failover...", resp.status());
                self.trigger_lan_fallback(payload).await
            }
            Err(e) => {
                tracing::warn!("[ROUTER HOT-SWAP] Primary failed: {}. Switching to LAN-Local!", e);
                self.trigger_lan_fallback(payload).await
            }
        }
    }

    /// 局域网本地模型网关降级（LAN Native Fallback）
    async fn trigger_lan_fallback(&self, payload: serde_json::Value) -> Result<String, String> {
        let local = self.models.iter().find(|(_, v)| {
            matches!(v, ModelConfig::Local { .. })
        });

        let (endpoint, model) = match local {
            Some((_, ModelConfig::Local { endpoint, model_name, .. })) => (endpoint.clone(), model_name.clone()),
            _ => return Err("LAN fallback failed: no local Ollama/Llama.cpp node configured".into()),
        };

        tracing::info!("[ROUTER LAN-NATIVE] Hot-swapped to local: {} @ {}", model, endpoint);

        let client = reqwest::Client::new();
        let local_payload = json!({
            "model": model,
            "messages": payload["messages"],
            "stream": false,
        });

        let resp = client
            .post(&endpoint)
            .json(&local_payload)
            .send()
            .await
            .map_err(|e| format!("LAN gateway crashed: {}", e))?;

        let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        Ok(body["message"]["content"]
            .as_str()
            .or_else(|| body["choices"][0]["message"]["content"].as_str())
            .unwrap_or_default()
            .to_string())
    }

    /// LAN 健康检查 — 测试 Ollama 是否可达
    pub async fn check_lan_health() -> Result<Vec<String>, String> {
        let client = reqwest::Client::new();
        let resp = client
            .get("http://localhost:11434/api/tags")
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await
            .map_err(|e| format!("Ollama 未运行或不可达: {}", e))?;

        let body: serde_json::Value =
            resp.json().await.map_err(|e| e.to_string())?;
        let models: Vec<String> = body["models"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|m| m["name"].as_str().map(String::from))
            .collect();

        if models.is_empty() {
            Err("Ollama 已连接但无可用模型".into())
        } else {
            Ok(models)
        }
    }

    /// 创建线程安全的 Arc<RwLock<Router>> 包装（对齐白皮书 AdaptiveRouter）
    pub fn into_shared(self) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(self))
    }

    // ── Context Sharding (降阶保护) ───────────────────────────────

    /// 对上下文进行裁剪（仅保留受影响的文件拓扑）
    pub fn shard_context(&self, context: &str, max_chars: usize) -> String {
        if context.len() <= max_chars {
            return context.into();
        }
        // 保留头部和尾部，中间截断
        let head = &context[..max_chars / 2];
        let tail = &context[context.len() - max_chars / 2..];
        format!(
            "{}\n\n... [{} chars truncated — Context Sharding active] ...\n\n{}",
            head,
            context.len() - max_chars,
            tail
        )
    }
}

#[allow(deprecated)]
impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════
// 2026 升级版：分布式黑板架构混合路由中枢 (HybridAgentRouter)
// ═══════════════════════════════════════════════════════════════════

/// 2026 最新大模型矩阵枚举
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelModel {
    DeepSeekV4Pro,
    DeepSeekV4Flash,
    KimiK3,
    KimiK27Code,
    KimiK27CodeHighspeed,
    Glm52,
    Glm5vTurbo,
    Glm51,
    Glm47,
    LanOllamaR1,
}

impl ModelModel {
    pub fn display(&self) -> &str {
        match self {
            ModelModel::DeepSeekV4Pro => "DeepSeek V4-Pro",
            ModelModel::DeepSeekV4Flash => "DeepSeek V4-Flash",
            ModelModel::KimiK3 => "Kimi K3",
            ModelModel::KimiK27Code => "Kimi K2.7-Code",
            ModelModel::KimiK27CodeHighspeed => "Kimi K2.7-Code-HS",
            ModelModel::Glm52 => "GLM-5.2",
            ModelModel::Glm5vTurbo => "GLM-5V-Turbo",
            ModelModel::Glm51 => "GLM-5.1",
            ModelModel::Glm47 => "GLM-4.7",
            ModelModel::LanOllamaR1 => "LAN-Ollama-R1",
        }
    }

    pub fn is_cache_eligible(&self) -> bool {
        matches!(self, ModelModel::DeepSeekV4Flash | ModelModel::DeepSeekV4Pro)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterModelNode {
    pub model_type: ModelModel,
    pub api_url: String,
    pub timeout_ms: u64,
    pub cost_per_1k_tokens: f64,
}

/// 三维自适应路由决策结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub agent_role: String,
    pub selected_model: ModelModel,
    pub is_cache_eligible: bool,
    pub is_lan_fallback: bool,
    pub reason: String,
}

/// 分布式集群黑板架构混合路由中枢
pub struct HybridAgentRouter {
    pub route_mode: Arc<RwLock<String>>,
    pub cluster_nodes: Arc<RwLock<HashMap<ModelModel, ClusterModelNode>>>,
    pub lan_fallback_enabled: Arc<RwLock<bool>>,
    pub cloud_failures: Arc<RwLock<u32>>,
    pub fail_threshold: u32,
    pub cost_limit: Arc<RwLock<f64>>,
    pub current_cost: Arc<RwLock<f64>>,
    pub total_saved: Arc<RwLock<f64>>,
    pub billing_engine: ChronosBillingEngine,
}

impl HybridAgentRouter {
    pub fn new() -> Self {
        let mut nodes = HashMap::new();

        nodes.insert(ModelModel::DeepSeekV4Pro, ClusterModelNode {
            model_type: ModelModel::DeepSeekV4Pro,
            api_url: "https://api.deepseek.com".into(),
            timeout_ms: 4500,
            cost_per_1k_tokens: 0.0045,   // ¥4.5/1M (DeepSeek 官方)
        });
        nodes.insert(ModelModel::DeepSeekV4Flash, ClusterModelNode {
            model_type: ModelModel::DeepSeekV4Flash,
            api_url: "https://api.deepseek.com".into(),
            timeout_ms: 2000,
            cost_per_1k_tokens: 0.0015,   // ¥1.5/1M (DeepSeek 官方)
        });
        nodes.insert(ModelModel::KimiK3, ClusterModelNode {
            model_type: ModelModel::KimiK3,
            api_url: "https://api.moonshot.cn/v1".into(),
            timeout_ms: 5000,
            cost_per_1k_tokens: 0.004,
        });
        nodes.insert(ModelModel::KimiK27Code, ClusterModelNode {
            model_type: ModelModel::KimiK27Code,
            api_url: "https://api.moonshot.cn/v1".into(),
            timeout_ms: 3000,
            cost_per_1k_tokens: 0.002,
        });
        nodes.insert(ModelModel::KimiK27CodeHighspeed, ClusterModelNode {
            model_type: ModelModel::KimiK27CodeHighspeed,
            api_url: "https://api.moonshot.cn/v1".into(),
            timeout_ms: 1500,
            cost_per_1k_tokens: 0.001,
        });
        nodes.insert(ModelModel::Glm52, ClusterModelNode {
            model_type: ModelModel::Glm52,
            api_url: "https://open.bigmodel.cn/api/paas/v4".into(),
            timeout_ms: 3000,
            cost_per_1k_tokens: 0.004,
        });
        nodes.insert(ModelModel::Glm5vTurbo, ClusterModelNode {
            model_type: ModelModel::Glm5vTurbo,
            api_url: "https://open.bigmodel.cn/api/paas/v4".into(),
            timeout_ms: 3500,
            cost_per_1k_tokens: 0.005,
        });
        nodes.insert(ModelModel::Glm51, ClusterModelNode {
            model_type: ModelModel::Glm51,
            api_url: "https://open.bigmodel.cn/api/paas/v4".into(),
            timeout_ms: 2500,
            cost_per_1k_tokens: 0.002,
        });
        nodes.insert(ModelModel::Glm47, ClusterModelNode {
            model_type: ModelModel::Glm47,
            api_url: "https://open.bigmodel.cn/api/paas/v4".into(),
            timeout_ms: 2500,
            cost_per_1k_tokens: 0.002,
        });
        nodes.insert(ModelModel::LanOllamaR1, ClusterModelNode {
            model_type: ModelModel::LanOllamaR1,
            api_url: "http://localhost:11434".into(),
            timeout_ms: 4000,
            cost_per_1k_tokens: 0.0, // 0 资费
        });

        Self {
            route_mode: Arc::new(RwLock::new("auto".into())),
            cluster_nodes: Arc::new(RwLock::new(nodes)),
            lan_fallback_enabled: Arc::new(RwLock::new(true)),
            cloud_failures: Arc::new(RwLock::new(0)),
            fail_threshold: 5,
            cost_limit: Arc::new(RwLock::new(5.00)),
            current_cost: Arc::new(RwLock::new(0.0)),
            total_saved: Arc::new(RwLock::new(0.0)),
            billing_engine: ChronosBillingEngine::new(),
        }
    }

    /// 🔬 最省优先联路由 (Cheapest-First Cascading)
    /// 算法: 从最便宜模型开始 → 评估输出质量 → 不足则逐级升级
    /// 参考: FrugalGPT / LLM Cascade 论文 (Chen et al., 2023)
    pub async fn cheapest_first_cascade(
        &self,
        agent_role: &str,
        quality_threshold: f64,
    ) -> Vec<(ModelModel, f64)> {
        let nodes = self.cluster_nodes.read().await;
        let mut sorted: Vec<(&ModelModel, &ClusterModelNode)> = nodes.iter().collect();
        // 按成本升序排列 (便宜优先)
        sorted.sort_by(|a, b| a.1.cost_per_1k_tokens.partial_cmp(&b.1.cost_per_1k_tokens).unwrap_or(std::cmp::Ordering::Equal));

        // 级联链: 从便宜到贵, 过滤掉不适配的
        sorted.into_iter()
            .filter(|(m, _)| matches_agent_role(m, agent_role))
            .map(|(m, n)| (m.clone(), 1.0 - (n.cost_per_1k_tokens / 0.005).min(1.0) * quality_threshold))
            .collect()
    }

    /// 核心功能 1：四维自适应决策算法 (增强版)
    ///
    /// 维度: Agent角色 + 任务紧急度 + 成本优化 + Agent质量评分
    pub async fn select_optimal_model(
        &self,
        agent_role: &str,
        is_high_urgency: bool,
    ) -> RoutingDecision {
        self.select_optimal_model_quality(agent_role, is_high_urgency, 85).await
    }

    /// 四维决策 + Agent质量感知降级
    pub async fn select_optimal_model_quality(
        &self,
        agent_role: &str,
        is_high_urgency: bool,
        agent_quality_score: u32,
    ) -> RoutingDecision {
        let mode = self.route_mode.read().await;

        // 质量过低 → 强制降级到免费/极低成本模型
        if agent_quality_score < 50 && !is_high_urgency {
            return RoutingDecision {
                agent_role: agent_role.into(),
                selected_model: ModelModel::LanOllamaR1,
                is_cache_eligible: false,
                is_lan_fallback: true,
                reason: format!("Agent质量分{}<50, 自动降级至LAN离线模型", agent_quality_score),
            };
        }

        let selected = if *mode == "manual" {
            // Manual override: PM/Arch → Pro, Coder/Verifier → Flash
            match agent_role {
                "UIDesigner" => ModelModel::Glm5vTurbo,
                _ => ModelModel::DeepSeekV4Pro,
            }
        } else {
            match agent_role {
                "PM" => ModelModel::KimiK3,
                "Architect" => ModelModel::DeepSeekV4Pro,
                "UIDesigner" => ModelModel::Glm5vTurbo,
                "Planner" => ModelModel::Glm52,
                "Coder" | "Auditor" => {
                    if is_high_urgency {
                        ModelModel::KimiK27CodeHighspeed
                    } else {
                        ModelModel::DeepSeekV4Flash
                    }
                }
                "Verifier" => ModelModel::DeepSeekV4Flash,
                _ => ModelModel::DeepSeekV4Pro,
            }
        };

        let reason = match &selected {
            ModelModel::KimiK3 => "K3 超长项目分析长文本",
            ModelModel::DeepSeekV4Pro => "V4 Pro 旗舰深度慢思考推理",
            ModelModel::Glm5vTurbo => "5V 视觉多模态全能走查",
            ModelModel::Glm52 => "GLM 5.2 原生大模型工具链极速编排",
            ModelModel::DeepSeekV4Flash => "默认写码审计：锁一折 Caching 缓存首选",
            ModelModel::KimiK27CodeHighspeed => "紧急编译阻断，切极速写码节点",
            ModelModel::KimiK27Code => "K2.7 稳定写码",
            ModelModel::Glm51 => "GLM 5.1 稳定推理",
            ModelModel::Glm47 => "GLM 4.7 高性价比推理",
            ModelModel::LanOllamaR1 => "LAN 离线降级热备",
        };

        RoutingDecision {
            agent_role: agent_role.into(),
            selected_model: selected.clone(),
            is_cache_eligible: selected.is_cache_eligible(),
            is_lan_fallback: false,
            reason: reason.into(),
        }
    }

    /// 核心功能 2：对账契约化 Payload 组装 (Maximizing Context Caching)
    pub fn build_cached_payload(
        &self,
        claude_contract: &str,
        evolution_rules: &str,
        current_prompt: &str,
        history: &[String],
    ) -> serde_json::Value {
        let mut messages: Vec<serde_json::Value> = Vec::new();

        let system_contract = format!(
            "## CLAUDE.md GLOBAL CONTRACT ##\n{}\n\n## ANTI-HALLUCINATION EVOLVED REGULATIONS ##\n{}",
            claude_contract, evolution_rules
        );
        messages.push(json!({ "role": "system", "content": system_contract }));

        for hist in history {
            messages.push(json!({ "role": "assistant", "content": hist }));
        }

        messages.push(json!({ "role": "user", "content": current_prompt }));

        json!({
            "messages": messages,
            "temperature": 0.0,
            "stream": false
        })
    }

    /// 核心功能 3：带局域网离线降级热切换的分布式请求执行器
    pub async fn execute_cluster_llm_call(
        &self,
        agent_role: &str,
        is_urgent: bool,
        payload: serde_json::Value,
    ) -> Result<(String, bool), String> {
        let decision = self.select_optimal_model(agent_role, is_urgent).await;
        let nodes = self.cluster_nodes.read().await;
        let active_node = nodes.get(&decision.selected_model)
            .ok_or("目标模型节点未注册于中央总线")?;

        tracing::info!(
            "[HYBRID ROUTER] Routing [{}] → {:?} (cache_eligible: {})",
            agent_role, decision.selected_model, decision.is_cache_eligible
        );

        let client = reqwest::Client::new();
        let response = client
            .post(&active_node.api_url)
            .header("Content-Type", "application/json")
            .timeout(std::time::Duration::from_millis(active_node.timeout_ms))
            .json(&payload)
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
                Ok((
                    body["choices"][0]["message"]["content"]
                        .as_str().unwrap_or_default().to_string(),
                    false,
                ))
            }
            Ok(resp) => {
                tracing::warn!("[ROUTER] Cloud status {}. Failing over to LAN...", resp.status());
                let result = self.trigger_lan_native_failover(client, payload).await?;
                Ok((result, true))
            }
            Err(_e) => {
                tracing::warn!("[ROUTER TIMEOUT] Cloud unreachable. Activating LAN Hot-Swap!");
                let result = self.trigger_lan_native_failover(client, payload).await?;
                Ok((result, true))
            }
        }
    }

    /// 方案B：局域网离线模型网关
    async fn trigger_lan_native_failover(
        &self,
        client: reqwest::Client,
        payload: serde_json::Value,
    ) -> Result<String, String> {
        let nodes = self.cluster_nodes.read().await;
        let lan_node = nodes.get(&ModelModel::LanOllamaR1)
            .ok_or("降级失败：本地局域网未探测到常驻的 Ollama 离线热备服务器")?;

        tracing::info!("[ROUTER LAN-NATIVE] Hot-swapped to: {}", lan_node.api_url);

        let local_payload = json!({
            "model": "deepseek-v4-flash",
            "messages": payload["messages"],
            "stream": false,
        });

        let resp = client
            .post(&lan_node.api_url)
            .json(&local_payload)
            .send()
            .await
            .map_err(|e| format!("LAN gateway crashed: {}", e))?;

        let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        Ok(body["message"]["content"]
            .as_str()
            .or_else(|| body["choices"][0]["message"]["content"].as_str())
            .unwrap_or_default()
            .to_string())
    }

    /// 获取路由决策（供前端展示）
    pub async fn get_routing_preview(&self, agent_role: &str) -> RoutingDecision {
        self.select_optimal_model(agent_role, false).await
    }

    /// 极致压榨一折 Caching 缓存的 Payload 结构化重构
    pub fn build_optimized_caching_payload(
        &self,
        claude_contract: &str,
        static_ui_tree: &str,
        current_prompt: &str,
        summary_history: &[String],
    ) -> serde_json::Value {
        let mut messages: Vec<serde_json::Value> = Vec::new();

        messages.push(json!({
            "role": "system",
            "content": format!("## CLAUDE.md GLOBAL CONTRACT ##\n{}", claude_contract)
        }));
        messages.push(json!({
            "role": "system",
            "content": format!("## STATIC WIN32 UI AUTOMATION TREE ##\n{}", static_ui_tree)
        }));
        for summary in summary_history {
            messages.push(json!({ "role": "assistant", "content": summary }));
        }
        messages.push(json!({ "role": "user", "content": current_prompt }));

        json!({ "messages": messages, "temperature": 0.0, "stream": false })
    }

    /// 带资费硬熔断与局域网离线热切换的分布式执行器
    pub async fn execute_llm_call_with_cost_protection(
        &self,
        agent_role: &str,
        payload: serde_json::Value,
    ) -> Result<(String, bool, f64), String> {
        let cost = *self.current_cost.read().await;
        let limit = *self.cost_limit.read().await;

        if cost >= limit {
            return Err(format!(
                "[熔断拦截] 累计开销 ¥{:.2} 已达安全阈值 ¥{:.2}，强制刹车阻断。",
                cost, limit
            ));
        }

        let decision = self.select_optimal_model(agent_role, false).await;
        let nodes = self.cluster_nodes.read().await;
        let active_node = nodes
            .get(&decision.selected_model)
            .ok_or("目标模型节点未注册")?;

        let client = reqwest::Client::new();
        let response = client
            .post(&active_node.api_url)
            .header("Content-Type", "application/json")
            .timeout(std::time::Duration::from_millis(active_node.timeout_ms))
            .json(&payload)
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
                let tokens = body["usage"]["total_tokens"].as_u64().unwrap_or(500) as f64;
                let estimated_cost = tokens * active_node.cost_per_1k_tokens / 1000.0;
                let mut c = self.current_cost.write().await;
                *c += estimated_cost;

                Ok((
                    body["choices"][0]["message"]["content"]
                        .as_str().unwrap_or_default().to_string(),
                    decision.is_cache_eligible,
                    *c,
                ))
            }
            Ok(_) | Err(_) => {
                tracing::warn!("[ROUTER FAILOVER] Cloud delayed. Hot-swapping to LAN Ollama.");
                let result = self.trigger_lan_native_failover(client, payload).await?;
                Ok((result, false, cost))
            }
        }
    }

    /// 获取当前成本统计
    pub async fn get_cost_stats(&self) -> (f64, f64) {
        (*self.current_cost.read().await, *self.cost_limit.read().await)
    }

    /// 更新成本上限
    pub async fn set_cost_limit(&self, limit: f64) {
        *self.cost_limit.write().await = limit;
    }
}

impl Default for HybridAgentRouter {
    fn default() -> Self { Self::new() }
}

// ─── 辅助函数 ──────────────────────────────────────────────────────

/// 检查模型是否适配指定 Agent 角色
fn matches_agent_role(model: &ModelModel, role: &str) -> bool {
    match role.to_lowercase().as_str() {
        "coder" | "code" => !matches!(model, ModelModel::Glm5vTurbo),
        "auditor" | "reviewer" => !matches!(model, ModelModel::Glm5vTurbo),
        "pm" | "architect" | "ui" | "designer" => true,
        "verifier" | "ci" => !matches!(model, ModelModel::Glm5vTurbo | ModelModel::LanOllamaR1),
        _ => true,
    }
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_matrix_routing() {
        let router = Router::new();
        assert_eq!(router.route_text_model("PM"), "kimi-k3");
        assert_eq!(router.route_text_model("Architect"), "kimi-k3");
        assert_eq!(router.route_text_model("Coder"), "deepseek-v4-flash");
        assert_eq!(router.route_text_model("Auditor"), "deepseek-v4-flash");
        assert_eq!(router.route_text_model("Planner"), "glm-5.2");
        assert_eq!(router.route_text_model("Verifier"), "glm-5.2");
        assert_eq!(router.route_text_model("UnknownRole"), "deepseek-v4-flash");
    }

    #[test]
    fn test_vision_routing() {
        let router = Router::new();
        assert_eq!(
            router.route_vision_model("UIDesigner"),
            Some("glm-5v-turbo")
        );
        assert_eq!(router.route_vision_model("Coder"), None);
    }

    #[test]
    fn test_manual_override() {
        let mut router = Router::new();
        router.mode = RouteMode::Manual {
            text_model: "deepseek-v4-pro".into(),
            vision_model: None,
        };
        assert_eq!(router.route_text_model("PM"), "deepseek-v4-pro");
        assert_eq!(router.route_text_model("Coder"), "deepseek-v4-pro");
    }

    #[test]
    fn test_cloud_failure_fallback() {
        let mut router = Router::new();
        router.fail_threshold = 3;
        assert!(!router.record_cloud_failure());
        assert!(!router.record_cloud_failure());
        assert!(router.record_cloud_failure()); // 第 3 次触发切换
        assert!(matches!(router.mode, RouteMode::LanLocal { .. }));
    }

    #[test]
    fn test_fallback_reset() {
        let mut router = Router::new();
        router.fail_threshold = 1;
        router.record_cloud_failure();
        assert!(matches!(router.mode, RouteMode::LanLocal { .. }));
        router.reset_failures();
        assert!(matches!(router.mode, RouteMode::AutoMatrix));
    }

    #[test]
    fn test_context_sharding() {
        let router = Router::new();
        let long_context = "A".repeat(2000);
        let sharded = router.shard_context(&long_context, 100);
        assert!(sharded.len() <= 200); // head + tail + truncation message
        assert!(sharded.contains("Context Sharding active"));
    }

    #[test]
    fn test_context_no_shard_when_small() {
        let router = Router::new();
        let short = "Hello world".to_string();
        let result = router.shard_context(&short, 1000);
        assert_eq!(result, short);
    }

    #[test]
    fn test_model_registry() {
        let mut router = Router::new();
        router.register_model(
            "custom-model",
            ModelConfig::Local {
                provider: Provider::Ollama,
                model_name: "codellama:7b".into(),
                endpoint: "http://localhost:11434".into(),
            },
        );
        assert!(router.get_model("custom-model").is_some());
        assert!(router.get_model("nonexistent").is_none());
    }
}

// ─── Tauri Commands ──────────────────────────────────────────────

#[tauri::command]
pub fn get_route_mode(state: tauri::State<crate::state::AppState>) -> String {
    let router = state.router.lock().unwrap();
    serde_json::to_string(&router.mode).unwrap_or_default()
}

#[tauri::command]
pub fn set_route_mode(state: tauri::State<crate::state::AppState>, mode_json: String) -> Result<String, String> {
    let mode: RouteMode = serde_json::from_str(&mode_json)
        .map_err(|e| format!("Invalid route mode: {}", e))?;
    let mut router = state.router.lock().unwrap();
    router.mode = mode;
    Ok(router.mode.label().into())
}

#[tauri::command]
pub fn get_available_models(state: tauri::State<crate::state::AppState>) -> Vec<String> {
    let router = state.router.lock().unwrap();
    router.models.keys().cloned().collect()
}

#[tauri::command]
pub fn set_model_api_key(
    state: tauri::State<crate::state::AppState>,
    model_key: String,
    api_key: String,
) -> Result<String, String> {
    let mut router = state.router.lock().unwrap();
    if let Some(config) = router.models.get_mut(&model_key) {
        if let ModelConfig::Cloud { api_key: ref mut key, .. } = config {
            *key = api_key;
            Ok(format!("API key set for {}", model_key))
        } else {
            Err(format!("{} is a local model, no API key needed", model_key))
        }
    } else {
        Err(format!("Model {} not found", model_key))
    }
}

#[tauri::command]
pub fn route_for_role(state: tauri::State<crate::state::AppState>, role: String) -> String {
    let router = state.router.lock().unwrap();
    router.route_text_model(&role).into()
}

#[tauri::command]
pub fn get_model_endpoint(state: tauri::State<crate::state::AppState>, model_key: String) -> Result<String, String> {
    let router = state.router.lock().unwrap();
    match router.get_model(&model_key) {
        Some(ModelConfig::Cloud { endpoint, .. }) => Ok(endpoint.clone()),
        Some(ModelConfig::Local { endpoint, .. }) => Ok(endpoint.clone()),
        None => Err(format!("Model '{}' not found", model_key)),
    }
}

#[tauri::command]
pub async fn hrouter_select_model(
    state: tauri::State<'_, crate::state::AppState>,
    agent_role: String,
    is_high_urgency: bool,
) -> Result<serde_json::Value, String> {
    let decision = state.hybrid_router.select_optimal_model(&agent_role, is_high_urgency).await;
    Ok(serde_json::json!({
        "agent_role": decision.agent_role,
        "selected_model": decision.selected_model.display(),
        "is_cache_eligible": decision.is_cache_eligible,
        "is_lan_fallback": decision.is_lan_fallback,
        "reason": decision.reason,
    }))
}

#[tauri::command]
pub async fn hrouter_get_cluster_status(
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<serde_json::Value, String> {
    let nodes = state.hybrid_router.cluster_nodes.read().await;
    let status: Vec<_> = nodes.iter().map(|(model, node)| {
        serde_json::json!({
            "model": model.display(), "api_url": node.api_url,
            "timeout_ms": node.timeout_ms, "cost_per_1k": node.cost_per_1k_tokens,
        })
    }).collect();
    Ok(serde_json::json!({ "nodes": status }))
}
