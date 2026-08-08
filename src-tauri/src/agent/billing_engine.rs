// 三层并行计费引擎 (Chronos Parallel Billing Engine)
//
// 设计原则：
//   Tier 1 (Official) — 厂商官方定价，精确对账，billing.rs 为唯一权威来源
//   Tier 2 (Budget)   — 官方价 × 1.2 安全系数，用户可见，驱动熔断决策
//   Tier 3 (Router)   — 节点级简化价，仅用于路由降级/切换决策
//
// 三轨并行：一次 record() 同步更新三轨，get_dashboard() 一次拿齐

use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use crate::agent::router::ModelModel;
use crate::agent::billing::{ChronosBillingEngine, ApiUsage};

// ─── 费率层级枚举 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillingTier {
    /// Tier 1: 厂商官方定价（billing.rs，不可配置）
    Official,
    /// Tier 2: 保守预算（Official × 1.2），用于用户可见累计 + 熔断
    Budget,
    /// Tier 3: 路由节点简化价，仅用于路由降级决策
    Router,
}

impl BillingTier {
    pub fn label(&self) -> &str {
        match self {
            BillingTier::Official => "official",
            BillingTier::Budget => "budget",
            BillingTier::Router => "router",
        }
    }
}

// ─── 单轨成本快照 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSnapshot {
    pub tier: String,
    pub total_cost_rmb: f64,
    pub tokens_used: u64,
    pub call_count: u64,
}

// ─── 统一仪表盘（前端一次查询） ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingDashboard {
    pub official: CostSnapshot,
    pub budget: CostSnapshot,
    pub router: CostSnapshot,
    pub cost_cap: f64,
    pub cost_cap_active: bool,
}

// ─── 模型特性矩阵 ──────────────────────────────────────────────────

/// Per-model optimization characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    pub model_key: String,
    pub display: String,
    pub context_window: u32,
    pub supports_cache: bool,
    pub cost_tier: &'static str, // "budget" | "standard" | "premium"
    pub best_for: &'static str,
}

impl ModelModel {
    pub fn profile(&self) -> ModelProfile {
        match self {
            ModelModel::DeepSeekV4Pro => ModelProfile {
                model_key: "deepseek-v4-pro".into(), display: "DeepSeek V4-Pro".into(),
                context_window: 128000, supports_cache: true, cost_tier: "standard",
                best_for: "深度推理 · 架构设计 · 代码审查",
            },
            ModelModel::DeepSeekV4Flash => ModelProfile {
                model_key: "deepseek-v4-flash".into(), display: "DeepSeek V4-Flash".into(),
                context_window: 128000, supports_cache: true, cost_tier: "budget",
                best_for: "代码生成 · 日常对话 · 批量任务 (1折缓存)",
            },
            ModelModel::KimiK3 => ModelProfile {
                model_key: "kimi-k3".into(), display: "Kimi K3".into(),
                context_window: 256000, supports_cache: false, cost_tier: "premium",
                best_for: "超长文档分析 · 项目全局理解",
            },
            ModelModel::KimiK27Code => ModelProfile {
                model_key: "kimi-k2.7-code".into(), display: "Kimi K2.7-Code".into(),
                context_window: 128000, supports_cache: false, cost_tier: "standard",
                best_for: "代码专用 · 算法实现",
            },
            ModelModel::KimiK27CodeHighspeed => ModelProfile {
                model_key: "kimi-k2.7-code-highspeed".into(), display: "Kimi K2.7-Code-HS".into(),
                context_window: 128000, supports_cache: false, cost_tier: "standard",
                best_for: "极速编程 · 低延迟场景",
            },
            ModelModel::Glm52 => ModelProfile {
                model_key: "glm-5.2".into(), display: "GLM-5.2".into(),
                context_window: 128000, supports_cache: false, cost_tier: "standard",
                best_for: "原生Agent规划 · 工具调用",
            },
            ModelModel::Glm5vTurbo => ModelProfile {
                model_key: "glm-5v-turbo".into(), display: "GLM-5V-Turbo".into(),
                context_window: 32000, supports_cache: false, cost_tier: "premium",
                best_for: "视觉理解 · 多模态分析",
            },
            ModelModel::Glm51 => ModelProfile {
                model_key: "glm-5.1".into(), display: "GLM-5.1".into(),
                context_window: 128000, supports_cache: false, cost_tier: "standard",
                best_for: "稳定推理 · 生产环境",
            },
            ModelModel::LanOllamaR1 => ModelProfile {
                model_key: "ollama-local".into(), display: "Ollama Local".into(),
                context_window: 8192, supports_cache: false, cost_tier: "budget",
                best_for: "离线场景 · 零资费 · 隐私优先",
            },
        }
    }
}

