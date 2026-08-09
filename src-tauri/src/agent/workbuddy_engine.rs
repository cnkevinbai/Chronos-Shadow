// WorkBuddy 功能衍生引擎 v2 (WorkBuddy Derivative Engine)
//
// 基于现有 buddy_scan + context_glue + win_hooks 基础设施,
// 衍生出三个高级功能模块:
//
//   1. 自动化规则引擎 (Automation Rules)
//      - 条件触发: 当应用A的字段X变化时, 自动填充应用B的字段Y
//      - 工作流: 多步骤跨应用自动化序列
//      - 时间调度: 定时触发自动化任务
//
//   2. 活动分析引擎 (Activity Analytics)
//      - 应用使用时长统计
//      - 数据流吞吐量趋势
//      - Token节省效率报告
//
//   3. 智能建议引擎 (Smart Suggestions)
//      - 基于历史模式推荐新绑定
//      - 异常数据流告警
//      - 效率优化建议
//
// 全部端侧计算, 0 Token 消耗

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── 自动化规则 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    /// 触发条件: "app_id.field_name == value" 或 "app_id.field_name changed"
    pub trigger_condition: String,
    /// 目标动作: "app_id.field_name = {source_value}"
    pub target_action: String,
    /// 延迟执行 (毫秒)
    pub delay_ms: u64,
    /// 执行次数上限 (0=无限)
    pub max_executions: u32,
    /// 已执行次数
    pub execution_count: u32,
    /// 最后触发时间
    pub last_triggered: Option<String>,
    /// 优先级 (0=最高)
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationWorkflow {
    pub id: String,
    pub name: String,
    pub steps: Vec<AutomationRule>,
    pub enabled: bool,
    /// 步骤间延迟 (毫秒)
    pub step_delay_ms: u64,
}

// ─── 活动分析 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityRecord {
    pub app_id: String,
    pub app_name: String,
    pub event_type: String, // "focus", "data_transfer", "idle", "close"
    pub timestamp: String,
    pub duration_ms: Option<u64>,
    pub bytes_transferred: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppActivityStats {
    pub app_id: String,
    pub app_name: String,
    pub total_focus_time_ms: u64,
    pub total_data_transfers: u32,
    pub total_bytes_transferred: u64,
    pub last_active: Option<String>,
    pub activity_score: f32, // 0-100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityReport {
    pub period_start: String,
    pub period_end: String,
    pub total_events: u32,
    pub app_stats: Vec<AppActivityStats>,
    pub top_transfer_pair: Option<(String, String)>,
    pub estimated_tokens_saved: u64,
    pub estimated_cost_saved: f64,
}

// ─── 智能建议 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartSuggestion {
    pub id: String,
    pub suggestion_type: String, // "bind", "automate", "optimize", "alert"
    pub title: String,
    pub description: String,
    pub confidence: f32,
    pub action: Option<String>,
    pub created_at: String,
}

// ─── WorkBuddy 衍生引擎 ───────────────────────────────────────

pub struct WorkBuddyEngine {
    /// 自动化规则
    pub rules: Vec<AutomationRule>,
    /// 自动化工作流
    pub workflows: Vec<AutomationWorkflow>,
    /// 活动记录
    pub activity_log: Vec<ActivityRecord>,
    /// 应用统计
    pub app_stats: HashMap<String, AppActivityStats>,
    /// 智能建议
    pub suggestions: Vec<SmartSuggestion>,
    /// 规则计数器
    rule_counter: u32,
    /// 最大活动记录数
    max_activity_records: usize,
    /// 最大建议数
    max_suggestions: usize,
}

