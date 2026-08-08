// 全自动 Agent 调度 + 技能匹配引擎
//
// 核心算法：
//   1. 意图检测 — 关键词 + 模式匹配，0 Token 消耗
//   2. Agent 推荐 — 匹配最佳 Agent 角色
//   3. 模型建议 — 根据任务复杂度选最优模型
//   4. 技能匹配 — 本地 Skill 命中检测
//
// 设计原则：全部端侧计算，不消耗 API Token

use serde::{Deserialize, Serialize};

// ─── 意图分类 ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentCategory {
    CodeGeneration,    // 代码生成
    CodeReview,        // 代码审查
    Architecture,      // 架构设计
    Debugging,         // 调试/Bug修复
    Security,          // 安全审计
    Documentation,     // 文档编写
    DataAnalysis,      // 数据分析
    ApiDesign,         // API设计
    UiUx,              // UI/UX设计
    General,           // 通用对话
    Refactoring,       // 代码重构
    Testing,           // 测试编写
    LegalCompliance,   // 法律合规
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
    Coder,        // 代码生成/调试
    Auditor,      // 安全审计
    Architect,    // 架构设计
    Reviewer,     // 代码审查
    Tester,       // 测试编写
    Documenter,   // 文档编写
    Compliance,   // 法律合规
    Generalist,   // 通用助手
}

impl ScheduledAgent {
    pub fn label(&self) -> &str {
        match self {
            ScheduledAgent::Coder => "Coder",
            ScheduledAgent::Auditor => "Auditor",
            ScheduledAgent::Architect => "Architect",
            ScheduledAgent::Reviewer => "Reviewer",
            ScheduledAgent::Tester => "Tester",
            ScheduledAgent::Documenter => "Documenter",
            ScheduledAgent::Compliance => "ComplianceOfficer",
            ScheduledAgent::Generalist => "Generalist",
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            ScheduledAgent::Coder => "🦀",
            ScheduledAgent::Auditor => "🛡️",
            ScheduledAgent::Architect => "🏗️",
            ScheduledAgent::Reviewer => "🔍",
            ScheduledAgent::Tester => "🧪",
            ScheduledAgent::Documenter => "📝",
            ScheduledAgent::Compliance => "⚖️",
            ScheduledAgent::Generalist => "🤖",
        }
    }
}

// ─── 调度结果 ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingResult {
    /// 检测到的意图
    pub intent: IntentCategory,
    /// 置信度 0.0-1.0
    pub confidence: f32,
    /// 推荐的 Agent
    pub recommended_agent: ScheduledAgent,
    /// 推荐的最优模型
    pub recommended_model: String,
    /// 模型推荐理由
    pub model_reason: String,
    /// 匹配到的本地 Skill（可选）
    pub matched_skill: Option<String>,
    /// 优化提示
    pub optimization_tip: Option<String>,
    /// 是否建议使用子Agent
    pub suggest_subagent: bool,
}

// ═══════════════════════════════════════════════════════════════════
// AgentSchedulingEngine
// ═══════════════════════════════════════════════════════════════════

pub struct AgentSchedulingEngine;

impl AgentSchedulingEngine {
    pub fn new() -> Self { Self }

    /// 核心算法：分析用户输入，输出调度建议
    pub fn analyze(&self, user_message: &str) -> SchedulingResult {
        let lower = user_message.to_lowercase();
        let len = user_message.chars().count();

        // ── 意图检测 ──────────────────────────────────────────

        let (intent, confidence) = self.detect_intent(&lower, len);

        // ── Agent 推荐 ─────────────────────────────────────────

        let agent = self.map_to_agent(&intent);

        // ── 模型推荐 ──────────────────────────────────────────

        let (model, reason) = self.recommend_model(&intent, len);

        // ── 技能匹配 ───────────────────────────────────────────

        let skill = self.match_skill(&lower, &intent);

        // ── 优化提示 ──────────────────────────────────────────

        let tip = self.generate_tip(&intent, len, skill.is_some());

        // ── 子Agent 建议 ──────────────────────────────────────

        let suggest_subagent = matches!(intent,
            IntentCategory::CodeReview | IntentCategory::Security |
            IntentCategory::Architecture | IntentCategory::Debugging |
            IntentCategory::LegalCompliance
        ) && len > 200;

        SchedulingResult {
            intent,
            confidence,
            recommended_agent: agent,
            recommended_model: model,
            model_reason: reason,
            matched_skill: skill,
            optimization_tip: tip,
            suggest_subagent,
        }
    }