// ─── 单轨累加器 ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct CostAccum {
    total_cost_rmb: f64,
    tokens_used: u64,
    call_count: u64,
}

impl CostAccum {
    fn new() -> Self {
        Self { total_cost_rmb: 0.0, tokens_used: 0, call_count: 0 }
    }

    fn add(&mut self, cost: f64, tokens: u32) {
        self.total_cost_rmb += cost;
        self.tokens_used += tokens as u64;
        self.call_count += 1;
    }
}

// ═══════════════════════════════════════════════════════════════════════
// ChronosParallelBillingEngine
// ═══════════════════════════════════════════════════════════════════════

pub struct ChronosParallelBillingEngine {
    official_accum: Mutex<CostAccum>,
    budget_accum: Mutex<CostAccum>,
    router_accum: Mutex<CostAccum>,
    official_rates: ChronosBillingEngine,
    cost_cap: Mutex<f64>,
    cost_cap_enabled: Mutex<bool>,
}

impl ChronosParallelBillingEngine {
    pub fn new() -> Self {
        Self {
            official_accum: Mutex::new(CostAccum::new()),
            budget_accum: Mutex::new(CostAccum::new()),
            router_accum: Mutex::new(CostAccum::new()),
            official_rates: ChronosBillingEngine::new(),
            cost_cap: Mutex::new(5.0),
            cost_cap_enabled: Mutex::new(true),
        }
    }

    // ─── 核心：一次 record，三轨并行更新 ─────────────────────────

    /// 记录一次 API 调用，三轨同步累加。
    /// 消费方（api_client / lib.rs / HybridAgentRouter）调用一次即可。
    pub fn record(
        &self,
        model: &ModelModel,
        prompt_tokens: u32,
        completion_tokens: u32,
        cached_tokens: Option<u32>,
    ) {
        let total_tokens = prompt_tokens + completion_tokens;

        // Tier 1: Official — 复用 billing.rs 精确双轨计算
        let usage = ApiUsage { prompt_tokens, completion_tokens, cached_tokens };
        let official_snap = self.official_rates.calculate_audit_ledger(model, &usage);
        self.official_accum.lock().unwrap().add(official_snap.exact_cost_rmb, total_tokens);

        // Tier 2: Budget — 保守 ×1.2 上浮
        let budget_cost = self.compute_budget_cost(model, prompt_tokens, completion_tokens, cached_tokens);
        self.budget_accum.lock().unwrap().add(budget_cost, total_tokens);

        // Tier 3: Router — 节点简化价
        let router_cost = self.compute_router_cost(model, prompt_tokens, completion_tokens);
        self.router_accum.lock().unwrap().add(router_cost, total_tokens);
    }

    // ─── 查询单轨 ────────────────────────────────────────────────

    pub fn get_ledger(&self, tier: BillingTier) -> CostSnapshot {
        let accum = match tier {
            BillingTier::Official => self.official_accum.lock().unwrap(),
            BillingTier::Budget => self.budget_accum.lock().unwrap(),
            BillingTier::Router => self.router_accum.lock().unwrap(),
        };
        CostSnapshot {
            tier: tier.label().into(),
            total_cost_rmb: accum.total_cost_rmb,
            tokens_used: accum.tokens_used,
            call_count: accum.call_count,
        }
    }

    // ─── 统一仪表盘 ──────────────────────────────────────────────