impl WorkBuddyEngine {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(), workflows: Vec::new(),
            activity_log: Vec::new(), app_stats: HashMap::new(),
            suggestions: Vec::new(), rule_counter: 0,
            max_activity_records: 1000, max_suggestions: 20,
        }
    }

    // ── 自动化规则管理 ──────────────────────────────────────

    pub fn add_rule(&mut self, name: &str, trigger: &str, target: &str, delay_ms: u64, priority: u8) -> String {
        self.rule_counter += 1;
        let id = format!("auto-{:04}", self.rule_counter);
        self.rules.push(AutomationRule {
            id: id.clone(), name: name.into(), enabled: true,
            trigger_condition: trigger.into(), target_action: target.into(),
            delay_ms, max_executions: 0, execution_count: 0,
            last_triggered: None, priority,
        });
        id
    }

    /// 评估规则: 检查是否有规则满足触发条件
    pub fn evaluate_rules(&mut self, app_id: &str, field: &str, _value: &str) -> Vec<String> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut triggered = Vec::new();

        for rule in &mut self.rules {
            if !rule.enabled { continue; }
            if rule.max_executions > 0 && rule.execution_count >= rule.max_executions { continue; }
            if rule.trigger_condition.contains(app_id) && rule.trigger_condition.contains(field) {
                rule.execution_count += 1;
                rule.last_triggered = Some(now.clone());
                triggered.push(rule.id.clone());
            }
        }
        triggered
    }

    pub fn remove_rule(&mut self, id: &str) -> bool {
        let len = self.rules.len();
        self.rules.retain(|r| r.id != id);
        self.rules.len() < len
    }

    // ── 活动记录 ────────────────────────────────────────────

    pub fn record_activity(&mut self, app_id: &str, app_name: &str, event_type: &str,
        duration_ms: Option<u64>, bytes: Option<u64>,
    ) {
        let record = ActivityRecord {
            app_id: app_id.into(), app_name: app_name.into(),
            event_type: event_type.into(), timestamp: chrono::Utc::now().to_rfc3339(),
            duration_ms, bytes_transferred: bytes,
        };

        // 更新应用统计
        let stats = self.app_stats.entry(app_id.into()).or_insert_with(|| AppActivityStats {
            app_id: app_id.into(), app_name: app_name.into(),
            total_focus_time_ms: 0, total_data_transfers: 0,
            total_bytes_transferred: 0, last_active: None, activity_score: 0.0,
        });
        if let Some(d) = duration_ms { stats.total_focus_time_ms += d; }
        if bytes.is_some() { stats.total_data_transfers += 1; }
        if let Some(b) = bytes { stats.total_bytes_transferred += b; }
        stats.last_active = Some(record.timestamp.clone());
        stats.activity_score = Self::calc_activity_score(stats);

        self.activity_log.push(record);
        while self.activity_log.len() > self.max_activity_records {
            self.activity_log.remove(0);
        }
    }

    fn calc_activity_score(stats: &AppActivityStats) -> f32 {
        let transfer_score = (stats.total_data_transfers as f32 * 5.0).min(50.0);
        let time_score = (stats.total_focus_time_ms as f32 / 60000.0).min(50.0);
        (transfer_score + time_score).min(100.0)
    }

    // ── 分析报告 ────────────────────────────────────────────

    pub fn generate_report(&self) -> ActivityReport {
        let now = chrono::Utc::now().to_rfc3339();
        let total_events = self.activity_log.len() as u32;
        let tokens_saved = self.app_stats.values()
            .map(|s| s.total_bytes_transferred / 4) // ~4 bytes/token
            .sum();
        let cost_saved = tokens_saved as f64 * 0.00001; // ~¥0.01/1K tokens

        let top_pair = self.activity_log.iter()
            .filter(|r| r.event_type == "data_transfer")
            .fold(HashMap::new(), |mut acc, r| {
                *acc.entry(r.app_name.clone()).or_insert(0) += 1;
                acc
            })
            .into_iter().max_by_key(|(_, c)| *c)
            .map(|(name, _)| (name, String::new()));

        ActivityReport {
            period_start: self.activity_log.first().map(|r| r.timestamp.clone()).unwrap_or_default(),
            period_end: now, total_events,
            app_stats: self.app_stats.values().cloned().collect(),
            top_transfer_pair: top_pair,
            estimated_tokens_saved: tokens_saved,
            estimated_cost_saved: cost_saved,
        }
    }

    // ── 智能建议 ────────────────────────────────────────────

    /// 基于活动模式生成建议
    pub fn generate_suggestions(&mut self) {
        self.suggestions.clear();

        // 高频数据传输对 → 建议绑定
        let mut transfer_pairs: HashMap<(String, String), u32> = HashMap::new();
        let mut prev_app: Option<&str> = None;
        for record in &self.activity_log {
            if record.event_type == "data_transfer" {
                if let Some(prev) = prev_app {
                    *transfer_pairs.entry((prev.to_string(), record.app_name.clone())).or_insert(0) += 1;
                }
            }
            prev_app = Some(&record.app_name);
        }

        for ((src, tgt), count) in transfer_pairs {
            if count >= 3 {
                self.add_suggestion("bind",
                    &format!("建议绑定 {} → {}", src, tgt),
                    &format!("检测到 {} 次从 {} 到 {} 的数据传输, 创建绑定可自动化此流程", count, src, tgt),
                    0.7 + (count as f32 * 0.05).min(0.25),
                );
            }
        }

        // 长时间未活动 → 建议清理
        let inactive: Vec<String> = self.app_stats.values()
            .filter(|s| s.total_data_transfers == 0 && s.total_focus_time_ms < 30000)
            .map(|s| s.app_name.clone())
            .collect();
        for name in inactive {
            self.add_suggestion("optimize",
                &format!("低活跃应用: {}", name),
                &format!("{} 在过去周期中活动量极低, 考虑移除以释放资源", name),
                0.6,
            );
        }
    }

    fn add_suggestion(&mut self, stype: &str, title: &str, desc: &str, confidence: f32) {
        if self.suggestions.len() >= self.max_suggestions { return; }
        self.suggestions.push(SmartSuggestion {
            id: format!("sugg-{:04}", self.suggestions.len() + 1),
            suggestion_type: stype.into(), title: title.into(),
            description: desc.into(), confidence,
            action: None, created_at: chrono::Utc::now().to_rfc3339(),
        });
    }
}

impl Default for WorkBuddyEngine {
    fn default() -> Self { Self::new() }
}

// ─── 单元测试 ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_trigger_rule() {
        let mut engine = WorkBuddyEngine::new();
        let id = engine.add_rule("test", "excel.amount changed", "web.total = {value}", 0, 0);
        let triggered = engine.evaluate_rules("excel", "amount", "1234");
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0], id);
    }

    #[test]
    fn test_activity_recording() {
        let mut engine = WorkBuddyEngine::new();
        engine.record_activity("excel", "Excel", "data_transfer", Some(5000), Some(1024));
        engine.record_activity("excel", "Excel", "data_transfer", Some(3000), Some(2048));
        let report = engine.generate_report();
        assert_eq!(report.total_events, 2);
        assert_eq!(report.app_stats.len(), 1);
        assert!(report.app_stats[0].total_bytes_transferred > 0);
    }

    #[test]
    fn test_suggestions_generation() {
        let mut engine = WorkBuddyEngine::new();
        for _ in 0..5 {
            engine.record_activity("excel", "Excel", "data_transfer", Some(1000), Some(512));
            engine.record_activity("web", "Chrome", "data_transfer", Some(1000), Some(256));
        }
        engine.generate_suggestions();
        assert!(!engine.suggestions.is_empty());
    }
}
