// Agent 专业能力质量评分与自我进化增强引擎 (Agent Quality & Rigor Engine)
//
// 核心功能：
//   1. 按 Agent 角色追踪幻觉率、成功率、改进趋势
//   2. 防幻觉检测结果自动桥接入进化引擎
//   3. Agent 专项学习 — 按角色标记经验 + 跨角色共享
//   4. 严谨度评分 (Rigor Score) — 0-100 综合质量评分
//
// 全部端侧计算，0 Token 消耗

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Agent 质量评分 ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentQualityScore {
    /// Agent 角色标识
    pub agent_role: String,
    /// 严谨度综合评分 0-100
    pub rigor_score: u32,
    /// 总任务数
    pub total_tasks: u32,
    /// 成功完成数
    pub successful_tasks: u32,
    /// 触发幻觉检测次数
    pub hallucination_events: u32,
    /// 假编程检测次数
    pub fake_programming_count: u32,
    /// 假完成检测次数
    pub fake_completion_count: u32,
    /// 编造谎言检测次数
    pub fabricated_facts_count: u32,
    /// 最近 10 次任务的成功率
    pub recent_success_rate: f32,
    /// 改进趋势: positive=变好, negative=变差, stable=持平
    pub improvement_trend: String,
    /// 上次评分时间
    pub last_evaluated: String,
    /// 累计节省 Token (通过经验复用)
    pub tokens_saved_by_learning: u64,
}

impl AgentQualityScore {
    pub fn new(role: &str) -> Self {
        Self {
            agent_role: role.into(),
            rigor_score: 85, // 初始默认 85 分
            total_tasks: 0,
            successful_tasks: 0,
            hallucination_events: 0,
            fake_programming_count: 0,
            fake_completion_count: 0,
            fabricated_facts_count: 0,
            recent_success_rate: 1.0,
            improvement_trend: "stable".into(),
            last_evaluated: chrono::Utc::now().to_rfc3339(),
            tokens_saved_by_learning: 0,
        }
    }

    /// 记录一次任务结果
    pub fn record_task(&mut self, success: bool, hallucination_categories: &[String]) {
        self.total_tasks += 1;
        if success { self.successful_tasks += 1; }

        for cat in hallucination_categories {
            self.hallucination_events += 1;
            match cat.as_str() {
                "假编程" => self.fake_programming_count += 1,
                "假完成" => self.fake_completion_count += 1,
                "编造谎言" => self.fabricated_facts_count += 1,
                _ => {}
            }
        }

        self.recompute();
    }

    /// 重新计算严谨度评分
    fn recompute(&mut self) {
        let success_rate = if self.total_tasks > 0 {
            self.successful_tasks as f32 / self.total_tasks as f32
        } else { 1.0 };

        // 幻觉惩罚: 每次幻觉扣 2-5 分
        let hallucination_penalty = (self.hallucination_events as u32 * 3).min(40);

        // 假编程额外惩罚
        let fake_code_penalty = (self.fake_programming_count as u32 * 5).min(25);

        // 假完成额外惩罚
        let fake_done_penalty = (self.fake_completion_count as u32 * 4).min(20);

        // 编造谎言严重惩罚
        let lie_penalty = (self.fabricated_facts_count as u32 * 5).min(25);

        self.rigor_score = 100u32
            .saturating_sub(hallucination_penalty)
            .saturating_sub(fake_code_penalty)
            .saturating_sub(fake_done_penalty)
            .saturating_sub(lie_penalty)
            .max(0);

        self.recent_success_rate = success_rate;

        // 趋势判断
        if success_rate > 0.85 && self.hallucination_events == 0 {
            self.improvement_trend = "positive".into();
        } else if self.hallucination_events > 5 {
            self.improvement_trend = "negative".into();
        } else {
            self.improvement_trend = "stable".into();
        }

        self.last_evaluated = chrono::Utc::now().to_rfc3339();
    }