    pub fn get_dashboard(&self) -> BillingDashboard {
        BillingDashboard {
            official: self.get_ledger(BillingTier::Official),
            budget: self.get_ledger(BillingTier::Budget),
            router: self.get_ledger(BillingTier::Router),
            cost_cap: *self.cost_cap.lock().unwrap(),
            cost_cap_active: *self.cost_cap_enabled.lock().unwrap(),
        }
    }

    // ─── 熔断控制 ────────────────────────────────────────────────

    pub fn is_over_cap(&self) -> bool {
        if !*self.cost_cap_enabled.lock().unwrap() {
            return false;
        }
        self.budget_accum.lock().unwrap().total_cost_rmb >= *self.cost_cap.lock().unwrap()
    }

    pub fn get_budget_total(&self) -> f64 {
        self.budget_accum.lock().unwrap().total_cost_rmb
    }

    pub fn get_cost_cap(&self) -> f64 {
        *self.cost_cap.lock().unwrap()
    }

    pub fn set_cost_cap(&self, cap: f64) {
        *self.cost_cap.lock().unwrap() = cap;
    }

    pub fn set_cost_cap_enabled(&self, enabled: bool) {
        *self.cost_cap_enabled.lock().unwrap() = enabled;
    }

    // ─── 旧数据迁移 ──────────────────────────────────────────────

    /// 将旧 SETTINGS.accumulated_cost 迁移到 Budget 轨道
    pub fn migrate_legacy_cost(&self, legacy_cost: f64) {
        self.budget_accum.lock().unwrap().total_cost_rmb = legacy_cost;
    }

    // ══════════════════════════════════════════════════════════════
    // 降本增效算法 — 模型特性驱动的优化引擎
    // ══════════════════════════════════════════════════════════════

    /// 预估单次调用成本（未发送前使用）
    pub fn estimate_cost(&self, model: &ModelModel, est_prompt_tokens: u32, est_completion_tokens: u32) -> f64 {
        let usage = ApiUsage { prompt_tokens: est_prompt_tokens, completion_tokens: est_completion_tokens, cached_tokens: None };
        self.official_rates.calculate_audit_ledger(model, &usage).exact_cost_rmb
    }

    /// 深度推理模型：DeepSeek V4-Pro / Kimi K3
    pub fn is_deep_reasoning(model: &ModelModel) -> bool {
        matches!(model, ModelModel::DeepSeekV4Pro | ModelModel::KimiK3)
    }

    /// 缓存友好模型：DeepSeek 系列 (90% discount on cache hit)
    pub fn supports_context_cache(model: &ModelModel) -> bool {
        matches!(model, ModelModel::DeepSeekV4Pro | ModelModel::DeepSeekV4Flash)
    }

    /// 按消息长度推荐最优模型，返回节省比例
    pub fn recommend_for_length(&self, message_chars: usize) -> ModelRecommendation {
        let est_tokens = (message_chars as f64 / 3.5) as u32; // Chinese: ~3.5 chars/token

        let budget = self.estimate_cost(&ModelModel::DeepSeekV4Flash, est_tokens, est_tokens / 2);
        let standard = self.estimate_cost(&ModelModel::DeepSeekV4Pro, est_tokens, est_tokens / 2);
        let cheap = self.estimate_cost(&ModelModel::Glm51, est_tokens, est_tokens / 2);

        let (model, cost, savings_pct) = if est_tokens < 4000 {
            (&ModelModel::DeepSeekV4Flash, budget, 0.0)
        } else if est_tokens > 32000 {
            (&ModelModel::KimiK3, self.estimate_cost(&ModelModel::KimiK3, est_tokens, est_tokens / 2), 0.0)
        } else if budget < standard && budget < cheap {
            (&ModelModel::DeepSeekV4Flash, budget,
                ((standard - budget) / standard * 100.0).max(0.0))
        } else if cheap < standard {
            (&ModelModel::Glm51, cheap,
                ((standard - cheap) / standard * 100.0).max(0.0))
        } else {
            (&ModelModel::DeepSeekV4Pro, standard, 0.0)
        };

        ModelRecommendation {
            model_key: model.profile().model_key,
            display: model.profile().display,
            estimated_cost_rmb: cost,
            estimated_tokens: est_tokens,
            savings_vs_pro: savings_pct,
            context_remaining: model.profile().context_window.saturating_sub(est_tokens),
        }
    }

