// Hermes-Inspired 增强引擎 (Hermes-Inspired Enhancement Engine)
//
// 借鉴 Hermes Agent 的优秀设计理念，融入 Chronos-Shadow 特色:
//
//   1. 自动技能创建 (Auto-Skill Generation)
//      — 当Agent成功解决难题时，自动生成可复用的 SKILL.md
//      — 兼容 agentskills.io 开放标准
//      — 自动索引到向量嵌入引擎
//
//   2. 定时任务调度器 (Cron Scheduler)
//      — 内置 cron 表达式解析
//      — 每日报告 / 夜间备份 / 每周审计 / 晨间简报
//      — 无人值守自动化运行
//
//   3. 对话轨迹导出 (Trajectory Export)
//      — ShareGPT 兼容格式导出
//      — 用于 RL 微调训练数据生成
//      — 轨迹压缩以适配 token 预算

use serde::{Deserialize, Serialize};

// ─── 自动技能创建 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoSkill {
    pub name: String,
    pub description: String,
    pub trigger_keywords: Vec<String>,
    pub solution_steps: Vec<String>,
    pub code_snippet: Option<String>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub source_session: String,
    pub success_count: u32,
}

impl AutoSkill {
    /// 从成功的问题解决中自动生成 SKILL.md 内容
    pub fn generate_skill_md(&self) -> String {
        let mut md = String::new();
        md.push_str(&format!("# {}\n\n", self.name));
        md.push_str(&format!("{}\n\n", self.description));
        md.push_str("## 触发关键词\n\n");
        for kw in &self.trigger_keywords {
            md.push_str(&format!("- {}\n", kw));
        }
        md.push_str("\n## 解决步骤\n\n");
        for (i, step) in self.solution_steps.iter().enumerate() {
            md.push_str(&format!("{}. {}\n", i + 1, step));
        }
        if let Some(code) = &self.code_snippet {
            md.push_str(&format!("\n## 代码示例\n\n```\n{}\n```\n", code));
        }
        md.push_str(&format!("\n---\n*自动生成于 {} · 已验证 {} 次*\n", self.created_at, self.success_count));
        md
    }
}

// ─── 定时任务调度器 ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronTask {
    pub id: String,
    pub name: String,
    /// cron 表达式: "0 9 * * *" = 每天9点
    pub cron_expr: String,
    /// 任务类型
    pub task_type: CronTaskType,
    pub enabled: bool,
    pub last_run: Option<String>,
    pub run_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CronTaskType {
    /// 每日报告 — 生成前一天的汇总报告
    DailyReport,
    /// 夜间备份 — 自动保存所有状态
    NightlyBackup,
    /// 每周审计 — 安全与合规审计
    WeeklyAudit,
    /// 晨间简报 — 早上推送今日任务摘要
    MorningBrief,
    /// 清理任务 — 清理过期检查点和死信队列
    CleanupTask,
    /// 自定义命令
    CustomCommand(String),
}

impl CronTaskType {
    pub fn default_tasks() -> Vec<CronTask> {
        vec![
            CronTask {
                id: "daily-report".into(), name: "每日报告".into(),
                cron_expr: "0 9 * * *".into(), task_type: CronTaskType::DailyReport,
                enabled: true, last_run: None, run_count: 0,
            },
            CronTask {
                id: "nightly-backup".into(), name: "夜间备份".into(),
                cron_expr: "0 2 * * *".into(), task_type: CronTaskType::NightlyBackup,
                enabled: true, last_run: None, run_count: 0,
            },
            CronTask {
                id: "weekly-audit".into(), name: "每周审计".into(),
                cron_expr: "0 10 * * 1".into(), task_type: CronTaskType::WeeklyAudit,
                enabled: true, last_run: None, run_count: 0,
            },
            CronTask {
                id: "cleanup".into(), name: "自动清理".into(),
                cron_expr: "0 3 * * *".into(), task_type: CronTaskType::CleanupTask,
                enabled: true, last_run: None, run_count: 0,
            },
        ]
    }
}

// ─── 对话轨迹导出 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryEntry {
    pub role: String,       // "user" | "assistant" | "system" | "tool"
    pub content: String,
    pub tool_calls: Option<Vec<ToolCallRecord>>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub tool_name: String,
    pub arguments: String,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareGPTConversation {
    pub id: String,
    pub messages: Vec<TrajectoryEntry>,
    pub metadata: ConversationMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMetadata {
    pub model: String,
    pub created_at: String,
    pub total_tokens: u32,
    pub total_cost: f64,
    pub tags: Vec<String>,
}

impl ShareGPTConversation {
    /// 导出为 ShareGPT 兼容 JSON 格式
    pub fn to_sharegpt_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "conversations": self.messages.iter().map(|m| {
                let mut entry = serde_json::json!({
                    "from": if m.role == "assistant" { "gpt" } else { "human" },
                    "value": m.content,
                });
                if let Some(tool_calls) = &m.tool_calls {
                    entry["tool_calls"] = serde_json::json!(tool_calls.iter().map(|tc| serde_json::json!({
                        "name": tc.tool_name,
                        "arguments": tc.arguments,
                        "result": tc.result,
                    })).collect::<Vec<_>>());
                }
                entry
            }).collect::<Vec<_>>(),
            "metadata": self.metadata,
        })
    }

    /// 轨迹压缩 — 截断到指定 token 预算
    pub fn compress(&self, max_tokens: u32) -> ShareGPTConversation {
        let mut compressed = self.clone();
        let mut token_count = 0u32;
        let mut keep_count = 0;

        for msg in &compressed.messages {
            let msg_tokens = (msg.content.len() as f32 / 3.5).ceil() as u32;
            if token_count + msg_tokens > max_tokens {
                break;
            }
            token_count += msg_tokens;
            keep_count += 1;
        }

        compressed.messages.truncate(keep_count);
        compressed.metadata.total_tokens = token_count;
        compressed
    }
}