    // ── 意图检测算法 ────────────────────────────────────────────

    fn detect_intent(&self, text: &str, len: usize) -> (IntentCategory, f32) {
        // Legal/Compliance patterns (highest priority)
        if self.match_any(text, &["gdpr", "ccpa", "pipl", "compliance", "license", "gpl", "agpl",
            "apache", "mit license", "copyright", "patent", "trademark", "数据保护",
            "隐私法", "合规", "许可协议", "知识产权", "法律风险", "个人信息保护",
            "terms of service", "privacy policy", "nda", "confidential",
            "regulatory", "audit trail", "data residency"]) {
            return (IntentCategory::LegalCompliance, 0.93);
        }

        // Security patterns (highest priority)
        if self.match_any(text, &["security", "vulnerability", "xss", "sql injection",
            "authentication", "authorization", "encrypt", "decrypt", "penetration",
            "安全", "漏洞", "注入", "认证", "加密", "渗透"]) {
            return (IntentCategory::Security, 0.92);
        }

        // Architecture patterns
        if self.match_any(text, &["architecture", "design pattern", "microservice",
            "system design", "scalability", "架构", "设计模式", "微服务", "系统设计"]) {
            return (IntentCategory::Architecture, 0.88);
        }

        // Debugging patterns
        if self.match_any(text, &["debug", "bug", "error", "crash", "fix", "not working",
            "调试", "错误", "崩溃", "修复", "不工作", "报错"]) {
            return (IntentCategory::Debugging, 0.90);
        }

        // Code review patterns
        if self.match_any(text, &["review", "code review", "refactor", "clean code",
            "审查", "重构", "代码质量"]) {
            return (IntentCategory::CodeReview, 0.85);
        }

        // Testing patterns
        if self.match_any(text, &["test", "unit test", "coverage", "mock", "assert",
            "测试", "单元测试", "覆盖率"]) {
            return (IntentCategory::Testing, 0.87);
        }

        // Code generation patterns
        if self.match_any(text, &["write", "create", "generate", "implement", "code",
            "function", "class", "component", "写", "创建", "生成", "实现", "函数", "类"]) {
            return (IntentCategory::CodeGeneration, 0.82);
        }

        // API design patterns
        if self.match_any(text, &["api", "endpoint", "rest", "graphql", "接口", "端点"]) {
            return (IntentCategory::ApiDesign, 0.84);
        }

        // Documentation patterns
        if self.match_any(text, &["document", "readme", "comment", "doc", "文档", "注释"]) {
            return (IntentCategory::Documentation, 0.80);
        }

        // Refactoring patterns
        if self.match_any(text, &["refactor", "restructure", "reorganize", "extract",
            "重构", "重组", "提取"]) {
            return (IntentCategory::Refactoring, 0.83);
        }

        // Data analysis patterns
        if self.match_any(text, &["analyze", "data", "statistics", "query", "sql",
            "分析", "数据", "统计", "查询"]) {
            return (IntentCategory::DataAnalysis, 0.81);
        }

        // UI/UX patterns
        if self.match_any(text, &["ui", "ux", "design", "css", "layout", "style",
            "界面", "布局", "样式", "设计"]) {
            return (IntentCategory::UiUx, 0.79);
        }

        // Default: General, confidence based on length
        let conf = if len > 500 { 0.55 } else if len > 100 { 0.65 } else { 0.75 };
        (IntentCategory::General, conf)
    }

    // ── Agent 映射 ───────────────────────────────────────────────