    /// 上下文窗口余量检查
    pub fn check_context_health(&self, model: &ModelModel, current_tokens: u32) -> ContextHealth {
        let profile = model.profile();
        let usage_pct = (current_tokens as f64 / profile.context_window as f64 * 100.0) as u32;
        let status = if usage_pct > 90 { "critical" }
            else if usage_pct > 70 { "warning" }
            else if usage_pct > 50 { "moderate" }
            else { "healthy" };

        let cache_tip = if profile.supports_cache && usage_pct < 50 {
            Some("💡 固定系统提示前置可触发 DeepSeek 一折缓存，后续调用节省 90% 输入费用")
        } else if profile.cost_tier == "premium" && usage_pct > 50 {
            Some("⚠️ 当前使用高价模型，建议压缩上下文或切换至 DeepSeek Flash")
        } else {
            None
        };

        ContextHealth {
            status: status.into(),
            usage_pct,
            remaining: profile.context_window.saturating_sub(current_tokens),
            total: profile.context_window,
            tip: cache_tip.map(|s| s.into()),
        }
    }
}

/// 模型推荐结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRecommendation {
    pub model_key: String,
    pub display: String,
    pub estimated_cost_rmb: f64,
    pub estimated_tokens: u32,
    pub savings_vs_pro: f64,
    pub context_remaining: u32,
}

/// 上下文健康检查
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextHealth {
    pub status: String,
    pub usage_pct: u32,
    pub remaining: u32,
    pub total: u32,
    pub tip: Option<String>,

    // ══════════════════════════════════════════════════════════════
    // 内部：Budget = Official × 1.2（派生自 billing.rs，无重复）
    // ══════════════════════════════════════════════════════════════

    fn compute_budget_cost(
        &self,
        model: &ModelModel,
        prompt_tokens: u32,
        completion_tokens: u32,
        cached_tokens: Option<u32>,
    ) -> f64 {
        let usage = ApiUsage { prompt_tokens, completion_tokens, cached_tokens };
        let official = self.official_rates.calculate_audit_ledger(model, &usage);
        official.exact_cost_rmb * 1.2
    }

    fn router_rate(&self, model: &ModelModel) -> f64 {
        match model {
            ModelModel::DeepSeekV4Pro   => 0.0045,
            ModelModel::DeepSeekV4Flash => 0.0015,
            ModelModel::KimiK3               => 0.004,
            ModelModel::KimiK27Code          => 0.002,
            ModelModel::KimiK27CodeHighspeed => 0.001,
            ModelModel::Glm52       => 0.004,
            ModelModel::Glm5vTurbo  => 0.005,
            ModelModel::Glm51       => 0.002,
            ModelModel::LanOllamaR1 => 0.0,
        }
    }

    fn compute_router_cost(
        &self,
        model: &ModelModel,
        prompt_tokens: u32,
        completion_tokens: u32,
    ) -> f64 {
        (prompt_tokens + completion_tokens) as f64 * self.router_rate(model) / 1000.0
    }
}

impl Default for ChronosParallelBillingEngine {
    fn default() -> Self { Self::new() }
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_all_three_tiers() {
        let engine = ChronosParallelBillingEngine::new();
        engine.record(&ModelModel::DeepSeekV4Flash, 10000, 5000, Some(8000));

        let official = engine.get_ledger(BillingTier::Official);
        let budget = engine.get_ledger(BillingTier::Budget);
        let router = engine.get_ledger(BillingTier::Router);

        // All three should have recorded the same call
        assert_eq!(official.call_count, 1);
        assert_eq!(budget.call_count, 1);
        assert_eq!(router.call_count, 1);
        assert_eq!(official.tokens_used, 15000);
        assert_eq!(budget.tokens_used, 15000);
        assert_eq!(router.tokens_used, 15000);

        // Budget should be ~1.2× Official (within rounding)
        assert!(budget.total_cost_rmb > official.total_cost_rmb);
        let ratio = budget.total_cost_rmb / official.total_cost_rmb;
        assert!(ratio >= 1.15 && ratio <= 1.3,
            "Budget/Official ratio {} out of expected range", ratio);

        // Router should be different (simplified rate)
        assert!(router.total_cost_rmb > 0.0);
    }

