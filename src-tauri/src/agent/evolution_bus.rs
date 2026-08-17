// 端侧计算引擎统一进化总线 (Evolution Bus)
//
// 连接所有端侧引擎的自适应反馈环路，实现：
//   1. 统一反馈收集 — 所有引擎的反馈汇聚到总线
//   2. 跨引擎知识共享 — 引擎A的学习成果可迁移到引擎B
//   3. 自动调参编排 — 按周期评估各引擎性能并自动调优
//   4. 进化状态持久化 — 引擎参数可落盘/恢复
//   5. 先进性度量 — 定期评估各引擎的先进性指标
//
// 设计原则：
//   1. 去中心化 — 每个引擎独立进化，总线仅负责协调
//   2. 增量改进 — 小步快跑，避免大幅度震荡
//   3. 安全保守 — 进化方向偏向安全一侧 (宁可保守不可激进)
//   4. 可观测 — 所有进化决策透明可追溯

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// ─── 引擎注册表 ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EngineId {
    Distillation,
    Scheduling,
    HallucinationGuard,
    CacheEngine,
    AgentQuality,
    Collaboration,
    TaskIntelligence,
    PredictiveAnalytics,
    LocalAnalytics,
}

impl EngineId {
    pub fn label(&self) -> &str {
        match self {
            Self::Distillation => "蒸馏引擎",
            Self::Scheduling => "调度引擎",
            Self::HallucinationGuard => "防幻觉引擎",
            Self::CacheEngine => "缓存引擎",
            Self::AgentQuality => "Agent质量",
            Self::Collaboration => "协作引擎",
            Self::TaskIntelligence => "任务智能",
            Self::PredictiveAnalytics => "预测分析",
            Self::LocalAnalytics => "本地分析",
        }
    }
}

// ─── 进化事件 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionEvent {
    pub id: String,
    pub timestamp: String,
    pub engine: EngineId,
    pub event_type: EvolutionEventType,
    pub metric_name: String,
    pub old_value: f64,
    pub new_value: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvolutionEventType {
    ParameterTuned,     // 参数被自动调整
    ThresholdAdjusted,  // 阈值被调整
    StrategyUpdated,    // 策略被更新
    WeightEvolved,      // 权重进化
    DegradationAlert,   // 性能退化告警
    ImprovementDetected, // 改进被检测到
}

// ─── 引擎健康指标 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineHealth {
    pub engine: EngineId,
    /// 综合先进性得分 (0-100)
    pub advancement_score: f64,
    /// 稳定性指标 (0-1)
    pub stability: f64,
    /// 进化次数
    pub evolution_count: u64,
    /// 最近进化时间
    pub last_evolution: Option<String>,
    /// 累计改进幅度
    pub cumulative_improvement: f64,
    /// 是否退化
    pub is_degrading: bool,
    /// 建议
    pub recommendation: Option<String>,
}

/// 系统健康综合评估（v2：系统化多维健康评分）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemAssessment {
    pub overall_health: f64,
    pub grade: String,
    pub engines_total: usize,
    pub degrading_engines: Vec<String>,
    pub top_engines: Vec<String>,
    pub recommendations: Vec<String>,
}

// ─── 进化总线 ──────────────────────────────────────────────────────

pub struct EvolutionBus {
    /// 引擎健康状态
    pub engine_health: HashMap<EngineId, EngineHealth>,
    /// 进化事件日志 (最近1000条)
    pub event_log: Vec<EvolutionEvent>,
    /// 全局进化计数器
    evolution_counter: u64,
    /// 是否启用自动进化
    pub auto_evolution_enabled: bool,
    /// 进化周期 (秒)
    pub evolution_cycle_secs: u64,
    /// 上次进化时间
    last_evolution_cycle: u64,
    /// 最大单次调整幅度 (防止震荡)
    pub max_adjustment_per_cycle: f64,
}

impl EvolutionBus {
    pub fn new() -> Self {
        let mut health = HashMap::new();
        for engine in &[
            EngineId::Distillation, EngineId::Scheduling, EngineId::HallucinationGuard,
            EngineId::CacheEngine, EngineId::AgentQuality, EngineId::Collaboration,
            EngineId::TaskIntelligence, EngineId::PredictiveAnalytics, EngineId::LocalAnalytics,
        ] {
            health.insert(engine.clone(), EngineHealth {
                engine: engine.clone(),
                advancement_score: 75.0,
                stability: 0.9,
                evolution_count: 0,
                last_evolution: None,
                cumulative_improvement: 0.0,
                is_degrading: false,
                recommendation: None,
            });
        }

        Self {
            engine_health: health,
            event_log: Vec::new(),
            evolution_counter: 0,
            auto_evolution_enabled: true,
            evolution_cycle_secs: 3600,
            last_evolution_cycle: now_secs(),
            max_adjustment_per_cycle: 0.15,
        }
    }

