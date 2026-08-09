// 全自动 Agent 调度 + 技能匹配引擎 v2
//
// 核心算法：
//   1. 意图检测 — 加权关键词评分 + 置信度分级 + 多意图混合检测
//   2. Agent 推荐 — 匹配最佳 Agent 角色
//   3. 模型建议 — 根据任务复杂度 + 意图置信度选最优模型
//   4. 技能匹配 — 本地 Skill 命中检测
//
// 科学化升级: 加权评分替代优先级级联

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── 意图分类 ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IntentCategory {
    CodeGeneration, CodeReview, Architecture, Debugging, Security,
    Documentation, DataAnalysis, ApiDesign, UiUx, General,
    Refactoring, Testing, LegalCompliance,
}

impl IntentCategory {
    pub fn label(&self) -> &str {
        match self {
            IntentCategory::CodeGeneration => "代码生成",
            IntentCategory::CodeReview => "代码审查",
            IntentCategory::Architecture => "架构设计",
            IntentCategory::Debugging => "调试修复",
            IntentCategory::Security => "安全审计",
            IntentCategory::Documentation => "文档编写",
            IntentCategory::DataAnalysis => "数据分析",
            IntentCategory::ApiDesign => "API设计",
            IntentCategory::UiUx => "UI/UX设计",
            IntentCategory::General => "通用对话",
            IntentCategory::Refactoring => "代码重构",
            IntentCategory::Testing => "测试编写",
            IntentCategory::LegalCompliance => "法律合规",
        }
    }
}

// ─── Agent 角色 ──────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduledAgent {
    Coder, Auditor, Architect, Reviewer, Tester, Documenter, Compliance, Generalist,
}

impl ScheduledAgent {
    pub fn label(&self) -> &str {
        match self {
            ScheduledAgent::Coder => "Coder", ScheduledAgent::Auditor => "Auditor",
            ScheduledAgent::Architect => "Architect", ScheduledAgent::Reviewer => "Reviewer",
            ScheduledAgent::Tester => "Tester", ScheduledAgent::Documenter => "Documenter",
            ScheduledAgent::Compliance => "ComplianceOfficer", ScheduledAgent::Generalist => "Generalist",
        }
    }
    pub fn icon(&self) -> &str {
        match self {
            ScheduledAgent::Coder => "🦀", ScheduledAgent::Auditor => "🛡️",
            ScheduledAgent::Architect => "🏗️", ScheduledAgent::Reviewer => "🔍",
            ScheduledAgent::Tester => "🧪", ScheduledAgent::Documenter => "📝",
            ScheduledAgent::Compliance => "⚖️", ScheduledAgent::Generalist => "🤖",
        }
    }
}

// ─── 调度结果 ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingResult {
    pub intent: IntentCategory,
    pub confidence: f32,
    pub recommended_agent: ScheduledAgent,
    pub recommended_model: String,
    pub model_reason: String,
    pub matched_skill: Option<String>,
    pub optimization_tip: Option<String>,
    pub suggest_subagent: bool,
    pub secondary_intents: Vec<(String, f32, f32)>, // 次要意图
}

// ─── 调度引擎 v2 ──────────────────────────────────────────────────

pub struct AgentSchedulingEngine;

impl AgentSchedulingEngine {
    pub fn new() -> Self { Self }

    pub fn analyze(&self, user_message: &str) -> SchedulingResult {
        let lower = user_message.to_lowercase();
        let len = user_message.chars().count();

        let (intent, confidence) = self.detect_intent(&lower, len);
        let agent = self.map_to_agent(&intent);
        let (model, reason) = self.recommend_model(&intent, len);
        let skill = self.match_skill(&lower, &intent);
        let tip = self.generate_tip(&intent, len, skill.is_some());
        let suggest_subagent = matches!(intent,
            IntentCategory::CodeReview | IntentCategory::Security |
            IntentCategory::Architecture | IntentCategory::Debugging |
            IntentCategory::LegalCompliance
        ) && len > 200;

        // 多意图检测
        let all = self.detect_all_intents(&lower);
        let secondary: Vec<_> = all.iter().skip(1).take(3)
            .map(|(i, s, c)| (i.label().to_string(), *s, *c))
            .collect();

        SchedulingResult { intent, confidence, recommended_agent: agent,
            recommended_model: model, model_reason: reason,
            matched_skill: skill, optimization_tip: tip,
            suggest_subagent, secondary_intents: secondary,
        }
    }

