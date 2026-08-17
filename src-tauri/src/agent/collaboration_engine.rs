// 多模型协作引擎 (Multi-Model Collaboration Engine)
//
// 核心功能：
//   1. 并行模型执行 — 同一任务同时发送给多个模型，对比结果选最优
//   2. 模型投票仲裁 — 关键决策由多模型投票决定，减少单一模型幻觉
//   3. 质量反馈闭环 — 追踪每模型的成功率/延迟/成本，动态调整路由权重
//   4. 自动降级切换 — 模型超时/错误时毫秒级切换到备用模型
//   5. 成本优化 — 在满足质量阈值的前提下选择最便宜的模型
//
// 设计原则：
//   1. 质量优先 — 关键任务宁可多花成本也要保证结果正确
//   2. 快速降级 — 主模型失败 < 100ms 内切换到备用
//   3. 持续学习 — 每次执行结果反馈到质量评分系统
//   4. 透明可观测 — 所有决策可追溯可审计

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── 协作模式 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CollaborationMode {
    /// 单模型：直接路由到最优模型 (默认)
    Single,
    /// 并行对比：2+ 模型同时执行，选质量最高的结果
    Parallel,
    /// 投票仲裁：3+ 模型投票，多数一致的结果胜出
    Voting,
    /// 级联接力：模型A先回答，模型B审核/改进
    Cascade,
    /// 分工协作：任务拆解后分派给不同模型
    DivideAndConquer,
}

impl CollaborationMode {
    pub fn label(&self) -> &str {
        match self {
            Self::Single => "单模型",
            Self::Parallel => "并行对比",
            Self::Voting => "投票仲裁",
            Self::Cascade => "级联接力",
            Self::DivideAndConquer => "分工协作",
        }
    }
}

// ─── 模型能力画像 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapability {
    pub model_name: String,
    /// 综合质量评分 (0-100)
    pub quality_score: f64,
    /// 平均延迟 (ms)
    pub avg_latency_ms: u64,
    /// 每 1K token 成本 (CNY)
    pub cost_per_1k_tokens: f64,
    /// 上下文窗口大小
    pub context_window: u32,
    /// 成功率
    pub success_rate: f64,
    /// 分任务类型的质量评分
    pub per_task_quality: HashMap<String, f64>,
    /// 总执行次数
    pub total_executions: u64,
    /// 总成功次数
    pub total_successes: u64,
    /// 最近一次执行时间
    pub last_execution: Option<String>,
    /// 是否在线
    pub online: bool,
}

impl ModelCapability {
    pub fn new(model_name: &str) -> Self {
        Self {
            model_name: model_name.into(),
            quality_score: 80.0,
            avg_latency_ms: 2000,
            cost_per_1k_tokens: 0.001,
            context_window: 131072,
            success_rate: 0.99,
            per_task_quality: HashMap::new(),
            total_executions: 0,
            total_successes: 0,
            last_execution: None,
            online: true,
        }
    }

    /// 成本效率分：质量 / 每百万 token 成本（cost_per_1k_tokens × 1000）
    pub fn efficiency_score(&self) -> f64 {
        if self.cost_per_1k_tokens <= 0.0 { return self.quality_score; }
        self.quality_score / (self.cost_per_1k_tokens * 1000.0).max(0.01)
    }
}

// ─── 协作执行结果 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationResult {
    /// 使用的协作模式
    pub mode: CollaborationMode,
    /// 最终选中的内容
    pub final_content: String,
    /// 使用的模型
    pub selected_model: String,
    /// 各个模型的输出
    pub model_outputs: Vec<ModelOutput>,
    /// 总耗时 (ms)
    pub total_latency_ms: u64,
    /// 总成本估算
    pub total_cost: f64,
    /// 是否经过了降级切换
    pub fallback_triggered: bool,
    /// 决策理由
    pub decision_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelOutput {
    pub model_name: String,
    pub content: String,
    pub latency_ms: u64,
    pub cost: f64,
    pub success: bool,
    pub quality_indicators: HashMap<String, f64>,
    pub error: Option<String>,
}