    /// 记录经验复用节省
    pub fn record_learning_save(&mut self, tokens: u64) {
        self.tokens_saved_by_learning += tokens;
    }
}

// ─── 进化经验桥接 (Hallucination → Evolution) ────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionBridgeEntry {
    /// 关联的幻觉发现类别
    pub hallucination_category: String,
    /// Agent 角色
    pub agent_role: String,
    /// 原始错误描述
    pub error_pattern: String,
    /// 建议修正
    pub correction: String,
    /// 严重度
    pub severity: String,
    /// 发生次数
    pub occurrence_count: u32,
    /// 最近发生时间
    pub last_occurred: String,
    /// 是否已学习
    pub learned: bool,
}

/// 跨 Agent 经验共享
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossAgentInsight {
    /// 经验 ID
    pub id: String,
    /// 来源 Agent
    pub source_agent: String,
    /// 目标 Agent (可受益的 Agent)
    pub target_agents: Vec<String>,
    /// 经验描述
    pub insight: String,
    /// 适用场景
    pub context: String,
    /// 置信度
    pub confidence: f32,
}

// ─── Agent 质量引擎 ────────────────────────────────────────────────

pub struct AgentQualityEngine {
    /// 各 Agent 的质量评分
    pub scores: HashMap<String, AgentQualityScore>,
    /// 幻觉→进化桥接记录
    pub bridge_entries: Vec<EvolutionBridgeEntry>,
    /// 跨 Agent 经验共享
    pub cross_agent_insights: Vec<CrossAgentInsight>,
    /// 全局严谨度阈值: 低于此分的 Agent 自动降级
    pub rigor_threshold: u32,
}

impl AgentQualityEngine {
    pub fn new() -> Self {
        let mut scores = HashMap::new();
        // 初始化所有 SDLC Agent 的评分
        for role in &["PM", "UIDesigner", "Architect", "Planner", "Coder", "Auditor", "Verifier"] {
            scores.insert(role.to_string(), AgentQualityScore::new(role));
        }
        Self {
            scores,
            bridge_entries: Vec::new(),
            cross_agent_insights: Vec::new(),
            rigor_threshold: 50, // 低于 50 分触发降级
        }
    }

    /// 记录 Agent 任务结果（含幻觉检测反馈）
    pub fn record_agent_task(
        &mut self,
        agent_role: &str,
        success: bool,
        hallucination_categories: &[String],
    ) {
        let score = self.scores.entry(agent_role.into())
            .or_insert_with(|| AgentQualityScore::new(agent_role));
        score.record_task(success, hallucination_categories);
    }

    /// 桥接：将幻觉发现自动输入进化引擎
    pub fn bridge_hallucination_to_evolution(
        &mut self,
        agent_role: &str,
        category: &str,
        error_pattern: &str,
        correction: &str,
        severity: &str,
    ) -> Option<EvolutionBridgeEntry> {
        // 查找是否已有相同模式
        let existing = self.bridge_entries.iter_mut()
            .find(|e| e.hallucination_category == category && e.error_pattern == error_pattern);

        if let Some(entry) = existing {
            entry.occurrence_count += 1;
            entry.last_occurred = chrono::Utc::now().to_rfc3339();
            // 重复出现 3 次以上 → 标记为需要学习
            if entry.occurrence_count >= 3 && !entry.learned {
                entry.learned = true;
                return Some(entry.clone());
            }
            None
        } else {
            let entry = EvolutionBridgeEntry {
                hallucination_category: category.into(),
                agent_role: agent_role.into(),
                error_pattern: error_pattern.into(),
                correction: correction.into(),
                severity: severity.into(),
                occurrence_count: 1,
                last_occurred: chrono::Utc::now().to_rfc3339(),
                learned: false,
            };
            self.bridge_entries.push(entry.clone());
            None // 首次出现，仅记录不触发学习
        }
    }