    // ── 加权关键词评分体系 ──────────────────────────────────────

    fn keyword_weights() -> Vec<(&'static str, f32, IntentCategory)> {
        vec![
            ("gdpr", 4.0, IntentCategory::LegalCompliance),
            ("compliance", 3.5, IntentCategory::LegalCompliance),
            ("license", 3.0, IntentCategory::LegalCompliance),
            ("合规", 4.0, IntentCategory::LegalCompliance),
            ("security", 3.5, IntentCategory::Security),
            ("vulnerability", 4.0, IntentCategory::Security),
            ("xss", 4.0, IntentCategory::Security),
            ("sql injection", 4.5, IntentCategory::Security),
            ("安全", 3.5, IntentCategory::Security),
            ("漏洞", 4.0, IntentCategory::Security),
            ("architecture", 3.5, IntentCategory::Architecture),
            ("microservice", 4.0, IntentCategory::Architecture),
            ("system design", 4.0, IntentCategory::Architecture),
            ("架构", 4.0, IntentCategory::Architecture),
            ("微服务", 4.0, IntentCategory::Architecture),
            ("debug", 3.0, IntentCategory::Debugging),
            ("bug", 3.5, IntentCategory::Debugging),
            ("error", 2.5, IntentCategory::Debugging),
            ("fix", 2.5, IntentCategory::Debugging),
            ("调试", 3.5, IntentCategory::Debugging),
            ("修复", 3.0, IntentCategory::Debugging),
            ("generate", 2.5, IntentCategory::CodeGeneration),
            ("write code", 3.5, IntentCategory::CodeGeneration),
            ("implement", 3.0, IntentCategory::CodeGeneration),
            ("编写", 3.5, IntentCategory::CodeGeneration),
            ("生成", 3.0, IntentCategory::CodeGeneration),
            ("review", 3.0, IntentCategory::CodeReview),
            ("code review", 4.0, IntentCategory::CodeReview),
            ("审查", 4.0, IntentCategory::CodeReview),
            ("refactor", 3.5, IntentCategory::Refactoring),
            ("重构", 4.0, IntentCategory::Refactoring),
            ("test", 2.5, IntentCategory::Testing),
            ("unit test", 4.0, IntentCategory::Testing),
            ("测试", 4.0, IntentCategory::Testing),
            ("api", 3.0, IntentCategory::ApiDesign),
            ("endpoint", 3.5, IntentCategory::ApiDesign),
            ("接口", 3.5, IntentCategory::ApiDesign),
            ("ui", 3.0, IntentCategory::UiUx),
            ("css", 3.5, IntentCategory::UiUx),
            ("界面", 3.5, IntentCategory::UiUx),
            ("document", 3.0, IntentCategory::Documentation),
            ("readme", 3.5, IntentCategory::Documentation),
            ("文档", 4.0, IntentCategory::Documentation),
            ("data", 2.5, IntentCategory::DataAnalysis),
            ("analyze", 3.0, IntentCategory::DataAnalysis),
            ("数据", 3.5, IntentCategory::DataAnalysis),
            ("统计", 3.5, IntentCategory::DataAnalysis),
        ]
    }