    // ── 进化事件记录 ──────────────────────────────────────────

    /// 记录一次进化事件
    pub fn record_evolution(
        &mut self,
        engine: EngineId,
        event_type: EvolutionEventType,
        metric: &str,
        old_val: f64,
        new_val: f64,
        reason: &str,
    ) {
        self.evolution_counter += 1;
        let event = EvolutionEvent {
            id: format!("evo-{:06}", self.evolution_counter),
            timestamp: now_iso(),
            engine: engine.clone(),
            event_type: event_type.clone(),
            metric_name: metric.into(),
            old_value: old_val,
            new_value: new_val,
            reason: reason.into(),
        };

        // 更新引擎健康
        if let Some(health) = self.engine_health.get_mut(&engine) {
            health.evolution_count += 1;
            health.last_evolution = Some(event.timestamp.clone());
            let delta = (new_val - old_val).abs();
            health.cumulative_improvement += delta * if new_val > old_val { 1.0 } else { -0.5 };
            health.is_degrading = new_val < old_val && delta > 0.1;
        }

        tracing::info!(
            "[EvoBus] {} | {} {}: {:.3} → {:.3} | {}",
            engine.label(), event_type_label(&event_type), metric, old_val, new_val, reason
        );

        self.event_log.push(event);
        while self.event_log.len() > 1000 {
            self.event_log.remove(0);
        }
    }

    // ── 性能反馈 ──────────────────────────────────────────────

    /// 接收引擎性能反馈并触发自适应调整
    pub fn feedback_performance(
        &mut self,
        engine: EngineId,
        metric: &str,
        current_value: f64,
        target_value: f64,
        direction_is_higher_better: bool,
    ) -> Option<f64> {
        if !self.auto_evolution_enabled { return None; }

        let error = if direction_is_higher_better {
            target_value - current_value
        } else {
            current_value - target_value
        };

        // 只在误差显著时调整
        if error.abs() < 0.05 { return None; }

        let adjustment = (error * 0.1).clamp(-self.max_adjustment_per_cycle, self.max_adjustment_per_cycle);
        let new_value = (current_value + adjustment).clamp(0.1, 0.99);

        // 安全检查：避免退化方向过大
        if new_value < current_value * 0.7 {
            tracing::warn!("[EvoBus] SAFETY: {} {} adjustment too aggressive, clamping", engine.label(), metric);
            return Some(current_value * 0.85); // 保守回落
        }

        self.record_evolution(
            engine,
            EvolutionEventType::ParameterTuned,
            metric,
            current_value,
            new_value,
            &format!("Feedback: error={:.3} direction={}", error, if direction_is_higher_better { "↑" } else { "↓" }),
        );

        Some(new_value)
    }

    // ── 跨引擎知识迁移 ────────────────────────────────────────

    /// 将一个引擎的成功参数迁移到相似引擎
    pub fn transfer_knowledge(
        &mut self,
        source: EngineId,
        target: EngineId,
        metric: &str,
        source_value: f64,
        target_value: &mut f64,
        similarity: f64,
    ) -> bool {
        if similarity < 0.5 { return false; }

        let blended = *target_value * (1.0 - similarity * 0.3) + source_value * similarity * 0.3;
        let delta = (blended - *target_value).abs();

        if delta > 0.01 {
            self.record_evolution(
                target,
                EvolutionEventType::StrategyUpdated,
                metric,
                *target_value,
                blended,
                &format!("Knowledge transfer from {} (similarity={:.1}%)", source.label(), similarity * 100.0),
            );
            *target_value = blended;
            return true;
        }
        false
    }

    // ── 进化周期检查 ──────────────────────────────────────────

    /// 检查是否应该执行定期进化
    pub fn should_evolve(&mut self) -> bool {
        let now = now_secs();
        if now - self.last_evolution_cycle >= self.evolution_cycle_secs {
            self.last_evolution_cycle = now;
            true
        } else {
            false
        }
    }

    // ── 先进性评估 ────────────────────────────────────────────