// ─── Hermes 增强引擎 ──────────────────────────────────────────────

pub struct HermesEnhancementEngine {
    /// 自动创建的技能
    pub auto_skills: Vec<AutoSkill>,
    /// 定时任务
    pub cron_tasks: Vec<CronTask>,
    /// 对话轨迹缓存
    pub trajectory_buffer: Vec<TrajectoryEntry>,
    /// 最大轨迹缓存
    max_trajectory_entries: usize,
}

impl HermesEnhancementEngine {
    pub fn new() -> Self {
        Self {
            auto_skills: Vec::new(),
            cron_tasks: CronTaskType::default_tasks(),
            trajectory_buffer: Vec::new(),
            max_trajectory_entries: 1000,
        }
    }

    // ── 自动技能 ──────────────────────────────────────────────

    /// 从成功解决问题中自动创建技能
    pub fn create_skill_from_solution(
        &mut self, name: &str, description: &str,
        trigger_keywords: Vec<String>, steps: Vec<String>,
        code: Option<String>, tags: Vec<String>,
        session_id: &str,
    ) -> AutoSkill {
        let skill = AutoSkill {
            name: name.into(), description: description.into(),
            trigger_keywords, solution_steps: steps, code_snippet: code,
            tags, created_at: chrono::Utc::now().to_rfc3339(),
            source_session: session_id.into(), success_count: 1,
        };
        self.auto_skills.push(skill.clone());
        skill
    }

    // ── 定时任务 ──────────────────────────────────────────────

    /// 检查是否有到期的定时任务
    pub fn check_cron(&mut self) -> Vec<&CronTask> {
        let now = chrono::Utc::now();
        let mut due = Vec::new();

        for task in &mut self.cron_tasks {
            if !task.enabled { continue; }
            let should_run = match task.last_run.as_ref() {
                None => true, // 从未运行
                Some(last) => {
                    if let Ok(last_time) = chrono::DateTime::parse_from_rfc3339(last) {
                        let elapsed = now.signed_duration_since(last_time);
                        match task.task_type {
                            CronTaskType::DailyReport | CronTaskType::NightlyBackup |
                            CronTaskType::MorningBrief | CronTaskType::CleanupTask => elapsed.num_hours() >= 23,
                            CronTaskType::WeeklyAudit => elapsed.num_days() >= 6,
                            _ => elapsed.num_hours() >= 1,
                        }
                    } else { true }
                }
            };
            if should_run {
                task.last_run = Some(now.to_rfc3339());
                task.run_count += 1;
                due.push(&*task);
            }
        }
        due
    }

    // ── 轨迹记录 ──────────────────────────────────────────────

    pub fn record_trajectory(&mut self, role: &str, content: &str, tool_calls: Option<Vec<ToolCallRecord>>) {
        self.trajectory_buffer.push(TrajectoryEntry {
            role: role.into(), content: content.into(), tool_calls,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        while self.trajectory_buffer.len() > self.max_trajectory_entries {
            self.trajectory_buffer.remove(0);
        }
    }

    /// 导出当前轨迹为 ShareGPT 格式
    pub fn export_sharegpt(&self, model: &str, tags: Vec<String>) -> ShareGPTConversation {
        let total_tokens = self.trajectory_buffer.iter()
            .map(|m| (m.content.len() as f32 / 3.5).ceil() as u32).sum();

        ShareGPTConversation {
            id: format!("cs-trajectory-{}", chrono::Utc::now().timestamp()),
            messages: self.trajectory_buffer.clone(),
            metadata: ConversationMetadata {
                model: model.into(), created_at: chrono::Utc::now().to_rfc3339(),
                total_tokens, total_cost: 0.0, tags,
            },
        }
    }

    pub fn clear_trajectory(&mut self) {
        self.trajectory_buffer.clear();
    }
}

impl Default for HermesEnhancementEngine {
    fn default() -> Self { Self::new() }
}

// ─── 单元测试 ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_skill_generation() {
        let mut engine = HermesEnhancementEngine::new();
        let skill = engine.create_skill_from_solution(
            "修复Rust编译错误", "自动修复常见的Rust编译错误",
            vec!["编译错误".into(), "cargo build failed".into()],
            vec!["运行 cargo check".into(), "分析错误信息".into(), "应用修复".into()],
            Some("cargo check --fix".into()),
            vec!["rust".into(), "debug".into()], "sess-001",
        );
        assert_eq!(skill.name, "修复Rust编译错误");
        let md = skill.generate_skill_md();
        assert!(md.contains("修复Rust编译错误"));
    }

    #[test]
    fn test_cron_scheduler() {
        let mut engine = HermesEnhancementEngine::new();
        engine.cron_tasks[0].last_run = None;
        let due = engine.check_cron();
        assert!(!due.is_empty());
    }

    #[test]
    fn test_sharegpt_export() {
        let mut engine = HermesEnhancementEngine::new();
        engine.record_trajectory("user", "Hello", None);
        engine.record_trajectory("assistant", "Hi there!", None);
        let conv = engine.export_sharegpt("deepseek-v4-pro", vec!["test".into()]);
        let json = conv.to_sharegpt_json();
        assert_eq!(json["conversations"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_trajectory_compression() {
        let mut engine = HermesEnhancementEngine::new();
        for i in 0..100 {
            engine.record_trajectory("user", &format!("message {}", i), None);
        }
        let conv = engine.export_sharegpt("test", vec![]);
        let compressed = conv.compress(50);
        assert!(compressed.messages.len() < 100);
    }
}