    /// 获取某 Agent 的严谨度评分
    pub fn get_score(&self, agent_role: &str) -> Option<&AgentQualityScore> {
        self.scores.get(agent_role)
    }

    /// 获取所有 Agent 评分（按严谨度降序）
    pub fn get_all_scores(&self) -> Vec<&AgentQualityScore> {
        let mut scores: Vec<&AgentQualityScore> = self.scores.values().collect();
        scores.sort_by(|a, b| b.rigor_score.cmp(&a.rigor_score));
        scores
    }

    /// 检查 Agent 是否需要降级
    pub fn should_downgrade(&self, agent_role: &str) -> bool {
        self.scores.get(agent_role)
            .map(|s| s.rigor_score < self.rigor_threshold)
            .unwrap_or(false)
    }

    /// 添加跨 Agent 共享经验
    pub fn share_insight(
        &mut self,
        source_agent: &str,
        target_agents: Vec<String>,
        insight: &str,
        context: &str,
        confidence: f32,
    ) {
        let id = format!("cai-{:04}", self.cross_agent_insights.len() + 1);
        self.cross_agent_insights.push(CrossAgentInsight {
            id,
            source_agent: source_agent.into(),
            target_agents,
            insight: insight.into(),
            context: context.into(),
            confidence,
        });
    }

    /// 获取某 Agent 可受益的跨 Agent 经验
    pub fn get_insights_for(&self, agent_role: &str) -> Vec<&CrossAgentInsight> {
        self.cross_agent_insights.iter()
            .filter(|i| i.target_agents.contains(&agent_role.to_string()))
            .collect()
    }

    /// 全局质量报告
    pub fn global_quality_report(&self) -> serde_json::Value {
        let avg_rigor = if self.scores.is_empty() { 0.0 } else {
            self.scores.values().map(|s| s.rigor_score as f64).sum::<f64>() / self.scores.len() as f64
        };

        let total_hallucinations: u32 = self.scores.values().map(|s| s.hallucination_events).sum();
        let total_tasks: u32 = self.scores.values().map(|s| s.total_tasks).sum();
        let total_learned = self.bridge_entries.iter().filter(|e| e.learned).count() as u32;

        serde_json::json!({
            "average_rigor": avg_rigor as u32,
            "total_tasks": total_tasks,
            "total_hallucinations": total_hallucinations,
            "hallucination_rate": if total_tasks > 0 {
                format!("{:.1}%", total_hallucinations as f64 / total_tasks as f64 * 100.0)
            } else { "0%".to_string() },
            "learned_patterns": total_learned,
            "cross_agent_insights": self.cross_agent_insights.len() as u32,
            "agents_below_threshold": self.scores.values()
                .filter(|s| s.rigor_score < self.rigor_threshold).count() as u32,
            "top_performer": self.get_all_scores().first()
                .map(|s| (s.agent_role.clone(), s.rigor_score)),
            "needs_improvement": self.get_all_scores().last()
                .filter(|s| s.rigor_score < 70)
                .map(|s| (s.agent_role.clone(), s.rigor_score)),
        })
    }
}

impl Default for AgentQualityEngine {
    fn default() -> Self { Self::new() }
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_score() {
        let score = AgentQualityScore::new("Coder");
        assert_eq!(score.rigor_score, 85);
        assert_eq!(score.total_tasks, 0);
    }

    #[test]
    fn test_successful_task_improves() {
        let mut score = AgentQualityScore::new("Coder");
        score.record_task(true, &[]);
        assert_eq!(score.total_tasks, 1);
        assert_eq!(score.successful_tasks, 1);
        assert_eq!(score.rigor_score, 100); // 无幻觉，满分
    }

