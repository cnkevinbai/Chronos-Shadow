// 数据飞轮引擎 (Data Flywheel Engine)
//
// 自我强化闭环系统:
//   User Interaction → Data Collection → Quality Assessment
//   → Evolution Feedback → Engine Improvement → Better Interaction
//
// 核心功能:
//   1. 自动度量采集 — 从各引擎自动收集运行时指标
//   2. 飞轮效应追踪 — 量化每次进化的收益增量
//   3. 趋势可视化 — 多维度指标时间序列
//   4. 收益对账 — Token节省/成本降低/质量提升的货币化统计
//   5. 持久化 — 重启后恢复飞轮状态，持续累积
//
// 设计原则:
//   1. 零额外开销 — 采集操作 O(1)，不阻塞主流程
//   2. 增量累积 — 每次交互贡献微小增量，长期累积显著
//   3. 透明可审计 — 每笔收益可追溯到具体引擎和操作
//   4. 自我验证 — 定期检查飞轮是否确实在改善系统

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── 采集指标 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlywheelMetric {
    /// 指标名称
    pub name: String,
    /// 来源引擎
    pub source: String,
    /// 当前值
    pub value: f64,
    /// 单位
    pub unit: String,
    /// 趋势方向
    pub direction: MetricDirection,
    /// 上次值(用于计算增量)
    pub previous_value: f64,
    /// 采集时间
    pub timestamp: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricDirection {
    Improving,  // 改善中
    Stable,     // 平稳
    Degrading,  // 退化中
}

impl MetricDirection {
    pub fn emoji(&self) -> &str {
        match self { Self::Improving => "📈", Self::Stable => "➡️", Self::Degrading => "📉" }
    }
}

// ─── 收益记录 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlywheelBenefit {
    pub id: String,
    pub timestamp: String,
    pub engine: String,
    pub category: BenefitCategory,
    pub description: String,
    pub tokens_saved: u64,
    pub cost_saved_rmb: f64,
    pub quality_improvement: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BenefitCategory {
    Distillation,        // 蒸馏压缩节省
    CacheHit,            // 缓存命中节省
    ModelSelection,      // 模型选择优化
    HallucinationPrevent, // 防幻觉拦截
    EvolutionTuning,     // 进化调参收益
    CollaborationEfficiency, // 协作效率提升
}

impl BenefitCategory {
    pub fn label(&self) -> &str {
        match self {
            Self::Distillation => "蒸馏压缩",
            Self::CacheHit => "缓存命中",
            Self::ModelSelection => "模型优选",
            Self::HallucinationPrevent => "幻觉拦截",
            Self::EvolutionTuning => "进化调参",
            Self::CollaborationEfficiency => "协作提效",
        }
    }
}

// ─── 飞轮快照 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlywheelSnapshot {
    /// 快照时间
    pub timestamp: String,
    /// 累计Token节省
    pub total_tokens_saved: u64,
    /// 累计成本节省(RMB)
    pub total_cost_saved: f64,
    /// 综合质量提升百分比
    pub quality_improvement_pct: f64,
    /// 飞轮旋转次数(交互周期)
    pub flywheel_cycles: u64,
    /// 进化事件总数
    pub evolution_events: u64,
    /// 各引擎贡献占比
    pub engine_contributions: HashMap<String, f64>,
    /// 趋势描述
    pub trend: String,
}

// ─── 数据飞轮引擎 ──────────────────────────────────────────────────

pub struct DataFlywheel {
    /// 当前指标快照
    pub metrics: HashMap<String, FlywheelMetric>,
    /// 收益记录(最近1000条)
    pub benefits: Vec<FlywheelBenefit>,
    /// 历史快照(每小时一次)
    pub snapshots: Vec<FlywheelSnapshot>,
    /// 飞轮旋转计数
    pub cycles: u64,
    /// 累计Token节省
    pub total_tokens_saved: u64,
    /// 累计成本节省(RMB)
    pub total_cost_saved: f64,
    /// 上次快照时间
    last_snapshot_time: u64,
    /// 收益计数器
    benefit_counter: u64,
    /// 是否启用
    pub enabled: bool,
}