    /// 评估所有引擎的先进性并更新健康评分
    pub fn assess_advancement(&mut self, engine_metrics: &[(EngineId, f64, f64)]) {
        for (engine, performance, stability) in engine_metrics {
            if let Some(health) = self.engine_health.get_mut(engine) {
                // 先进性 = 性能 × 稳定性 × (1 + 累积改进)
                let improvement_bonus = (health.cumulative_improvement * 10.0).min(20.0);
                health.advancement_score = (performance * 60.0 + stability * 40.0 + improvement_bonus).min(100.0);
                health.stability = *stability;

                // 生成建议
                health.recommendation = if health.advancement_score < 50.0 {
                    Some(format!("⚠️ {} 先进性不足({:.0})，建议重点优化", engine.label(), health.advancement_score))
                } else if health.is_degrading {
                    Some(format!("📉 {} 正在退化，建议检查最近参数变更", engine.label()))
                } else if health.advancement_score > 90.0 {
                    Some(format!("🌟 {} 处于领先水平({:.0})，可作为其他引擎的参考", engine.label(), health.advancement_score))
                } else {
                    None
                };
            }
        }
    }

    // ── 综合健康报告 ──────────────────────────────────────────

    pub fn health_report(&self) -> serde_json::Value {
        let engines: Vec<_> = self.engine_health.values().map(|h| {
            serde_json::json!({
                "engine": h.engine.label(),
                "advancement_score": format!("{:.0}", h.advancement_score),
                "stability": format!("{:.1}%", h.stability * 100.0),
                "evolution_count": h.evolution_count,
                "cumulative_improvement": format!("{:.3}", h.cumulative_improvement),
                "is_degrading": h.is_degrading,
                "recommendation": h.recommendation,
            })
        }).collect();

        let avg_advancement: f64 = self.engine_health.values()
            .map(|h| h.advancement_score).sum::<f64>() / self.engine_health.len().max(1) as f64;

        serde_json::json!({
            "auto_evolution_enabled": self.auto_evolution_enabled,
            "total_evolutions": self.evolution_counter,
            "engines_tracked": self.engine_health.len(),
            "average_advancement": format!("{:.0}", avg_advancement),
            "cycle_secs": self.evolution_cycle_secs,
            "engines": engines,
        })
    }

    /// v2: 系统化健康自评估 — 综合所有引擎的先进性/稳定性/退化状态
    pub fn self_assess(&self) -> SystemAssessment {
        let engines: Vec<&EngineHealth> = self.engine_health.values().collect();
        let total = engines.len();
        if total == 0 {
            return SystemAssessment {
                overall_health: 0.0,
                grade: "F".into(),
                engines_total: 0,
                degrading_engines: vec![],
                top_engines: vec![],
                recommendations: vec![],
            };
        }

        let avg_advancement = engines.iter().map(|e| e.advancement_score).sum::<f64>() / total as f64;
        let avg_stability = engines.iter().map(|e| e.stability).sum::<f64>() / total as f64;
        let degrading_count = engines.iter().filter(|e| e.is_degrading).count();

        // 综合评分：先进性 70% + 稳定性 30%
        let overall = avg_advancement * 0.7 + avg_stability * 100.0 * 0.3;
        let grade = if overall >= 85.0 {
            "A"
        } else if overall >= 70.0 {
            "B"
        } else if overall >= 55.0 {
            "C"
        } else if overall >= 40.0 {
            "D"
        } else {
            "F"
        };

        let mut degrading: Vec<String> = engines
            .iter()
            .filter(|e| e.is_degrading)
            .map(|e| e.engine.label().to_string())
            .collect();
        degrading.sort();

        let mut sorted = engines.clone();
        sorted.sort_by(|a, b| b.advancement_score.partial_cmp(&a.advancement_score).unwrap());
        let top_engines: Vec<String> = sorted.iter().take(3).map(|e| e.engine.label().to_string()).collect();

        let mut recommendations = Vec::new();
        if degrading_count > 0 {
            recommendations.push(format!("{} 个引擎退化，建议触发自愈或回滚", degrading_count));
        }
        if avg_stability < 0.7 {
            recommendations.push("整体稳定性偏低，建议增大 max_adjustment_per_cycle 保护".into());
        }
        if self.auto_evolution_enabled && avg_advancement < 60.0 {
            recommendations.push("自动进化已启用但先进性偏低，建议审查进化策略".into());
        }

        SystemAssessment {
            overall_health: overall,
            grade: grade.to_string(),
            engines_total: total,
            degrading_engines: degrading,
            top_engines,
            recommendations,
        }
    }

