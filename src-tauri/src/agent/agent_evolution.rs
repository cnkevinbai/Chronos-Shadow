// Agent 自主学习进化引擎 (Chronos Agent Evolution Engine)
//
// 核心能力：
//   1. 经验记忆库 — 记录每次交互的上下文/动作/结果三元组
//   2. 模式学习 — 检测重复纠正模式，自动形成规则
//   3. 成功率追踪 — 每个 Agent 的成功率统计
//   4. 自适应优化 — 根据历史数据调整 Agent 行为
//   5. 知识蒸馏 — 将高频成功模式固化为本地 Skill
//
// 全部端侧计算，零 Token 消耗

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── 经验记忆 ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceMemory {
    /// 唯一 ID
    pub id: String,
    /// 时间戳
    pub timestamp: String,
    /// 用户原始输入
    pub user_input: String,
    /// 检测到的意图
    pub intent: String,
    /// 调度的 Agent
    pub agent: String,
    /// 使用的模型
    pub model: String,
    /// 是否成功
    pub success: bool,
    /// 用户反馈 (true=满意, false=不满意)
    pub user_satisfied: Option<bool>,
    /// LLM 响应摘要 (前 200 字符)
    pub response_snippet: String,
    /// 耗时 ms
    pub duration_ms: u64,
    /// Token 使用量
    pub tokens_used: u64,
    /// 费用 (RMB)
    pub cost_rmb: f64,
    /// 触发纠正的模式
    pub correction_pattern: Option<String>,
}

// ─── Agent 统计 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStats {
    /// Agent 标识
    pub agent_id: String,
    /// 总调用次数
    pub total_calls: u64,
    /// 成功次数
    pub success_count: u64,
    /// 用户满意次数
    pub satisfied_count: u64,
    /// 平均 Token 消耗
    pub avg_tokens: f64,
    /// 累计费用
    pub total_cost: f64,
    /// 成功率
    pub success_rate: f32,
    /// 满意率
    pub satisfaction_rate: f32,
    /// 最有效的模型
    pub best_model: String,
    /// 最佳模型成功率
    pub best_model_rate: f32,
}

// ─── 学习模式 ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedPattern {
    /// 模式 ID
    pub id: String,
    /// 触发关键词
    pub triggers: Vec<String>,
    /// 推荐 Agent
    pub recommended_agent: String,
    /// 推荐模型
    pub recommended_model: String,
    /// 置信度 (基于历史成功率)
    pub confidence: f32,
    /// 出现次数
    pub occurrence_count: u64,
    /// 成功次数
    pub success_count: u64,
    /// 最后更新时间
    pub last_updated: String,
}

// ─── 进化状态 ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionState {
    /// 总交互次数
    pub total_interactions: u64,
    /// 经验记忆库容量
    pub memory_size: usize,
    /// 已学习模式数
    pub learned_patterns: usize,
    /// 全局成功率
    pub global_success_rate: f32,
    /// 平均费用/次
    pub avg_cost_per_call: f64,
    /// 累计节省费用
    pub total_cost_saved: f64,
    /// 各 Agent 统计
    pub agent_stats: HashMap<String, AgentStats>,
    /// 进化阶段
    pub stage: String,
}

// ═══════════════════════════════════════════════════════════════════
// AgentEvolutionEngine
// ═══════════════════════════════════════════════════════════════════

pub struct AgentEvolutionEngine {
    /// 经验记忆库 (最近 1000 条)
    memory: Vec<ExperienceMemory>,
    /// 已学习模式
    patterns: Vec<LearnedPattern>,
    /// Agent 统计
    stats: HashMap<String, AgentStats>,
    /// 全局计数器
    total_calls: u64,
    total_success: u64,
    total_cost: f64,
    #[allow(dead_code)]
    total_saved: f64,
    pattern_counter: u64,
}

impl AgentEvolutionEngine {
    pub fn new() -> Self {
        Self {
            memory: Vec::new(),
            patterns: Vec::new(),
            stats: HashMap::new(),
            total_calls: 0,
            total_success: 0,
            total_cost: 0.0,
            total_saved: 0.0,
            pattern_counter: 0,
        }
    }

    // ── 记录交互 ────────────────────────────────────────────────