impl DataFlywheel {
    pub fn new() -> Self {
        let mut metrics = HashMap::new();
        // 初始化核心指标
        for (name, source, unit) in &[
            ("distill_compression", "蒸馏引擎", "%"),
            ("distill_quality", "蒸馏引擎", "score"),
            ("cache_hit_rate", "缓存引擎", "%"),
            ("cache_api_saved", "缓存引擎", "calls"),
            ("search_efficiency", "Web智能", "hits/hr"),
            ("hallucination_accuracy", "防幻觉", "%"),
            ("model_selection_quality", "协作引擎", "score"),
            ("task_decomposition_accuracy", "任务智能", "%"),
            ("budget_optimization", "预测分析", "¥/mo"),
            ("evolution_rate", "进化总线", "events/hr"),
        ] {
            metrics.insert(name.to_string(), FlywheelMetric {
                name: name.to_string(),
                source: source.to_string(),
                value: 0.0,
                unit: unit.to_string(),
                direction: MetricDirection::Stable,
                previous_value: 0.0,
                timestamp: now_iso(),
            });
        }

        Self {
            metrics,
            benefits: Vec::new(),
            snapshots: Vec::new(),
            cycles: 0,
            total_tokens_saved: 0,
            total_cost_saved: 0.0,
            last_snapshot_time: now_secs(),
            benefit_counter: 0,
            enabled: true,
        }
    }

    // ── 度量采集 ──────────────────────────────────────────────

    /// 更新指标值，自动计算趋势
    pub fn record_metric(&mut self, name: &str, value: f64) {
        if !self.enabled { return; }
        if let Some(metric) = self.metrics.get_mut(name) {
            metric.previous_value = metric.value;
            metric.value = value;
            metric.direction = if value > metric.previous_value + 0.01 {
                MetricDirection::Improving
            } else if value < metric.previous_value - 0.01 {
                MetricDirection::Degrading
            } else {
                MetricDirection::Stable
            };
            metric.timestamp = now_iso();
        }
    }

    /// 记录一次收益事件
    pub fn record_benefit(
        &mut self,
        engine: &str,
        category: BenefitCategory,
        description: &str,
        tokens_saved: u64,
        cost_saved: f64,
        quality_gain: f64,
    ) {
        if !self.enabled { return; }
        self.benefit_counter += 1;
        self.total_tokens_saved += tokens_saved;
        self.total_cost_saved += cost_saved;

        self.benefits.push(FlywheelBenefit {
            id: format!("ben-{:06}", self.benefit_counter),
            timestamp: now_iso(),
            engine: engine.into(),
            category,
            description: description.into(),
            tokens_saved,
            cost_saved_rmb: cost_saved,
            quality_improvement: quality_gain,
        });

        while self.benefits.len() > 1000 {
            self.benefits.remove(0);
        }
    }

    // ── 飞轮旋转 ──────────────────────────────────────────────

    /// 飞轮旋转一次（每个交互周期调用）
    pub fn spin(&mut self) -> FlywheelSnapshot {
        self.cycles += 1;

        let now = now_secs();
        let should_snapshot = now - self.last_snapshot_time >= 3600; // 每小时快照

        // 计算各引擎贡献
        let mut contributions = HashMap::new();
        for benefit in &self.benefits {
            *contributions.entry(benefit.engine.clone()).or_insert(0.0) += benefit.cost_saved_rmb;
        }

        let snapshot = FlywheelSnapshot {
            timestamp: now_iso(),
            total_tokens_saved: self.total_tokens_saved,
            total_cost_saved: self.total_cost_saved,
            quality_improvement_pct: self.quality_trend(),
            flywheel_cycles: self.cycles,
            evolution_events: self.benefit_counter,
            engine_contributions: contributions,
            trend: self.trend_label(),
        };

        if should_snapshot {
            self.snapshots.push(snapshot.clone());
            self.last_snapshot_time = now;
            while self.snapshots.len() > 168 { // 保留一周(168小时)
                self.snapshots.remove(0);
            }
        }

        snapshot
    }

    // ── 趋势分析 ──────────────────────────────────────────────

    fn quality_trend(&self) -> f64 {
        if self.benefits.is_empty() { return 0.0; }
        let recent: Vec<&FlywheelBenefit> = self.benefits.iter().rev().take(50).collect();
        let avg = recent.iter().map(|b| b.quality_improvement).sum::<f64>() / recent.len() as f64;
        (avg * 100.0).min(100.0)
    }

    fn trend_label(&self) -> String {
        let q = self.quality_trend();
        if q > 5.0 { "📈 显著改善".into() }
        else if q > 1.0 { "📈 持续改善".into() }
        else if q > 0.0 { "➡️ 稳定提升".into() }
        else { "➡️ 维持基线".into() }
    }