    #[test]
    fn test_dashboard_includes_all_tiers() {
        let engine = ChronosParallelBillingEngine::new();
        engine.record(&ModelModel::KimiK3, 5000, 2000, None);

        let dashboard = engine.get_dashboard();
        assert_eq!(dashboard.official.call_count, 1);
        assert_eq!(dashboard.budget.call_count, 1);
        assert_eq!(dashboard.router.call_count, 1);
        assert_eq!(dashboard.cost_cap, 5.0);
        assert!(dashboard.cost_cap_active);
    }

    #[test]
    fn test_cost_cap_enforcement() {
        let engine = ChronosParallelBillingEngine::new();
        engine.set_cost_cap(0.001); // Very low cap
        assert!(!engine.is_over_cap()); // 0 < 0.001

        // Record enough to exceed cap
        engine.record(&ModelModel::KimiK3, 1000000, 1000000, None); // ~¥30 budget
        assert!(engine.is_over_cap());
    }

    #[test]
    fn test_cost_cap_disabled() {
        let engine = ChronosParallelBillingEngine::new();
        engine.set_cost_cap(0.001);
        engine.set_cost_cap_enabled(false);
        engine.record(&ModelModel::KimiK3, 1000000, 1000000, None);
        assert!(!engine.is_over_cap());
    }

    #[test]
    fn test_lan_ollama_zero_cost_all_tiers() {
        let engine = ChronosParallelBillingEngine::new();
        engine.record(&ModelModel::LanOllamaR1, 50000, 50000, None);

        assert_eq!(engine.get_ledger(BillingTier::Official).total_cost_rmb, 0.0);
        assert_eq!(engine.get_ledger(BillingTier::Budget).total_cost_rmb, 0.0);
        assert_eq!(engine.get_ledger(BillingTier::Router).total_cost_rmb, 0.0);
    }

    #[test]
    fn test_multiple_calls_accumulate() {
        let engine = ChronosParallelBillingEngine::new();
        engine.record(&ModelModel::DeepSeekV4Flash, 10000, 5000, None);
        engine.record(&ModelModel::DeepSeekV4Flash, 20000, 10000, Some(15000));
        engine.record(&ModelModel::Glm52, 8000, 4000, None);

        let official = engine.get_ledger(BillingTier::Official);
        assert_eq!(official.call_count, 3);
        assert_eq!(official.tokens_used, 57000);
    }

    #[test]
    fn test_migrate_legacy_cost() {
        let engine = ChronosParallelBillingEngine::new();
        engine.migrate_legacy_cost(3.42);

        let budget = engine.get_ledger(BillingTier::Budget);
        assert_eq!(budget.total_cost_rmb, 3.42);
        // Official and Router should be unaffected
        assert_eq!(engine.get_ledger(BillingTier::Official).total_cost_rmb, 0.0);
        assert_eq!(engine.get_ledger(BillingTier::Router).total_cost_rmb, 0.0);
    }

    #[test]
    fn test_all_models_have_rates() {
        let engine = ChronosParallelBillingEngine::new();
        let models = [
            ModelModel::DeepSeekV4Pro, ModelModel::DeepSeekV4Flash,
            ModelModel::KimiK3, ModelModel::KimiK27Code, ModelModel::KimiK27CodeHighspeed,
            ModelModel::Glm52, ModelModel::Glm5vTurbo, ModelModel::Glm51,
            ModelModel::LanOllamaR1,
        ];
        for model in &models {
            // Should not panic
            engine.record(model, 1000, 500, None);
        }
        let dashboard = engine.get_dashboard();
        assert_eq!(dashboard.official.call_count, models.len() as u64);
    }
}
