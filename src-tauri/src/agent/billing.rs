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

// ─── 2026 官方费率矩阵 (RMB / 1M tokens, 精确到微钱) ────────────

/// (输入价格, 缓存命中输入, 输出价格) — 单位: RMB/1M tokens
type RateTriple = (f64, f64, f64);

pub struct ChronosBillingEngine {
    rates: HashMap<ModelModel, RateTriple>,
}

impl ChronosBillingEngine {
    pub fn new() -> Self {
        let mut rates = HashMap::new();

        // ═══════════════════════════════════════════════════════
        // DeepSeek 官方定价 (2026)
        // 来源: https://api-docs.deepseek.com/zh-cn/quick_start/pricing
        // V4-Pro:  ¥1.00/M输入(未命中) ¥0.10/M输入(缓存命中) ¥4.00/M输出
        // V4-Flash: ¥0.10/M输入(未命中) ¥0.01/M输入(缓存命中) ¥0.40/M输出
        // 缓存命中 = 一折计费 (90% discount)
        // ═══════════════════════════════════════════════════════
        rates.insert(ModelModel::DeepSeekV4Pro,   (1.00, 0.10, 4.00));
        rates.insert(ModelModel::DeepSeekV4Flash, (0.10, 0.01, 0.40));

        // ═══════════════════════════════════════════════════════
        // Kimi (Moonshot) 官方定价 (2026)
        // 来源: https://platform.kimi.com/docs/pricing/chat
        // K3:   ¥8.00/M输入  ¥8.00/M缓存  ¥8.00/M输出
        // K2.7: ¥3.00/M输入  ¥3.00/M缓存  ¥3.00/M输出
        // K2.7-HS: ¥1.00/M输入 ¥1.00/M缓存 ¥1.00/M输出
        // ═══════════════════════════════════════════════════════
        rates.insert(ModelModel::KimiK3,               (8.00,  8.00,  8.00));
        rates.insert(ModelModel::KimiK27Code,          (3.00,  3.00,  3.00));
        rates.insert(ModelModel::KimiK27CodeHighspeed, (1.00,  1.00,  1.00));

        // ═══════════════════════════════════════════════════════
        // GLM (智谱) 官方定价 (2026)
        // 来源: https://bigmodel.cn/pricing
        // GLM-5.2:     ¥1.00/M输入  ¥1.00/M缓存  ¥2.00/M输出
        // GLM-5V-Turbo: ¥3.00/M输入  ¥3.00/M缓存  ¥5.00/M输出 (多模态)
        // GLM-5.1:     ¥0.50/M输入  ¥0.50/M缓存  ¥2.00/M输出
        // ═══════════════════════════════════════════════════════
        rates.insert(ModelModel::Glm52,       (1.00, 1.00, 2.00));
        rates.insert(ModelModel::Glm5vTurbo,  (3.00, 3.00, 5.00));
        rates.insert(ModelModel::Glm51,       (0.50, 0.50, 2.00));

        // LAN 离线: 0 资费
        rates.insert(ModelModel::LanOllamaR1, (0.00, 0.00, 0.00));

        Self { rates }
    }

    /// 预估单次调用费用 (基于输入字符数估算)
    pub fn estimate_cost_from_chars(&self, model: &ModelModel, input_chars: usize, estimated_output_chars: usize) -> BillingSnapshot {
        // 中文≈3.5 chars/token, 英文≈4 chars/token, 取3.5更保守
        let prompt_tokens = (input_chars as f64 / 3.5).ceil() as u32;
        let completion_tokens = (estimated_output_chars as f64 / 3.5).ceil() as u32;
        let usage = ApiUsage { prompt_tokens, completion_tokens, cached_tokens: None };
        self.calculate_audit_ledger(model, &usage)
    }

    /// 预估含缓存命中的费用
    pub fn estimate_cached_cost(&self, model: &ModelModel, prompt_tokens: u32, cached_ratio: f32, completion_tokens: u32) -> BillingSnapshot {
        let cached = (prompt_tokens as f32 * cached_ratio) as u32;
        let usage = ApiUsage { prompt_tokens, completion_tokens, cached_tokens: Some(cached) };
        self.calculate_audit_ledger(model, &usage)
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

        // 微钱精度: 保留6位小数
        let exact_rounded = (exact_total * 1_000_000.0).round() / 1_000_000.0;
        let saved_rounded = (saved * 1_000_000.0).round() / 1_000_000.0;

        BillingSnapshot {
            exact_cost_rmb: exact_rounded,
            saved_cost_rmb: saved_rounded,
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