    #[test]
    fn test_hallucination_penalty() {
        let mut score = AgentQualityScore::new("Coder");
        score.record_task(false, &["假编程".into(), "假完成".into()]);
        // 假编程 -5, 假完成 -4, 幻觉事件 -3*2=-6, 共 -15
        assert!(score.rigor_score <= 85);
        assert_eq!(score.fake_programming_count, 1);
        assert_eq!(score.fake_completion_count, 1);
    }

    #[test]
    fn test_repeated_hallucination_causes_downgrade() {
        let mut engine = AgentQualityEngine::new();
        for _ in 0..10 {
            engine.record_agent_task("Coder", false, &["编造谎言".into()]);
        }
        let score = engine.get_score("Coder").unwrap();
        assert!(score.rigor_score < 40);
        assert!(engine.should_downgrade("Coder"));
    }

    #[test]
    fn test_bridge_repeated_pattern_learns() {
        let mut engine = AgentQualityEngine::new();
        // 首次出现 → 不触发学习
        assert!(engine.bridge_hallucination_to_evolution("Coder", "假编程", "TODO占位", "提供完整代码", "high").is_none());
        // 第二次
        assert!(engine.bridge_hallucination_to_evolution("Coder", "假编程", "TODO占位", "提供完整代码", "high").is_none());
        // 第三次 → 触发学习
        assert!(engine.bridge_hallucination_to_evolution("Coder", "假编程", "TODO占位", "提供完整代码", "high").is_some());
    }

    #[test]
    fn test_cross_agent_insight() {
        let mut engine = AgentQualityEngine::new();
        engine.share_insight("Coder", vec!["Auditor".into()], "避免使用unwrap()", "Rust错误处理", 0.9);
        let insights = engine.get_insights_for("Auditor");
        assert_eq!(insights.len(), 1);
        assert_eq!(insights[0].insight, "避免使用unwrap()");
    }
}

// ─── Tauri Commands ──────────────────────────────────────────────

#[tauri::command]
pub fn get_agent_quality_scores(
    state: tauri::State<crate::state::AppState>,
) -> Vec<AgentQualityScore> {
    let engine = state.agent_quality.lock().unwrap();
    engine.get_all_scores().into_iter().cloned().collect()
}

#[tauri::command]
pub async fn record_agent_task_quality(
    state: tauri::State<'_, crate::state::AppState>,
    agent_role: String,
    success: bool,
    hallucination_categories: Vec<String>,
) -> Result<String, String> {
    // 第一阶段：同步更新 Agent 质量评分
    let bridge_entries: Vec<_> = {
        let mut engine = state.agent_quality.lock().unwrap();
        engine.record_agent_task(&agent_role, success, &hallucination_categories);

        hallucination_categories.iter().filter_map(|cat| {
            engine.bridge_hallucination_to_evolution(
                &agent_role, cat, &format!("{} by {}", cat, agent_role),
                "请查阅防幻觉报告获取修正建议", "medium",
            )
        }).collect()
    };

    // 第二阶段：异步写入 EvolutionEngine
    for entry in bridge_entries {
        let evo = state.evolution.lock().await;
        let delta = crate::agent::evolving::consolidator::EvoDelta {
            experience_id: format!("hbridge-{}", chrono::Utc::now().timestamp()),
            context_trigger_hash: entry.error_pattern.clone(),
            failed_llm_action: entry.error_pattern,
            correct_human_action: entry.correction,
            token_sunk_cost_saved: 50,
            accuracy_weight: 0.7,
        };
        let _ = evo.local_consolidator.validate_and_commit_experience(delta).await;
    }

    let engine = state.agent_quality.lock().unwrap();
    let score = engine.get_score(&agent_role)
        .map(|s| s.rigor_score).unwrap_or(85);
    Ok(format!("Agent '{}' rigor score: {}/100", agent_role, score))
}

#[tauri::command]
pub fn get_global_quality_report(
    state: tauri::State<crate::state::AppState>,
) -> serde_json::Value {
    state.agent_quality.lock().unwrap().global_quality_report()
}