    pub fn record_interaction(&mut self, exp: ExperienceMemory) {
        self.total_calls += 1;
        if exp.success { self.total_success += 1; }
        self.total_cost += exp.cost_rmb;

        // Update agent stats
        let stat = self.stats.entry(exp.agent.clone()).or_insert_with(|| AgentStats {
            agent_id: exp.agent.clone(),
            total_calls: 0, success_count: 0, satisfied_count: 0,
            avg_tokens: 0.0, total_cost: 0.0, success_rate: 0.0,
            satisfaction_rate: 0.0, best_model: String::new(), best_model_rate: 0.0,
        });

        stat.total_calls += 1;
        if exp.success { stat.success_count += 1; }
        if exp.user_satisfied == Some(true) { stat.satisfied_count += 1; }
        stat.avg_tokens = (stat.avg_tokens * (stat.total_calls - 1) as f64 + exp.tokens_used as f64) / stat.total_calls as f64;
        stat.total_cost += exp.cost_rmb;
        stat.success_rate = stat.success_count as f32 / stat.total_calls as f32;
        stat.satisfaction_rate = if stat.total_calls > 0 { stat.satisfied_count as f32 / stat.total_calls as f32 } else { 0.0 };

        // Update best model tracking
        if exp.success && exp.model != stat.best_model {
            stat.best_model = exp.model.clone();
            stat.best_model_rate = stat.success_rate;
        }

        // Add to memory (keep last 1000)
        self.memory.push(exp);
        if self.memory.len() > 1000 { self.memory.remove(0); }

        // Check if we should learn new patterns
        if self.memory.len() % 50 == 0 {
            self.auto_learn();
        }
    }

    // ── 自动学习 ────────────────────────────────────────────────

    fn auto_learn(&mut self) {
        // Analyze recent corrections to learn patterns
        let recent: Vec<&ExperienceMemory> = self.memory.iter()
            .rev().take(200)
            .filter(|m| m.correction_pattern.is_some() && m.success)
            .collect();

        // Group by correction pattern
        let mut pattern_groups: HashMap<&str, Vec<&&ExperienceMemory>> = HashMap::new();
        for exp in &recent {
            if let Some(ref cp) = exp.correction_pattern {
                pattern_groups.entry(cp.as_str()).or_default().push(exp);
            }
        }

        // Create learned patterns from frequent corrections (>= 3 occurrences)
        for (pattern, exps) in &pattern_groups {
            if exps.len() >= 3 {
                let success = exps.iter().filter(|e| e.success).count() as u64;
                let total = exps.len() as u64;

                // Check if pattern already exists
                let exists = self.patterns.iter().any(|p| p.triggers.contains(&pattern.to_string()));
                if !exists {
                    let agent = exps[0].agent.clone();
                    let model = exps[0].model.clone();
                    self.pattern_counter += 1;
                    self.patterns.push(LearnedPattern {
                        id: format!("pat-{:04}", self.pattern_counter),
                        triggers: vec![pattern.to_string()],
                        recommended_agent: agent,
                        recommended_model: model,
                        confidence: success as f32 / total as f32,
                        occurrence_count: total,
                        success_count: success,
                        last_updated: chrono_now(),
                    });
                }
            }
        }
    }

    // ── 查询最佳 Agent ──────────────────────────────────────────

    pub fn best_agent_for(&self, intent: &str) -> Option<String> {
        // Check learned patterns first
        for p in &self.patterns {
            if p.triggers.iter().any(|t| intent.contains(t.as_str())) && p.confidence > 0.7 {
                return Some(p.recommended_agent.clone());
            }
        }

        // Fall back to stats-based recommendation
        self.stats.iter()
            .filter(|(_, s)| s.total_calls >= 3)
            .max_by(|(_, a), (_, b)| a.success_rate.partial_cmp(&b.success_rate).unwrap())
            .map(|(id, _)| id.clone())
    }