    // ── 持久化 ────────────────────────────────────────────────

    pub fn save_state(&self, dir: &std::path::Path) -> Result<String, String> {
        let path = dir.join("evolution_bus.json");
        let state = serde_json::json!({
            "engine_health": self.engine_health.iter().map(|(k, v)| {
                (format!("{:?}", k), serde_json::to_value(v).unwrap_or_default())
            }).collect::<HashMap<_, _>>(),
            "evolution_counter": self.evolution_counter,
            "auto_evolution_enabled": self.auto_evolution_enabled,
        });
        std::fs::write(&path, serde_json::to_string_pretty(&state).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        Ok(format!("EvolutionBus state saved to {:?}", path))
    }

    pub fn load_state(&mut self, dir: &std::path::Path) -> Result<String, String> {
        let path = dir.join("evolution_bus.json");
        if !path.exists() { return Ok("No saved state".into()); }
        let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let state: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        if let Some(h) = state.get("engine_health") {
            if let Some(obj) = h.as_object() {
                for (k, v) in obj {
                    if let Ok(eid) = parse_engine_id(k) {
                        if let Ok(health) = serde_json::from_value::<EngineHealth>(v.clone()) {
                            self.engine_health.insert(eid, health);
                        }
                    }
                }
            }
        }
        if let Some(c) = state.get("evolution_counter").and_then(|v| v.as_u64()) {
            self.evolution_counter = c;
        }
        Ok(format!("EvolutionBus state loaded from {:?}", path))
    }
}

impl Default for EvolutionBus {
    fn default() -> Self { Self::new() }
}

// ─── 工具 ──────────────────────────────────────────────────────────

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn event_type_label(et: &EvolutionEventType) -> &str {
    match et {
        EvolutionEventType::ParameterTuned => "参数调优",
        EvolutionEventType::ThresholdAdjusted => "阈值调整",
        EvolutionEventType::StrategyUpdated => "策略更新",
        EvolutionEventType::WeightEvolved => "权重进化",
        EvolutionEventType::DegradationAlert => "退化告警",
        EvolutionEventType::ImprovementDetected => "改进检测",
    }
}

fn parse_engine_id(s: &str) -> Result<EngineId, String> {
    match s {
        "Distillation" => Ok(EngineId::Distillation),
        "Scheduling" => Ok(EngineId::Scheduling),
        "HallucinationGuard" => Ok(EngineId::HallucinationGuard),
        "CacheEngine" => Ok(EngineId::CacheEngine),
        "AgentQuality" => Ok(EngineId::AgentQuality),
        "Collaboration" => Ok(EngineId::Collaboration),
        "TaskIntelligence" => Ok(EngineId::TaskIntelligence),
        "PredictiveAnalytics" => Ok(EngineId::PredictiveAnalytics),
        "LocalAnalytics" => Ok(EngineId::LocalAnalytics),
        _ => Err(format!("Unknown engine id: {}", s)),
    }
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_evolution() {
        let mut bus = EvolutionBus::new();
        bus.record_evolution(
            EngineId::Distillation,
            EvolutionEventType::ParameterTuned,
            "code_retention", 0.9, 0.92,
            "Quality feedback improved",
        );
        assert_eq!(bus.event_log.len(), 1);
        assert_eq!(bus.engine_health.get(&EngineId::Distillation).unwrap().evolution_count, 1);
    }

    #[test]
    fn test_feedback_performance() {
        let mut bus = EvolutionBus::new();
        let new_val = bus.feedback_performance(
            EngineId::CacheEngine, "hit_rate", 0.6, 0.8, true,
        );
        assert!(new_val.is_some());
        assert!(new_val.unwrap() > 0.6);
    }

    #[test]
    fn test_feedback_no_change_when_close() {
        let mut bus = EvolutionBus::new();
        let new_val = bus.feedback_performance(
            EngineId::CacheEngine, "hit_rate", 0.79, 0.8, true,
        );
        assert!(new_val.is_none()); // Error < 0.05, no adjustment
    }

    #[test]
    fn test_transfer_knowledge() {
        let mut bus = EvolutionBus::new();
        let mut target_val = 0.5;
        let transferred = bus.transfer_knowledge(
            EngineId::Distillation, EngineId::Scheduling,
            "confidence", 0.9, &mut target_val, 0.8,
        );
        assert!(transferred);
        assert!(target_val > 0.5); // Blended toward 0.9
    }

    #[test]
    fn test_transfer_low_similarity() {
        let mut bus = EvolutionBus::new();
        let mut target_val = 0.5;
        let transferred = bus.transfer_knowledge(
            EngineId::Distillation, EngineId::Scheduling,
            "confidence", 0.9, &mut target_val, 0.3,
        );
        assert!(!transferred);
        assert_eq!(target_val, 0.5); // Unchanged
    }

    #[test]
    fn test_assess_advancement() {
        let mut bus = EvolutionBus::new();
        bus.assess_advancement(&[
            (EngineId::Distillation, 0.85, 0.92),
            (EngineId::CacheEngine, 0.5, 0.7),
        ]);
        let dist = bus.engine_health.get(&EngineId::Distillation).unwrap();
        let cache = bus.engine_health.get(&EngineId::CacheEngine).unwrap();
        assert!(dist.advancement_score > cache.advancement_score);
    }

    #[test]
    fn test_health_report() {
        let mut bus = EvolutionBus::new();
        bus.record_evolution(EngineId::Distillation, EvolutionEventType::WeightEvolved,
            "fact_extraction", 0.8, 0.85, "Test");
        let report = bus.health_report();
        assert_eq!(report["engines_tracked"], 9);
        assert!(report["total_evolutions"].as_u64().unwrap() > 0);
    }

    #[test]
    fn test_safety_clamp() {
        let mut bus = EvolutionBus::new();
        // Simulate a massive drop that would be unsafe
        let new_val = bus.feedback_performance(
            EngineId::HallucinationGuard, "detection_rate", 0.9, 0.1, true,
        );
        assert!(new_val.is_some());
        // Should be clamped to not drop below 70% of original
        assert!(new_val.unwrap() >= 0.63); // 0.9 * 0.7
    }
}

// ─── Tauri Commands ──────────────────────────────────────────────

fn parse_evo_engine(s: &str) -> Result<EngineId, String> {
    match s {
        "distillation" => Ok(EngineId::Distillation),
        "scheduling" => Ok(EngineId::Scheduling),
        "hallucination" => Ok(EngineId::HallucinationGuard),
        "cache" => Ok(EngineId::CacheEngine),
        "agent_quality" => Ok(EngineId::AgentQuality),
        "collaboration" => Ok(EngineId::Collaboration),
        "task_intelligence" => Ok(EngineId::TaskIntelligence),
        "predictive" => Ok(EngineId::PredictiveAnalytics),
        "local_analytics" => Ok(EngineId::LocalAnalytics),
        _ => Err(format!("Unknown engine: {}", s)),
    }
}

#[tauri::command]
pub fn evobus_health_report(
    state: tauri::State<crate::state::AppState>,
) -> Result<serde_json::Value, String> {
    let bus = state.evolution_bus.lock().unwrap();
    Ok(bus.health_report())
}

#[tauri::command]
pub fn evobus_record_feedback(
    state: tauri::State<crate::state::AppState>,
    engine: String,
    metric: String,
    current_value: f64,
    target_value: f64,
    direction_is_higher_better: Option<bool>,
) -> Result<serde_json::Value, String> {
    let mut bus = state.evolution_bus.lock().unwrap();
    let eid = parse_evo_engine(&engine)?;
    let new_val = bus.feedback_performance(eid, &metric, current_value, target_value, direction_is_higher_better.unwrap_or(true));
    Ok(serde_json::json!({
        "adjusted": new_val.is_some(),
        "new_value": new_val,
        "engine": engine,
        "metric": metric,
    }))
}

#[tauri::command]
pub fn hallucination_feedback(
    state: tauri::State<crate::state::AppState>,
    is_false_positive: Option<bool>,
) -> Result<serde_json::Value, String> {
    let mut evo = state.evolution_bus.lock().unwrap();
    let accuracy = if is_false_positive.unwrap_or(false) { 0.6 } else { 0.9 };
    let fp_rate = if is_false_positive.unwrap_or(false) { 0.25 } else { 0.1 };

    evo.feedback_performance(
        EngineId::HallucinationGuard,
        "accuracy", accuracy, 0.9, true,
    );
    evo.feedback_performance(
        EngineId::HallucinationGuard,
        "false_positive_rate", fp_rate, 0.1, false,
    );

    Ok(serde_json::json!({
        "recorded": true,
        "is_false_positive": is_false_positive.unwrap_or(false),
        "accuracy": accuracy,
        "false_positive_rate": fp_rate,
    }))
}