// ─── 执行记录 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub id: String,
    pub timestamp: String,
    pub task_type: String,
    pub mode: CollaborationMode,
    pub models_used: Vec<String>,
    pub selected_model: String,
    pub latency_ms: u64,
    pub cost: f64,
    pub success: bool,
    pub quality_score: f64,
}

// ─── 多模型协作引擎 ────────────────────────────────────────────────

pub struct CollaborationEngine {
    /// 模型能力画像
    pub model_profiles: HashMap<String, ModelCapability>,
    /// 执行历史
    pub execution_history: Vec<ExecutionRecord>,
    /// 默认协作模式
    pub default_mode: CollaborationMode,
    /// 并行执行超时 (ms)
    pub parallel_timeout_ms: u64,
    /// 投票所需最少模型数
    pub min_voting_models: usize,
    /// 质量阈值 (低于此分数触发备选)
    pub quality_threshold: f64,
    /// 执行计数器
    execution_counter: u64,
}

impl CollaborationEngine {
    pub fn new() -> Self {
        let mut model_profiles = HashMap::new();

        // cost_per_1k_tokens 对齐 billing.rs 官方输入定价（¥/1k tokens）：
        //   V4-Pro ¥1.00/M · V4-Flash ¥0.10/M · Kimi K3 ¥8/M · K2.7-Code ¥3/M · GLM-5.2 ¥1/M

        // DeepSeek 系列
        model_profiles.insert("deepseek-v4-pro".into(), ModelCapability {
            model_name: "deepseek-v4-pro".into(),
            quality_score: 92.0,
            avg_latency_ms: 2500,
            cost_per_1k_tokens: 0.001,
            context_window: 131072,
            success_rate: 0.995,
            per_task_quality: HashMap::from([
                ("architecture".into(), 95.0), ("security".into(), 93.0),
                ("code_review".into(), 90.0), ("debug".into(), 88.0),
            ]),
            total_executions: 0, total_successes: 0,
            last_execution: None, online: true,
        });

        model_profiles.insert("deepseek-v4-flash".into(), ModelCapability {
            model_name: "deepseek-v4-flash".into(),
            quality_score: 85.0,
            avg_latency_ms: 800,
            cost_per_1k_tokens: 0.0001,
            context_window: 65536,
            success_rate: 0.99,
            per_task_quality: HashMap::from([
                ("code_generation".into(), 90.0), ("refactor".into(), 87.0),
                ("testing".into(), 85.0), ("documentation".into(), 83.0),
            ]),
            total_executions: 0, total_successes: 0,
            last_execution: None, online: true,
        });

        // Kimi 系列
        model_profiles.insert("kimi-k3".into(), ModelCapability {
            model_name: "kimi-k3".into(),
            quality_score: 90.0,
            avg_latency_ms: 3000,
            cost_per_1k_tokens: 0.008,
            context_window: 1_000_000,
            success_rate: 0.99,
            per_task_quality: HashMap::from([
                ("legal".into(), 95.0), ("analysis".into(), 92.0),
                ("long_context".into(), 98.0),
            ]),
            total_executions: 0, total_successes: 0,
            last_execution: None, online: true,
        });

        model_profiles.insert("kimi-k2.7-code".into(), ModelCapability {
            model_name: "kimi-k2.7-code".into(),
            quality_score: 86.0,
            avg_latency_ms: 1500,
            cost_per_1k_tokens: 0.003,
            context_window: 131072,
            success_rate: 0.985,
            per_task_quality: HashMap::from([
                ("code_generation".into(), 89.0), ("debug".into(), 85.0),
            ]),
            total_executions: 0, total_successes: 0,
            last_execution: None, online: true,
        });

        // GLM 系列
        model_profiles.insert("glm-5.2".into(), ModelCapability {
            model_name: "glm-5.2".into(),
            quality_score: 87.0,
            avg_latency_ms: 2000,
            cost_per_1k_tokens: 0.001,
            context_window: 131072,
            success_rate: 0.99,
            per_task_quality: HashMap::from([
                ("api_design".into(), 90.0), ("planning".into(), 92.0),
                ("ui_design".into(), 88.0),
            ]),
            total_executions: 0, total_successes: 0,
            last_execution: None, online: true,
        });

        // LAN 本地模型
        model_profiles.insert("ollama-local".into(), ModelCapability {
            model_name: "ollama-local".into(),
            quality_score: 70.0,
            avg_latency_ms: 5000,
            cost_per_1k_tokens: 0.0,
            context_window: 32768,
            success_rate: 0.95,
            per_task_quality: HashMap::new(),
            total_executions: 0, total_successes: 0,
            last_execution: None, online: true,
        });

        Self {
            model_profiles,
            execution_history: Vec::new(),
            default_mode: CollaborationMode::Single,
            parallel_timeout_ms: 10000,
            min_voting_models: 3,
            quality_threshold: 70.0,
            execution_counter: 0,
        }
    }