    fn detect_intent(&self, text: &str, _len: usize) -> (IntentCategory, f32) {
        let lower = text.to_lowercase();
        let mut scores: HashMap<IntentCategory, f32> = HashMap::new();
        for (keyword, weight, intent) in &Self::keyword_weights() {
            if lower.contains(keyword) {
                let count = lower.matches(keyword).count() as f32;
                *scores.entry(intent.clone()).or_insert(0.0) += weight * count;
            }
        }
        if let Some((intent, score)) = scores.iter().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()) {
            let confidence = (*score / 5.0).min(0.99);
            (intent.clone(), (confidence * 100.0).round() / 100.0)
        } else {
            (IntentCategory::General, 0.5)
        }
    }

    pub fn detect_all_intents(&self, text: &str) -> Vec<(IntentCategory, f32, f32)> {
        let lower = text.to_lowercase();
        let mut scores: HashMap<IntentCategory, f32> = HashMap::new();
        for (keyword, weight, intent) in &Self::keyword_weights() {
            if lower.contains(keyword) {
                let count = lower.matches(keyword).count() as f32;
                *scores.entry(intent.clone()).or_insert(0.0) += weight * count;
            }
        }
        let max_score = scores.values().cloned().fold(0.0f32, f32::max).max(1.0);
        let mut results: Vec<_> = scores.into_iter()
            .map(|(i, s)| { let c = (s / max_score).min(0.99); (i, s, (c * 100.0).round() / 100.0) })
            .collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results
    }

    // ── Agent 映射 ────────────────────────────────────────────────

    fn map_to_agent(&self, intent: &IntentCategory) -> ScheduledAgent {
        match intent {
            IntentCategory::CodeGeneration => ScheduledAgent::Coder,
            IntentCategory::CodeReview => ScheduledAgent::Reviewer,
            IntentCategory::Architecture => ScheduledAgent::Architect,
            IntentCategory::Debugging => ScheduledAgent::Coder,
            IntentCategory::Security => ScheduledAgent::Auditor,
            IntentCategory::Documentation => ScheduledAgent::Documenter,
            IntentCategory::DataAnalysis => ScheduledAgent::Generalist,
            IntentCategory::ApiDesign => ScheduledAgent::Architect,
            IntentCategory::UiUx => ScheduledAgent::Coder,
            IntentCategory::Refactoring => ScheduledAgent::Coder,
            IntentCategory::Testing => ScheduledAgent::Tester,
            IntentCategory::LegalCompliance => ScheduledAgent::Compliance,
            IntentCategory::General => ScheduledAgent::Generalist,
        }
    }

    // ── 模型推荐 ──────────────────────────────────────────────────

    fn recommend_model(&self, intent: &IntentCategory, msg_len: usize) -> (String, String) {
        match intent {
            IntentCategory::LegalCompliance => (
                "kimi-k3".into(), "法律合规审查需完整阅读协议全文，Kimi K3 的 1M 上下文窗口确保不遗漏条款".into(),
            ),
            IntentCategory::Architecture | IntentCategory::Security => (
                "deepseek-v4-pro".into(), "深度推理任务推荐 DeepSeek V4-Pro 以获得最佳分析质量".into(),
            ),
            IntentCategory::CodeReview | IntentCategory::DataAnalysis if msg_len > 2000 => (
                "kimi-k3".into(), "长上下文分析，Kimi K3 的 1M 窗口确保完整理解".into(),
            ),
            IntentCategory::ApiDesign => (
                "glm-5.2".into(), "API 设计适合 GLM-5.2 的原生 Agent 规划能力".into(),
            ),
            IntentCategory::CodeGeneration | IntentCategory::Debugging if msg_len < 500 => (
                "deepseek-v4-flash".into(), "轻量任务，DeepSeek V4-Flash 速度最快且支持 1 折缓存".into(),
            ),
            _ => ("deepseek-v4-flash".into(), "通用任务，推荐最具性价比的 DeepSeek V4-Flash".into()),
        }
    }

    fn match_skill(&self, text: &str, intent: &IntentCategory) -> Option<String> {
        if text.contains("rewind") || text.contains("回滚") || text.contains("恢复") {
            return Some("chronos_omni_rewind_trigger".into());
        }
        if text.contains("snapshot") || text.contains("快照") { return Some("checkpoints_chronotrigger".into()); }
        if text.contains("docker") || text.contains("container") { return Some("cluster_docker_hothealer".into()); }
        if text.contains("privacy") || text.contains("mask") { return Some("vlm_privacy_dynamic_mask".into()); }
        if text.contains("excel") || text.contains("表格") { return Some("context_glue_excelfiller".into()); }
        if *intent == IntentCategory::CodeReview { return Some("vlm_diff_inspector".into()); }
        None
    }

    fn generate_tip(&self, intent: &IntentCategory, len: usize, has_skill: bool) -> Option<String> {
        if has_skill { return Some("💡 匹配到本地 Skill — 零 Token 执行，不消耗 API 费用".into()); }
        match intent {
            IntentCategory::CodeGeneration if len < 100 =>
                Some("⚡ 短任务建议使用 DeepSeek V4-Flash (¥0.10/1M)，成本最低".into()),
            IntentCategory::Architecture =>
                Some("🏗️ 建议开启 DeepSeek Context Caching — 固定 System Prompt 可触发 1 折计费".into()),
            _ if len > 4000 =>
                Some("📏 消息较长，建议使用 Kimi K3 (1M 窗口) 或压缩历史上下文".into()),
            _ => None,
        }
    }
}

impl Default for AgentSchedulingEngine {
    fn default() -> Self { Self::new() }
}