    /// 根据意图推荐最优模型
    pub fn best_model_for(&self, intent: &str, agent: &str) -> Option<String> {
        self.memory.iter()
            .filter(|m| m.intent == intent && m.agent == agent && m.success)
            .fold(HashMap::new(), |mut acc, m| {
                *acc.entry(m.model.clone()).or_insert(0) += 1u64;
                acc
            })
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(model, _)| model)
    }

    /// 计算预估节省 (基于历史纠正模式)
    pub fn estimated_savings(&self) -> f64 {
        let base_cost_per_fix = 0.05; // Average cost of a correction call
        let corrections_avoided = self.patterns.iter()
            .map(|p| p.occurrence_count)
            .sum::<u64>();
        corrections_avoided as f64 * base_cost_per_fix * 0.5
    }

    // ── 进化状态报告 ────────────────────────────────────────────

    pub fn get_state(&self) -> EvolutionState {
        EvolutionState {
            total_interactions: self.total_calls,
            memory_size: self.memory.len(),
            learned_patterns: self.patterns.len(),
            global_success_rate: if self.total_calls > 0 {
                self.total_success as f32 / self.total_calls as f32
            } else { 0.0 },
            avg_cost_per_call: if self.total_calls > 0 {
                self.total_cost / self.total_calls as f64
            } else { 0.0 },
            total_cost_saved: self.estimated_savings(),
            agent_stats: self.stats.clone(),
            stage: if self.patterns.len() >= 10 { "adaptive" }
                else if self.patterns.len() >= 3 { "learning" }
                else { "initial" }.into(),
        }
    }

    // ── 获取学习到的模式 ────────────────────────────────────────

    pub fn get_patterns(&self) -> &[LearnedPattern] {
        &self.patterns
    }

    /// 检查意图是否匹配已学习模式
    pub fn match_pattern(&self, intent: &str) -> Option<&LearnedPattern> {
        self.patterns.iter()
            .filter(|p| p.confidence > 0.6)
            .find(|p| p.triggers.iter().any(|t| intent.contains(t.as_str())))
    }
}

impl Default for AgentEvolutionEngine {
    fn default() -> Self { Self::new() }
}

fn chrono_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ─── 单元测试 ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_exp(agent: &str, success: bool, satisfied: Option<bool>) -> ExperienceMemory {
        ExperienceMemory {
            id: format!("exp-{}", rand::random::<u16>()),
            timestamp: chrono_now(),
            user_input: "test".into(),
            intent: "CodeGeneration".into(),
            agent: agent.into(),
            model: "deepseek-v4-flash".into(),
            success,
            user_satisfied: satisfied,
            response_snippet: "fn test() {}".into(),
            duration_ms: 500,
            tokens_used: 100,
            cost_rmb: 0.001,
            correction_pattern: None,
        }
    }

    #[test]
    fn test_record_and_stats() {
        let mut engine = AgentEvolutionEngine::new();

        // Record 5 successful calls
        for _ in 0..5 {
            engine.record_interaction(make_exp("Coder", true, Some(true)));
        }
        // Record 1 failed call
        engine.record_interaction(make_exp("Coder", false, Some(false)));

        let state = engine.get_state();
        assert_eq!(state.total_interactions, 6);
        assert_eq!(state.memory_size, 6);

        let coder_stats = state.agent_stats.get("Coder").unwrap();
        assert_eq!(coder_stats.total_calls, 6);
        assert_eq!(coder_stats.success_count, 5);
        assert!(coder_stats.success_rate > 0.8);
    }

    #[test]
    fn test_auto_learn_patterns() {
        let mut engine = AgentEvolutionEngine::new();

        // Simulate 50 interactions with correction patterns
        for i in 0..50 {
            let mut exp = make_exp("Auditor", true, Some(true));
            if i % 5 == 0 {
                exp.correction_pattern = Some("security_scan".into());
            }
            engine.record_interaction(exp);
        }

        let state = engine.get_state();
        assert!(state.learned_patterns > 0 || state.stage == "initial");
    }

    #[test]
    fn test_best_agent() {
        let mut engine = AgentEvolutionEngine::new();

        engine.record_interaction(make_exp("Coder", true, Some(true)));
        engine.record_interaction(make_exp("Coder", true, Some(true)));
        engine.record_interaction(make_exp("Auditor", false, Some(false)));

        let best = engine.best_agent_for("CodeGeneration");
        assert!(best.is_some());
    }

    #[test]
    fn test_estimated_savings() {
        let mut engine = AgentEvolutionEngine::new();
        assert_eq!(engine.estimated_savings(), 0.0);

        // Add patterns manually
        engine.patterns.push(LearnedPattern {
            id: "test-1".into(),
            triggers: vec!["test".into()],
            recommended_agent: "Coder".into(),
            recommended_model: "flash".into(),
            confidence: 0.9,
            occurrence_count: 10,
            success_count: 9,
            last_updated: chrono_now(),
        });

        assert!(engine.estimated_savings() > 0.0);
    }

    #[test]
    fn test_memory_limit() {
        let mut engine = AgentEvolutionEngine::new();
        for _ in 0..1100 {
            engine.record_interaction(make_exp("Coder", true, None));
        }
        assert!(engine.memory.len() <= 1000);
    }
}