    // ── 模型选择 ──────────────────────────────────────────────

    /// 根据任务类型选择最优模型
    pub fn select_best_model(&self, task_type: &str, prefer_cheap: bool) -> Option<String> {
        let mut candidates: Vec<&ModelCapability> = self.model_profiles.values()
            .filter(|p| p.online)
            .collect();

        if candidates.is_empty() { return None; }

        // 按任务类型质量排序
        candidates.sort_by(|a, b| {
            let qa = a.per_task_quality.get(task_type).copied().unwrap_or(a.quality_score);
            let qb = b.per_task_quality.get(task_type).copied().unwrap_or(b.quality_score);

            if prefer_cheap {
                // 成本效率 = 质量 / 每百万 token 成本；免费模型（成本 0）效率退化为质量本身
                let ea = if a.cost_per_1k_tokens <= 0.0 { qa } else { qa / (a.cost_per_1k_tokens * 1000.0).max(0.01) };
                let eb = if b.cost_per_1k_tokens <= 0.0 { qb } else { qb / (b.cost_per_1k_tokens * 1000.0).max(0.01) };
                eb.partial_cmp(&ea).unwrap()
            } else {
                qb.partial_cmp(&qa).unwrap()
            }
        });

        Some(candidates[0].model_name.clone())
    }

    /// v2: UCB 探索-利用模型选择（科学化 — 多臂老虎机 UCB1 算法）
    ///
    /// 在「质量均值（利用）」与「不确定性（探索）」间平衡：
    /// 执行次数少的模型获得探索奖励，避免过早收敛到次优模型。
    /// `exploration_weight` 越大越偏向探索（默认 2.0）。
    pub fn select_best_model_ucb(&self, task_type: &str, exploration_weight: f64) -> Option<String> {
        let candidates: Vec<&ModelCapability> = self.model_profiles.values()
            .filter(|p| p.online)
            .collect();
        if candidates.is_empty() { return None; }

        let total: u64 = candidates.iter().map(|p| p.total_executions).sum();

        candidates.into_iter()
            .max_by(|a, b| {
                let qa = a.per_task_quality.get(task_type).copied().unwrap_or(a.quality_score);
                let qb = b.per_task_quality.get(task_type).copied().unwrap_or(b.quality_score);
                let na = a.total_executions.max(1) as f64;
                let nb = b.total_executions.max(1) as f64;
                // UCB1 探索奖励：c * sqrt(ln(N) / n)
                let ea = exploration_weight * ((total.max(1) as f64).ln() / na).sqrt();
                let eb = exploration_weight * ((total.max(1) as f64).ln() / nb).sqrt();
                (qa + ea).partial_cmp(&(qb + eb)).unwrap()
            })
            .map(|p| p.model_name.clone())
    }

