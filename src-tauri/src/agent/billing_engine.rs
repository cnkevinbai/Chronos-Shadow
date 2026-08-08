// 三层并行计费引擎 + 模型降本增效算法

use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use crate::agent::router::ModelModel;
use crate::agent::billing::{ChronosBillingEngine, ApiUsage};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillingTier { Official, Budget, Router }
impl BillingTier { pub fn label(&self) -> &str { match self { BillingTier::Official => "official", BillingTier::Budget => "budget", BillingTier::Router => "router" } } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSnapshot { pub tier: String, pub total_cost_rmb: f64, pub tokens_used: u64, pub call_count: u64 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingDashboard { pub official: CostSnapshot, pub budget: CostSnapshot, pub router: CostSnapshot, pub cost_cap: f64, pub cost_cap_active: bool }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile { pub model_key: String, pub display: String, pub context_window: u32, pub supports_cache: bool, pub cost_tier: &'static str, pub best_for: &'static str }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRecommendation { pub model_key: String, pub display: String, pub estimated_cost_rmb: f64, pub estimated_tokens: u32, pub savings_vs_pro: f64, pub context_remaining: u32 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextHealth { pub status: String, pub usage_pct: u32, pub remaining: u32, pub total: u32, pub tip: Option<String> }

impl ModelModel {
    pub fn profile(&self) -> ModelProfile { match self {
        ModelModel::DeepSeekV4Pro => ModelProfile { model_key:"deepseek-v4-pro".into(),display:"DeepSeek V4-Pro".into(),context_window:128000,supports_cache:true,cost_tier:"standard",best_for:"深度推理·架构设计·代码审查" },
        ModelModel::DeepSeekV4Flash => ModelProfile { model_key:"deepseek-v4-flash".into(),display:"DeepSeek V4-Flash".into(),context_window:128000,supports_cache:true,cost_tier:"budget",best_for:"代码生成·日常对话·批量任务(1折缓存)" },
        ModelModel::KimiK3 => ModelProfile { model_key:"kimi-k3".into(),display:"Kimi K3".into(),context_window:256000,supports_cache:false,cost_tier:"premium",best_for:"超长文档分析·项目全局理解" },
        ModelModel::KimiK27Code => ModelProfile { model_key:"kimi-k2.7-code".into(),display:"Kimi K2.7-Code".into(),context_window:128000,supports_cache:false,cost_tier:"standard",best_for:"代码专用·算法实现" },
        ModelModel::KimiK27CodeHighspeed => ModelProfile { model_key:"kimi-k2.7-code-highspeed".into(),display:"Kimi K2.7-Code-HS".into(),context_window:128000,supports_cache:false,cost_tier:"standard",best_for:"极速编程·低延迟场景" },
        ModelModel::Glm52 => ModelProfile { model_key:"glm-5.2".into(),display:"GLM-5.2".into(),context_window:128000,supports_cache:false,cost_tier:"standard",best_for:"原生Agent规划·工具调用" },
        ModelModel::Glm5vTurbo => ModelProfile { model_key:"glm-5v-turbo".into(),display:"GLM-5V-Turbo".into(),context_window:32000,supports_cache:false,cost_tier:"premium",best_for:"视觉理解·多模态分析" },
        ModelModel::Glm51 => ModelProfile { model_key:"glm-5.1".into(),display:"GLM-5.1".into(),context_window:128000,supports_cache:false,cost_tier:"standard",best_for:"稳定推理·生产环境" },
        ModelModel::LanOllamaR1 => ModelProfile { model_key:"ollama-local".into(),display:"Ollama Local".into(),context_window:8192,supports_cache:false,cost_tier:"budget",best_for:"离线场景·零资费·隐私优先" },
    }}
}

#[derive(Debug, Clone)]
struct CostAccum { total_cost_rmb: f64, tokens_used: u64, call_count: u64 }
impl CostAccum { fn new() -> Self { Self { total_cost_rmb: 0.0, tokens_used: 0, call_count: 0 } }
    fn add(&mut self, cost: f64, tokens: u32) { self.total_cost_rmb += cost; self.tokens_used += tokens as u64; self.call_count += 1; } }

pub struct ChronosParallelBillingEngine {
    official_accum: Mutex<CostAccum>, budget_accum: Mutex<CostAccum>, router_accum: Mutex<CostAccum>,
    official_rates: ChronosBillingEngine, cost_cap: Mutex<f64>, cost_cap_enabled: Mutex<bool>,
}

impl ChronosParallelBillingEngine {
    pub fn new() -> Self { Self { official_accum: Mutex::new(CostAccum::new()), budget_accum: Mutex::new(CostAccum::new()), router_accum: Mutex::new(CostAccum::new()), official_rates: ChronosBillingEngine::new(), cost_cap: Mutex::new(5.0), cost_cap_enabled: Mutex::new(true) } }
    pub fn record(&self, model: &ModelModel, prompt_tokens: u32, completion_tokens: u32, cached_tokens: Option<u32>) {
        let total = prompt_tokens + completion_tokens;
        let usage = ApiUsage { prompt_tokens, completion_tokens, cached_tokens };
        let snap = self.official_rates.calculate_audit_ledger(model, &usage);
        self.official_accum.lock().unwrap().add(snap.exact_cost_rmb, total);
        self.budget_accum.lock().unwrap().add(snap.exact_cost_rmb * 1.2, total);
        let rc = (total as f64 * self.router_rate(model) / 1000.0);
        self.router_accum.lock().unwrap().add(rc, total);
    }
    pub fn get_ledger(&self, tier: BillingTier) -> CostSnapshot {
        let a = match tier { BillingTier::Official => self.official_accum.lock().unwrap(), BillingTier::Budget => self.budget_accum.lock().unwrap(), BillingTier::Router => self.router_accum.lock().unwrap() };
        CostSnapshot { tier: tier.label().into(), total_cost_rmb: a.total_cost_rmb, tokens_used: a.tokens_used, call_count: a.call_count }
    }
    pub fn get_dashboard(&self) -> BillingDashboard { BillingDashboard { official: self.get_ledger(BillingTier::Official), budget: self.get_ledger(BillingTier::Budget), router: self.get_ledger(BillingTier::Router), cost_cap: *self.cost_cap.lock().unwrap(), cost_cap_active: *self.cost_cap_enabled.lock().unwrap() } }
    pub fn is_over_cap(&self) -> bool { if !*self.cost_cap_enabled.lock().unwrap() { return false; } self.budget_accum.lock().unwrap().total_cost_rmb >= *self.cost_cap.lock().unwrap() }
    pub fn get_budget_total(&self) -> f64 { self.budget_accum.lock().unwrap().total_cost_rmb }
    pub fn get_cost_cap(&self) -> f64 { *self.cost_cap.lock().unwrap() }
    pub fn set_cost_cap(&self, cap: f64) { *self.cost_cap.lock().unwrap() = cap; }
    pub fn set_cost_cap_enabled(&self, enabled: bool) { *self.cost_cap_enabled.lock().unwrap() = enabled; }
    pub fn migrate_legacy_cost(&self, legacy_cost: f64) { self.budget_accum.lock().unwrap().total_cost_rmb = legacy_cost; }
    pub fn estimate_cost(&self, model: &ModelModel, pt: u32, ct: u32) -> f64 { let u = ApiUsage { prompt_tokens: pt, completion_tokens: ct, cached_tokens: None }; self.official_rates.calculate_audit_ledger(model, &u).exact_cost_rmb }
    pub fn supports_context_cache(model: &ModelModel) -> bool { matches!(model, ModelModel::DeepSeekV4Pro | ModelModel::DeepSeekV4Flash) }
    pub fn recommend_for_length(&self, chars: usize) -> ModelRecommendation {
        let est = (chars as f64 / 3.5) as u32;
        let budget = self.estimate_cost(&ModelModel::DeepSeekV4Flash, est, est/2);
        let standard = self.estimate_cost(&ModelModel::DeepSeekV4Pro, est, est/2);
        let cheap = self.estimate_cost(&ModelModel::Glm51, est, est/2);
        let (model, cost, sv) = if est < 4000 { (&ModelModel::DeepSeekV4Flash, budget, 0.0) }
        else if est > 32000 { let k = self.estimate_cost(&ModelModel::KimiK3, est, est/2); (&ModelModel::KimiK3, k, 0.0) }
        else if budget < standard && budget < cheap { (&ModelModel::DeepSeekV4Flash, budget, ((standard-budget)/standard*100.0).max(0.0)) }
        else if cheap < standard { (&ModelModel::Glm51, cheap, ((standard-cheap)/standard*100.0).max(0.0)) }
        else { (&ModelModel::DeepSeekV4Pro, standard, 0.0) };
        let p = model.profile();
        ModelRecommendation { model_key: p.model_key, display: p.display, estimated_cost_rmb: cost, estimated_tokens: est, savings_vs_pro: sv, context_remaining: p.context_window.saturating_sub(est) }
    }
    pub fn check_context_health(&self, model: &ModelModel, tokens: u32) -> ContextHealth {
        let p = model.profile();
        let pct = (tokens as f64 / p.context_window as f64 * 100.0) as u32;
        let status = if pct > 90 { "critical" } else if pct > 70 { "warning" } else if pct > 50 { "moderate" } else { "healthy" };
        let tip = if p.supports_cache && pct < 50 { Some("固定系统提示前置可触发DeepSeek一折缓存".into()) } else if p.cost_tier == "premium" && pct > 50 { Some("建议切换至DeepSeek Flash节省费用".into()) } else { None };
        ContextHealth { status: status.into(), usage_pct: pct, remaining: p.context_window.saturating_sub(tokens), total: p.context_window, tip }
    }
    fn router_rate(&self, model: &ModelModel) -> f64 { match model { ModelModel::DeepSeekV4Pro=>0.0045,ModelModel::DeepSeekV4Flash=>0.0015,ModelModel::KimiK3=>0.004,ModelModel::KimiK27Code=>0.002,ModelModel::KimiK27CodeHighspeed=>0.001,ModelModel::Glm52=>0.004,ModelModel::Glm5vTurbo=>0.005,ModelModel::Glm51=>0.002,ModelModel::LanOllamaR1=>0.0 } }
}

impl Default for ChronosParallelBillingEngine { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_record() { let e = ChronosParallelBillingEngine::new(); e.record(&ModelModel::DeepSeekV4Flash,10000,5000,Some(8000)); assert_eq!(e.get_ledger(BillingTier::Official).call_count,1); }
    #[test] fn test_dashboard() { let e = ChronosParallelBillingEngine::new(); e.record(&ModelModel::KimiK3,5000,2000,None); assert_eq!(e.get_dashboard().cost_cap,5.0); }
    #[test] fn test_cap() { let e = ChronosParallelBillingEngine::new(); e.set_cost_cap(0.001); e.record(&ModelModel::KimiK3,1000000,1000000,None); assert!(e.is_over_cap()); }
    #[test] fn test_recommend() { let e = ChronosParallelBillingEngine::new(); let r = e.recommend_for_length(500); assert!(r.estimated_cost_rmb < 0.01); }
    #[test] fn test_health() { let e = ChronosParallelBillingEngine::new(); let h = e.check_context_health(&ModelModel::DeepSeekV4Flash, 100000); assert_eq!(h.status, "critical"); }
    #[test] fn test_profile() { assert!(ModelModel::DeepSeekV4Flash.profile().supports_cache); assert_eq!(ModelModel::KimiK3.profile().context_window, 256000); }
}