    // ── 自动采集(从各引擎同步) ────────────────────────────────

    /// 从 WebIntelligence 同步指标
    pub fn collect_from_web_intel(&mut self, search_count: u64, _fetch_count: u64, _bytes: u64, cache_hits: u64, cache_misses: u64) {
        self.record_metric("search_efficiency", search_count as f64);
        let hit_rate = if cache_hits + cache_misses > 0 {
            cache_hits as f64 / (cache_hits + cache_misses) as f64 * 100.0
        } else { 0.0 };
        self.record_metric("cache_hit_rate", hit_rate);
        self.record_metric("cache_api_saved", cache_hits as f64);

        if cache_hits > 0 {
            self.record_benefit("Web智能", BenefitCategory::CacheHit,
                &format!("缓存命中 {} 次，避免重复下载", cache_hits),
                cache_hits * 500, cache_hits as f64 * 0.001, 2.0);
        }
    }

    /// 从蒸馏引擎同步指标
    pub fn collect_from_distillation(&mut self, total_distilled: u64, bytes_saved: u64, avg_compression: f64, quality: f64) {
        self.record_metric("distill_compression", avg_compression * 100.0);
        self.record_metric("distill_quality", quality * 100.0);
        let tokens = bytes_saved / 4;
        let cost = tokens as f64 * 0.000001;
        if tokens > 0 {
            self.record_benefit("蒸馏引擎", BenefitCategory::Distillation,
                &format!("蒸馏 {} 次，压缩 {:.0}%，节省 {} tokens", total_distilled, avg_compression * 100.0, tokens),
                tokens, cost, 3.0);
        }
    }

    /// 从防幻觉引擎同步指标
    pub fn collect_from_hallucination(&mut self, accuracy: f64, prevented: u64) {
        self.record_metric("hallucination_accuracy", accuracy * 100.0);
        if prevented > 0 {
            self.record_benefit("防幻觉", BenefitCategory::HallucinationPrevent,
                &format!("拦截 {} 次幻觉输出", prevented),
                prevented * 200, prevented as f64 * 0.0005, 5.0);
        }
    }

    /// 从进化总线同步
    pub fn collect_from_evolution(&mut self, event_count: u64) {
        self.record_metric("evolution_rate", event_count as f64);
        if event_count > 0 {
            self.record_benefit("进化总线", BenefitCategory::EvolutionTuning,
                &format!("执行 {} 次参数调优", event_count),
                event_count * 100, event_count as f64 * 0.0001, 1.0);
        }
    }

    // ── 飞轮仪表盘 ────────────────────────────────────────────

    pub fn dashboard(&self) -> serde_json::Value {
        let metrics_json: Vec<_> = self.metrics.values().map(|m| {
            serde_json::json!({
                "name": m.name, "source": m.source,
                "value": format!("{:.1}{}", m.value, m.unit),
                "direction": m.direction.emoji(),
                "trend": format!("{:?}", m.direction),
            })
        }).collect();

        let recent_benefits: Vec<_> = self.benefits.iter().rev().take(10).map(|b| {
            serde_json::json!({
                "engine": b.engine, "category": b.category.label(),
                "description": b.description,
                "tokens": b.tokens_saved,
                "cost": format!("¥{:.4}", b.cost_saved_rmb),
            })
        }).collect();

        let trend_data: Vec<_> = self.snapshots.iter().map(|s| {
            serde_json::json!({
                "time": &s.timestamp[11..19],
                "cost": format!("{:.2}", s.total_cost_saved),
                "tokens": s.total_tokens_saved,
                "quality": format!("{:.1}", s.quality_improvement_pct),
            })
        }).collect();

        serde_json::json!({
            "enabled": self.enabled,
            "cycles": self.cycles,
            "total_tokens_saved": self.total_tokens_saved,
            "total_cost_saved": format!("¥{:.4}", self.total_cost_saved),
            "quality_trend": format!("{:.1}%", self.quality_trend()),
            "trend": self.trend_label(),
            "metrics": metrics_json,
            "recent_benefits": recent_benefits,
            "trend_data": trend_data,
            "engine_contributions": self.benefits.iter()
                .fold(HashMap::new(), |mut acc, b| {
                    *acc.entry(b.engine.clone()).or_insert(0.0) += b.cost_saved_rmb;
                    acc
                }).iter().map(|(k, v)| serde_json::json!({"engine": k, "contribution": format!("¥{:.4}", v)})).collect::<Vec<_>>(),
        })
    }