    fn map_to_agent(&self, intent: &IntentCategory) -> ScheduledAgent {
        match intent {
            IntentCategory::CodeGeneration => ScheduledAgent::Coder,
            IntentCategory::Debugging => ScheduledAgent::Coder,
            IntentCategory::Refactoring => ScheduledAgent::Coder,
            IntentCategory::CodeReview => ScheduledAgent::Reviewer,
            IntentCategory::Security => ScheduledAgent::Auditor,
            IntentCategory::Architecture => ScheduledAgent::Architect,
            IntentCategory::Testing => ScheduledAgent::Tester,
            IntentCategory::Documentation => ScheduledAgent::Documenter,
            IntentCategory::ApiDesign => ScheduledAgent::Architect,
            IntentCategory::UiUx => ScheduledAgent::Coder,
            IntentCategory::LegalCompliance => ScheduledAgent::Compliance,
            _ => ScheduledAgent::Generalist,
        }
    }

    // ── 模型推荐算法 ─────────────────────────────────────────────

    fn recommend_model(&self, intent: &IntentCategory, msg_len: usize) -> (String, String) {
        match intent {
            // Legal/Compliance → Kimi K3 (256K window for full document review)
            IntentCategory::LegalCompliance => (
                "kimi-k3".into(),
                "法律合规审查需要完整阅读协议全文，Kimi K3 的 256K 上下文窗口确保不遗漏条款".into(),
            ),
            // Deep reasoning tasks → Pro model
            IntentCategory::Architecture | IntentCategory::Security => (
                "deepseek-v4-pro".into(),
                "深度推理任务，推荐 DeepSeek V4-Pro 以获得最佳分析质量".into(),
            ),
            // Long context tasks → Kimi K3
            IntentCategory::CodeReview | IntentCategory::DataAnalysis
                if msg_len > 2000 => (
                "kimi-k3".into(),
                "长上下文分析，Kimi K3 的 256K 窗口确保完整理解".into(),
            ),
            // Agent/tool use → GLM
            IntentCategory::ApiDesign => (
                "glm-5.2".into(),
                "API 设计适合 GLM-5.2 的原生 Agent 规划能力".into(),
            ),
            // Short code tasks → Flash (cheapest)
            IntentCategory::CodeGeneration | IntentCategory::Debugging
                if msg_len < 500 => (
                "deepseek-v4-flash".into(),
                "轻量任务，DeepSeek V4-Flash 速度最快且支持 1 折缓存".into(),
            ),
            // Default → Flash
            _ => (
                "deepseek-v4-flash".into(),
                "通用任务，推荐最具性价比的 DeepSeek V4-Flash".into(),
            ),
        }
    }

    // ── 技能匹配 ─────────────────────────────────────────────────

    fn match_skill(&self, text: &str, intent: &IntentCategory) -> Option<String> {
        // Rewind/Snapshot detection
        if text.contains("rewind") || text.contains("回滚") || text.contains("恢复") {
            return Some("chronos_omni_rewind_trigger".into());
        }
        if text.contains("snapshot") || text.contains("快照") || text.contains("备份") {
            return Some("checkpoints_chronotrigger".into());
        }
        // Docker/healing
        if text.contains("docker") || text.contains("container") || text.contains("容器") {
            return Some("cluster_docker_hothealer".into());
        }
        // Privacy
        if text.contains("privacy") || text.contains("mask") || text.contains("隐私") {
            return Some("vlm_privacy_dynamic_mask".into());
        }
        // Excel/context glue
        if text.contains("excel") || text.contains("spreadsheet") || text.contains("表格") {
            return Some("context_glue_excelfiller".into());
        }
        // Diff
        if *intent == IntentCategory::CodeReview {
            return Some("vlm_diff_inspector".into());
        }
        // Win32
        if text.contains("window") || text.contains("handle") || text.contains("窗口") {
            return Some("win32_handle_texthijacker".into());
        }
        None
    }

    // ── 优化提示 ─────────────────────────────────────────────────

