// 官方对账与财务审计引擎 (Chronos Billing & Audit Engine)
//
// 2026 各大模型厂商官方计费标准对齐：
// - DeepSeek: https://api-docs.deepseek.com/zh-cn/quick_start/pricing
// - Kimi: https://platform.kimi.com/docs/pricing/chat
// - GLM: https://bigmodel.cn/pricing
//
// 核心功能：
// - 双轨计费：区分缓存命中/未命中，精确到微钱
// - 省钱审计：实时计算端侧拦截+缓存一折省下的真金白银
// - 费率矩阵：官方价格硬编码，每 1M Token RMB

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::agent::router::ModelModel;

// ─── API 用量结构 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub cached_tokens: Option<u32>,
}

// ─── 财务快照 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingSnapshot {
    pub exact_cost_rmb: f64,
    pub saved_cost_rmb: f64,
    pub current_saving_rate: f32,
    pub model_name: String,
    pub tokens_used: u32,
    pub cached_tokens: u32,
}

// ─── 2026 官方费率矩阵 (RMB / 1M tokens) ──────────────────────────

/// (未命中输入, 缓存命中输入, 输出)
type RateTriple = (f64, f64, f64);

pub struct ChronosBillingEngine {
    rates: HashMap<ModelModel, RateTriple>,
}

impl ChronosBillingEngine {
    pub fn new() -> Self {
        let mut rates = HashMap::new();

        // DeepSeek 官方: https://api-docs.deepseek.com/zh-cn/quick_start/pricing
        rates.insert(ModelModel::DeepSeekV4Pro,   (1.00, 0.10, 2.00));
        rates.insert(ModelModel::DeepSeekV4Flash, (0.10, 0.01, 0.20));

        // Kimi 官方: https://platform.kimi.com/docs/pricing/chat
        rates.insert(ModelModel::KimiK3,               (15.00, 15.00, 15.00));
        rates.insert(ModelModel::KimiK27Code,          (5.00,  5.00,  5.00));
        rates.insert(ModelModel::KimiK27CodeHighspeed, (5.00,  5.00,  5.00));

        // GLM 官方: https://bigmodel.cn/pricing
        rates.insert(ModelModel::Glm52,       (1.00, 1.00, 2.00));
        rates.insert(ModelModel::Glm5vTurbo,  (2.00, 2.00, 4.00));
        rates.insert(ModelModel::Glm51,       (1.00, 1.00, 2.00));

        // LAN 离线: 0 资费
        rates.insert(ModelModel::LanOllamaR1, (0.00, 0.00, 0.00));

        Self { rates }
    }

    /// 核心：解析 API 用量，计算真实费用 + 省钱审计
    pub fn calculate_audit_ledger(
        &self,
        model: &ModelModel,
        usage: &ApiUsage,
    ) -> BillingSnapshot {
        let (input_miss, input_hit, output) =
            self.rates.get(model).cloned().unwrap_or((1.0, 1.0, 2.0));

        let cached = usage.cached_tokens.unwrap_or(0);
        let normal_input = usage.prompt_tokens.saturating_sub(cached);

        // 真实费用
        let exact_input = (normal_input as f64 * input_miss + cached as f64 * input_hit) / 1_000_000.0;
        let exact_output = usage.completion_tokens as f64 * output / 1_000_000.0;
        let exact_total = exact_input + exact_output;

        // 假设无缓存时的虚拟费用
        let hypothetical_input = usage.prompt_tokens as f64 * input_miss / 1_000_000.0;
        let hypothetical_total = hypothetical_input + exact_output;

        // 省下的钱
        let saved = if *model == ModelModel::LanOllamaR1 {
            hypothetical_total
        } else {
            (hypothetical_total - exact_total).max(0.0)
        };

        let rate = if hypothetical_total > 0.0 {
            ((saved / hypothetical_total) * 100.0) as f32
        } else {
            0.0
        };

        BillingSnapshot {
            exact_cost_rmb: exact_total,
            saved_cost_rmb: saved,
            current_saving_rate: rate,
            model_name: model.display().into(),
            tokens_used: usage.prompt_tokens + usage.completion_tokens,
            cached_tokens: cached,
        }
    }
}