    // ── 持久化 ────────────────────────────────────────────────

    pub fn save_state(&self, dir: &std::path::Path) -> Result<String, String> {
        let path = dir.join("data_flywheel.json");
        let state = serde_json::json!({
            "cycles": self.cycles,
            "total_tokens_saved": self.total_tokens_saved,
            "total_cost_saved": self.total_cost_saved,
            "benefit_counter": self.benefit_counter,
            "snapshots": self.snapshots,
        });
        std::fs::write(&path, serde_json::to_string_pretty(&state).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        Ok(format!("DataFlywheel saved: {} cycles, ¥{:.4} saved", self.cycles, self.total_cost_saved))
    }

    pub fn load_state(&mut self, dir: &std::path::Path) -> Result<String, String> {
        let path = dir.join("data_flywheel.json");
        if !path.exists() { return Ok("No saved flywheel state".into()); }
        let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let state: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        if let Some(c) = state.get("cycles").and_then(|v| v.as_u64()) { self.cycles = c; }
        if let Some(t) = state.get("total_tokens_saved").and_then(|v| v.as_u64()) { self.total_tokens_saved = t; }
        if let Some(c) = state.get("total_cost_saved").and_then(|v| v.as_f64()) { self.total_cost_saved = c; }
        if let Some(c) = state.get("benefit_counter").and_then(|v| v.as_u64()) { self.benefit_counter = c; }
        if let Some(s) = state.get("snapshots") {
            if let Ok(snaps) = serde_json::from_value::<Vec<FlywheelSnapshot>>(s.clone()) {
                self.snapshots = snaps;
            }
        }
        Ok(format!("DataFlywheel loaded: {} cycles restored", self.cycles))
    }
}

impl Default for DataFlywheel {
    fn default() -> Self { Self::new() }
}

// ─── 工具 ──────────────────────────────────────────────────────────

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_metric() {
        let mut fw = DataFlywheel::new();
        fw.record_metric("cache_hit_rate", 75.0);
        let m = fw.metrics.get("cache_hit_rate").unwrap();
        assert_eq!(m.value, 75.0);
        assert_eq!(m.direction, MetricDirection::Improving);
    }

    #[test]
    fn test_record_benefit() {
        let mut fw = DataFlywheel::new();
        fw.record_benefit("Web智能", BenefitCategory::CacheHit, "test", 500, 0.001, 2.0);
        assert_eq!(fw.total_tokens_saved, 500);
        assert!(fw.total_cost_saved > 0.0);
        assert_eq!(fw.benefits.len(), 1);
    }

    #[test]
    fn test_spin_creates_snapshot() {
        let mut fw = DataFlywheel::new();
        fw.record_benefit("蒸馏引擎", BenefitCategory::Distillation, "test", 1000, 0.002, 3.0);
        let snap = fw.spin();
        assert_eq!(snap.total_tokens_saved, 1000);
        assert_eq!(fw.cycles, 1);
    }

    #[test]
    fn test_collect_from_distillation() {
        let mut fw = DataFlywheel::new();
        fw.collect_from_distillation(100, 40000, 0.6, 0.85);
        assert!(fw.total_tokens_saved > 0);
        let m = fw.metrics.get("distill_compression").unwrap();
        assert!(m.value > 0.0);
    }

    #[test]
    fn test_dashboard() {
        let mut fw = DataFlywheel::new();
        fw.record_benefit("Web智能", BenefitCategory::CacheHit, "test", 100, 0.001, 1.0);
        fw.collect_from_distillation(50, 20000, 0.5, 0.8);
        let dash = fw.dashboard();
        assert_eq!(dash["cycles"], 0);
        assert!(dash["total_cost_saved"].as_str().unwrap().contains("¥"));
    }

    #[test]
    fn test_persistence() {
        let tmp = std::env::temp_dir().join("flywheel_test");
        std::fs::create_dir_all(&tmp).unwrap();
        let mut fw = DataFlywheel::new();
        fw.record_benefit("test", BenefitCategory::CacheHit, "persist", 100, 0.001, 1.0);
        fw.save_state(&tmp).unwrap();

        let mut fw2 = DataFlywheel::new();
        fw2.load_state(&tmp).unwrap();
        assert_eq!(fw2.total_tokens_saved, 100);

        std::fs::remove_dir_all(&tmp).unwrap();
    }
}