    /// 获取 Top N 个候选模型 (用于并行/投票)
    pub fn top_models(&self, task_type: &str, n: usize) -> Vec<String> {
        let mut candidates: Vec<&ModelCapability> = self.model_profiles.values()
            .filter(|p| p.online)
            .collect();

        candidates.sort_by(|a, b| {
            let qa = a.per_task_quality.get(task_type).copied().unwrap_or(a.quality_score);
            let qb = b.per_task_quality.get(task_type).copied().unwrap_or(b.quality_score);
            qb.partial_cmp(&qa).unwrap()
        });

        candidates.iter().take(n).map(|c| c.model_name.clone()).collect()
    }

    /// 获取备用模型列表 (当前模型失败时的降级目标)
    pub fn fallback_models(&self, primary_model: &str, task_type: &str) -> Vec<String> {
        self.top_models(task_type, 5)
            .into_iter()
            .filter(|m| m != primary_model)
            .take(3)
            .collect()
    }

    // ── 质量反馈 ──────────────────────────────────────────────

    /// 记录执行结果，更新模型能力画像
    pub fn record_execution(
        &mut self,
        model_name: &str,
        task_type: &str,
        success: bool,
        latency_ms: u64,
        quality_score: f64,
    ) {
        self.execution_counter += 1;

        let record = ExecutionRecord {
            id: format!("exec-{:06}", self.execution_counter),
            timestamp: now_iso(),
            task_type: task_type.into(),
            mode: CollaborationMode::Single,
            models_used: vec![model_name.into()],
            selected_model: model_name.into(),
            latency_ms,
            cost: 0.0,
            success,
            quality_score,
        };
        self.execution_history.push(record);
        if self.execution_history.len() > 500 {
            self.execution_history.remove(0);
        }

        // 更新能力画像
        if let Some(profile) = self.model_profiles.get_mut(model_name) {
            profile.total_executions += 1;
            if success { profile.total_successes += 1; }
            profile.success_rate = if profile.total_executions > 0 {
                profile.total_successes as f64 / profile.total_executions as f64
            } else { 1.0 };

            // 指数移动平均更新延迟
            profile.avg_latency_ms = ((profile.avg_latency_ms as f64 * 0.7) + (latency_ms as f64 * 0.3)) as u64;

            // 更新质量评分
            profile.quality_score = profile.quality_score * 0.8 + quality_score * 0.2;

            // 更新分任务质量
            let entry = profile.per_task_quality.entry(task_type.into()).or_insert(quality_score);
            *entry = *entry * 0.8 + quality_score * 0.2;

            profile.last_execution = Some(now_iso());
        }
    }

    /// 记录协作执行结果
    pub fn record_collaboration(
        &mut self,
        result: &CollaborationResult,
        task_type: &str,
    ) {
        for output in &result.model_outputs {
            let quality = output.quality_indicators.get("overall").copied().unwrap_or(75.0);
            self.record_execution(
                &output.model_name, task_type, output.success,
                output.latency_ms, quality,
            );
        }

        let record = ExecutionRecord {
            id: format!("exec-{:06}", self.execution_counter),
            timestamp: now_iso(),
            task_type: task_type.into(),
            mode: result.mode,
            models_used: result.model_outputs.iter().map(|o| o.model_name.clone()).collect(),
            selected_model: result.selected_model.clone(),
            latency_ms: result.total_latency_ms,
            cost: result.total_cost,
            success: !result.fallback_triggered,
            quality_score: 85.0,
        };
        self.execution_history.push(record);
    }

    // ── 决策辅助 ──────────────────────────────────────────────