impl Default for ChronosBillingEngine {
    fn default() -> Self { Self::new() }
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deepseek_flash_cached() {
        let engine = ChronosBillingEngine::new();
        let usage = ApiUsage {
            prompt_tokens: 10000,
            completion_tokens: 5000,
            cached_tokens: Some(8000),
        };
        let snap = engine.calculate_audit_ledger(&ModelModel::DeepSeekV4Flash, &usage);
        // 8000 cached × 0.01/1M ≈ ¥0.00008, 2000 normal × 0.10/1M ≈ ¥0.0002
        // input ≈ ¥0.00028, output 5000 × 0.20/1M = ¥0.001
        // total ≈ ¥0.00128
        assert!(snap.exact_cost_rmb < 0.01);
        assert!(snap.saved_cost_rmb > 0.0);
        assert!(snap.current_saving_rate > 50.0);
    }

    #[test]
    fn test_lan_ollama_free() {
        let engine = ChronosBillingEngine::new();
        let usage = ApiUsage { prompt_tokens: 1000, completion_tokens: 500, cached_tokens: None };
        let snap = engine.calculate_audit_ledger(&ModelModel::LanOllamaR1, &usage);
        assert_eq!(snap.exact_cost_rmb, 0.0);
        assert!(snap.saved_cost_rmb > 0.0);
    }

    #[test]
    fn test_deepseek_pro_no_cache() {
        let engine = ChronosBillingEngine::new();
        let usage = ApiUsage { prompt_tokens: 100000, completion_tokens: 50000, cached_tokens: None };
        let snap = engine.calculate_audit_ledger(&ModelModel::DeepSeekV4Pro, &usage);
        // 100K × 1.0/1M = ¥0.10, 50K × 2.0/1M = ¥0.10, total ≈ ¥0.20
        assert!(snap.exact_cost_rmb > 0.1);
        assert!(snap.exact_cost_rmb < 0.5);
    }
}

// ─── 公共工具函数（供全项目使用） ─────────────────────────────

/// 统一模型字符串→枚举解析器（全项目唯一权威来源）
/// 供 api_client、billing_engine、lib.rs 共用
pub fn parse_model_string(model: &str) -> ModelModel {
    if model.contains("deepseek-v4-pro") {
        ModelModel::DeepSeekV4Pro
    } else if model.contains("deepseek-v4-flash") {
        ModelModel::DeepSeekV4Flash
    } else if model.contains("kimi-k3") {
        ModelModel::KimiK3
    } else if model.contains("kimi-k2.7-code-highspeed") {
        ModelModel::KimiK27CodeHighspeed
    } else if model.contains("kimi-k2.7") {
        ModelModel::KimiK27Code
    } else if model.contains("glm-5v-turbo") {
        ModelModel::Glm5vTurbo
    } else if model.contains("glm-5.2") {
        ModelModel::Glm52
    } else if model.contains("glm-5.1") {
        ModelModel::Glm51
    } else if model.contains("ollama") || model.contains("local") {
        ModelModel::LanOllamaR1
    } else {
        ModelModel::DeepSeekV4Flash // safe fallback
    }
}

/// 根据模型字符串名称查询官方费率并估算费用
/// 此函数是 ChronosBillingEngine 的轻量封装，统一全项目费率来源
pub fn estimate_cost_from_model_name(model: &str, prompt_tokens: u32, completion_tokens: u32) -> f64 {
    let engine = ChronosBillingEngine::new();
    let model_enum = parse_model_string(model);
    let usage = ApiUsage { prompt_tokens, completion_tokens, cached_tokens: None };
    engine.calculate_audit_ledger(&model_enum, &usage).exact_cost_rmb
}