    fn generate_tip(&self, intent: &IntentCategory, len: usize, has_skill: bool) -> Option<String> {
        if has_skill {
            return Some("💡 匹配到本地 Skill — 零 Token 执行，不消耗 API 费用".into());
        }
        match intent {
            IntentCategory::CodeGeneration if len < 100 => {
                Some("⚡ 短任务建议使用 DeepSeek V4-Flash (¥0.10/1M)，成本最低".into())
            }
            IntentCategory::Architecture => {
                Some("🏗️ 建议开启 DeepSeek Context Caching — 固定 System Prompt 可触发 1 折计费".into())
            }
            IntentCategory::Security => {
                Some("🛡️ 安全任务可使用 @Auditor 子Agent 进行离线 AST 白盒扫描".into())
            }
            _ if len > 4000 => {
                Some("📏 消息较长，建议使用 Kimi K3 (256K 窗口) 或压缩历史上下文".into())
            }
            _ => None,
        }
    }

    // ─── 辅助 ────────────────────────────────────────────────────

    fn match_any(&self, text: &str, keywords: &[&str]) -> bool {
        keywords.iter().any(|kw| text.contains(kw))
    }
}

impl Default for AgentSchedulingEngine {
    fn default() -> Self { Self::new() }
}

// ─── 单元测试 ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_security() {
        let engine = AgentSchedulingEngine::new();
        let r = engine.analyze("发现一个SQL注入漏洞，请帮我修复");
        assert_eq!(r.intent, IntentCategory::Security);
        assert_eq!(r.recommended_agent, ScheduledAgent::Auditor);
        assert!(r.confidence > 0.9);
    }

    #[test]
    fn test_detect_code_generation() {
        let engine = AgentSchedulingEngine::new();
        let r = engine.analyze("请帮我写一个Rust函数来处理HTTP请求");
        assert_eq!(r.intent, IntentCategory::CodeGeneration);
        assert_eq!(r.recommended_agent, ScheduledAgent::Coder);
    }

    #[test]
    fn test_detect_architecture() {
        let engine = AgentSchedulingEngine::new();
        let r = engine.analyze("请设计一个微服务架构的系统设计方案");
        assert_eq!(r.intent, IntentCategory::Architecture);
        assert_eq!(r.recommended_agent, ScheduledAgent::Architect);
    }

    #[test]
    fn test_detect_debugging() {
        let engine = AgentSchedulingEngine::new();
        let r = engine.analyze("这个函数报错了，帮我debug一下");
        assert_eq!(r.intent, IntentCategory::Debugging);
    }

    #[test]
    fn test_short_task_model() {
        let engine = AgentSchedulingEngine::new();
        let r = engine.analyze("写一个 hello world");
        assert!(r.recommended_model.contains("flash"));
    }

    #[test]
    fn test_security_model() {
        let engine = AgentSchedulingEngine::new();
        let r = engine.analyze("审计这段代码的安全漏洞");
        assert!(r.recommended_model.contains("pro"));
    }

    #[test]
    fn test_skill_match() {
        let engine = AgentSchedulingEngine::new();
        let r = engine.analyze("帮我回滚到上一个版本");
        assert!(r.matched_skill.is_some());
    }

    #[test]
    fn test_no_skill_match() {
        let engine = AgentSchedulingEngine::new();
        let r = engine.analyze("今天天气怎么样");
        assert!(r.matched_skill.is_none());
        assert_eq!(r.intent, IntentCategory::General);
    }

    #[test]
    fn test_long_context_model() {
        let engine = AgentSchedulingEngine::new();
        let long_msg = "请审查这段代码".to_string() + &"x".repeat(2500);
        let r = engine.analyze(&long_msg);
        assert!(r.recommended_model.contains("kimi"));
    }

    #[test]
    fn test_subagent_suggestion() {
        let engine = AgentSchedulingEngine::new();
        let long_review = "请审查这段代码的安全性和架构设计".to_string() + &"y".repeat(300);
        let r = engine.analyze(&long_review);
        assert!(r.suggest_subagent);
    }
}