    /// 🔬 Round-Robin 多模型投票 (Ensemble Voting)
    /// 算法: 查询 N 个模型 → 提取关键回答 → 多数投票选出最终答案
    /// 参考: LLM-Blender / Mixture-of-Agents 论文
    pub fn voting_consensus(
        &self,
        outputs: &[(String, String)], // (model_name, response_text)
        min_agreement: f64,
    ) -> Option<(String, f64)> {
        if outputs.is_empty() { return None; }
        if outputs.len() == 1 { return Some((outputs[0].0.clone(), 1.0)); }

        // 提取每个回答的关键句子作为投票依据
        let extracts: Vec<Vec<String>> = outputs.iter()
            .map(|(_, text)| {
                text.split(|c: char| c == '.' || c == '\n')
                    .map(|s| s.trim().to_lowercase())
                    .filter(|s| s.len() > 10)
                    .take(5)
                    .collect()
            })
            .collect();

        // 计算两两相似度 (Jaccard 简化)
        let n = outputs.len();
        let mut votes = vec![0u32; n];
        for i in 0..n {
            for j in (i + 1)..n {
                let set_i: std::collections::HashSet<_> = extracts[i].iter().collect();
                let set_j: std::collections::HashSet<_> = extracts[j].iter().collect();
                let intersection = set_i.intersection(&set_j).count();
                let union = set_i.union(&set_j).count();
                let sim = if union > 0 { intersection as f64 / union as f64 } else { 0.0 };
                if sim > 0.3 {
                    votes[i] += 1;
                    votes[j] += 1;
                }
            }
        }

        // 找最高票
        let max_votes = *votes.iter().max().unwrap_or(&0);
        let max_idx = votes.iter().position(|&v| v == max_votes).unwrap_or(0);
        let agreement = max_votes as f64 / (n - 1).max(1) as f64;

        if agreement >= min_agreement {
            Some((outputs[max_idx].0.clone(), agreement))
        } else {
            None // 未达成共识
        }
    }

    /// 决定使用哪种协作模式
    pub fn decide_mode(&self, _task_type: &str, task_complexity: f64, is_critical: bool) -> CollaborationMode {
        if is_critical && task_complexity > 0.7 {
            return CollaborationMode::Voting;
        }
        if task_complexity > 0.8 {
            return CollaborationMode::Parallel;
        }
        if task_complexity > 0.5 {
            return CollaborationMode::Cascade;
        }
        CollaborationMode::Single
    }

    /// 获取模型推荐理由
    pub fn recommend_model_with_reason(&self, task_type: &str, prefer_cheap: bool) -> (String, String) {
        let best = self.select_best_model(task_type, prefer_cheap);
        if let Some(ref model) = best {
            if let Some(profile) = self.model_profiles.get(model) {
                let task_q = profile.per_task_quality.get(task_type).copied().unwrap_or(profile.quality_score);
                let eff = profile.efficiency_score();
                return (model.clone(), format!(
                    "质量:{:.0} 效率:{:.0} 延迟:{}ms 成本:¥{:.4}/1K",
                    task_q, eff, profile.avg_latency_ms, profile.cost_per_1k_tokens
                ));
            }
        }
        ("deepseek-v4-flash".into(), "默认兜底模型".into())
    }

    // ── 统计 ──────────────────────────────────────────────────

    pub fn get_model_ranking(&self) -> Vec<&ModelCapability> {
        let mut profiles: Vec<&ModelCapability> = self.model_profiles.values().collect();
        profiles.sort_by(|a, b| b.quality_score.partial_cmp(&a.quality_score).unwrap());
        profiles
    }

    pub fn stats(&self) -> serde_json::Value {
        let total = self.execution_history.len();
        let successes = self.execution_history.iter().filter(|r| r.success).count();
        serde_json::json!({
            "total_executions": total,
            "success_rate": if total > 0 { successes as f64 / total as f64 } else { 0.0 },
            "models_tracked": self.model_profiles.len(),
            "model_ranking": self.get_model_ranking().iter().map(|p| serde_json::json!({
                "name": p.model_name,
                "quality": format!("{:.0}", p.quality_score),
                "success_rate": format!("{:.1}%", p.success_rate * 100.0),
                "avg_latency": format!("{}ms", p.avg_latency_ms),
                "cost": format!("¥{:.4}/1K", p.cost_per_1k_tokens),
                "online": p.online,
            })).collect::<Vec<_>>(),
        })
    }
}

impl Default for CollaborationEngine {
    fn default() -> Self { Self::new() }
}

// ─── 工具 ──────────────────────────────────────────────────────────

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_best_model_for_architecture() {
        let engine = CollaborationEngine::new();
        let best = engine.select_best_model("architecture", false);
        assert_eq!(best.unwrap(), "deepseek-v4-pro");
    }

    #[test]
    fn test_select_cheapest_model() {
        let engine = CollaborationEngine::new();
        let best = engine.select_best_model("code_generation", true);
        assert_eq!(best.unwrap(), "deepseek-v4-flash");
    }

    #[test]
    fn test_fallback_models() {
        let engine = CollaborationEngine::new();
        let fallbacks = engine.fallback_models("deepseek-v4-pro", "architecture");
        assert!(!fallbacks.is_empty());
        assert!(!fallbacks.contains(&"deepseek-v4-pro".to_string()));
    }

    #[test]
    fn test_record_execution_updates_profile() {
        let mut engine = CollaborationEngine::new();
        engine.record_execution("deepseek-v4-flash", "code_generation", true, 500, 90.0);
        engine.record_execution("deepseek-v4-flash", "code_generation", true, 600, 92.0);

        let profile = engine.model_profiles.get("deepseek-v4-flash").unwrap();
        assert_eq!(profile.total_executions, 2);
        assert_eq!(profile.total_successes, 2);
        assert!(profile.quality_score > 85.0);
    }

    #[test]
    fn test_decide_mode_critical() {
        let engine = CollaborationEngine::new();
        assert_eq!(engine.decide_mode("security", 0.9, true), CollaborationMode::Voting);
        assert_eq!(engine.decide_mode("general", 0.3, false), CollaborationMode::Single);
    }

    #[test]
    fn test_top_models() {
        let engine = CollaborationEngine::new();
        let top3 = engine.top_models("code_generation", 3);
        assert_eq!(top3.len(), 3);
    }

    #[test]
    fn test_model_ranking() {
        let engine = CollaborationEngine::new();
        let ranking = engine.get_model_ranking();
        assert!(ranking.len() >= 5);
        // deepseek-v4-pro should be top for quality
        assert_eq!(ranking[0].model_name, "deepseek-v4-pro");
    }
}

// ─── Tauri Commands ──────────────────────────────────────────────

#[tauri::command]
pub async fn collab_get_model_ranking(
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<serde_json::Value, String> {
    let engine = state.collaboration.lock().await;
    Ok(engine.stats())
}

#[tauri::command]
pub async fn collab_recommend_model(
    state: tauri::State<'_, crate::state::AppState>,
    task_type: String,
    prefer_cheap: Option<bool>,
) -> Result<serde_json::Value, String> {
    let engine = state.collaboration.lock().await;
    let (model, reason) = engine.recommend_model_with_reason(&task_type, prefer_cheap.unwrap_or(false));
    let best = engine.select_best_model(&task_type, prefer_cheap.unwrap_or(false));
    let fallbacks = engine.fallback_models(&model, &task_type);
    Ok(serde_json::json!({
        "recommended": model,
        "reason": reason,
        "best_by_quality": best,
        "fallbacks": fallbacks,
        "mode": format!("{:?}", engine.decide_mode(&task_type, 0.5, false)),
    }))
}

#[tauri::command]
pub async fn collab_record_execution(
    state: tauri::State<'_, crate::state::AppState>,
    model_name: String,
    task_type: String,
    success: bool,
    latency_ms: u64,
    quality_score: f64,
) -> Result<String, String> {
    let mut engine = state.collaboration.lock().await;
    engine.record_execution(&model_name, &task_type, success, latency_ms, quality_score);
    Ok(format!("Recorded execution for {}", model_name))
}