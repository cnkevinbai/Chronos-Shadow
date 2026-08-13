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

impl AgentSchedulingEngine {
    // ── 增强算法 v3：TF-IDF 意图分类 ──────────────────────────

    /// TF-IDF 风格评分：词频 × 逆文档频率，更精确的意图检测
    pub fn detect_intent_tfidf(&self, text: &str) -> Vec<(IntentCategory, f32, f32)> {
        let lower = text.to_lowercase();
        let words: Vec<&str> = lower.split_whitespace().collect();
        let total_words = words.len().max(1) as f32;

        // 计算每个词的 TF
        let mut tf: HashMap<String, f32> = HashMap::new();
        for w in &words {
            *tf.entry(w.to_string()).or_insert(0.0) += 1.0;
        }
        for v in tf.values_mut() { *v /= total_words; }

        // 计算每个意图类别的 TF-IDF 总得分
        let mut scores: HashMap<IntentCategory, f32> = HashMap::new();
        let keyword_count = Self::keyword_weights().len() as f32;

        for (keyword, weight, intent) in Self::keyword_weights() {
            // IDF: log(总关键词数 / 包含此关键词的文档数+1)
            let docs_with_keyword = Self::keyword_weights().iter()
                .filter(|(k, _, _)| k == &keyword).count() as f32;
            let idf = (keyword_count / (docs_with_keyword + 1.0)).ln().max(0.5);

            // 双词匹配加分
            let bigram_match = text.contains(&format!("{} {}", keyword, keyword));

            let tf_score = tf.get(keyword).copied().unwrap_or(0.0);
            let tfidf = tf_score * idf * weight * if bigram_match { 1.5 } else { 1.0 };

            *scores.entry(intent.clone()).or_insert(0.0) += tfidf;
        }

        let max_score = scores.values().cloned().fold(0.0f32, f32::max).max(1.0);
        let mut results: Vec<_> = scores.into_iter()
            .map(|(i, s)| {
                let confidence = (s / max_score).min(0.99);
                (i, s, (confidence * 100.0).round() / 100.0)
            })
            .collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results
    }

    // ── 增强算法：贝叶斯置信度更新 ────────────────────────────

    /// 贝叶斯更新：结合先验置信度和当前证据计算后验置信度
    /// posterior = (prior × likelihood) / normalizer
    pub fn bayesian_confidence(&self, prior: f32, evidence_strength: f32, contradiction_count: u32) -> f32 {
        // 似然度：证据强度 × 衰减因子
        let likelihood = evidence_strength * (0.9f32.powi(contradiction_count as i32));

        // 贝叶斯更新
        let posterior = (prior * likelihood) / (prior * likelihood + (1.0 - prior) * (1.0 - likelihood));

        // Beta 分布平滑
        let alpha = 2.0 + prior * 10.0;   // 伪计数
        let beta = 2.0 + (1.0 - prior) * 10.0;
        let smoothed = (alpha + posterior * 5.0) / (alpha + beta + 5.0);

        smoothed.min(0.99).max(0.01)
    }

    // ── 增强算法：N-gram 匹配 ──────────────────────────────────

    /// 提取 bigrams 并计算匹配度
    pub fn bigram_match_score(&self, text: &str, intent: &IntentCategory) -> f32 {
        let lower = text.to_lowercase();
        let words: Vec<&str> = lower.split_whitespace().collect();
        if words.len() < 2 { return 0.0; }

        // 生成 bigrams
        let bigrams: Vec<String> = words.windows(2)
            .map(|w| format!("{} {}", w[0], w[1]))
            .collect();

        // 获取此意图的关键词
        let keywords: Vec<&str> = Self::keyword_weights().iter()
            .filter(|(_, _, i)| i == intent)
            .map(|(k, _, _)| *k)
            .collect();

        if bigrams.is_empty() || keywords.is_empty() { return 0.0; }

        let mut matches = 0u32;
        for bg in &bigrams {
            for kw in &keywords {
                if bg.contains(kw) { matches += 1; }
            }
        }

        (matches as f32 / bigrams.len() as f32).min(1.0)
    }

    /// 提取 trigrams 用于精确匹配
    pub fn trigram_match_score(&self, text: &str, intent: &IntentCategory) -> f32 {
        let lower = text.to_lowercase();
        let words: Vec<&str> = lower.split_whitespace().collect();
        if words.len() < 3 { return 0.0; }

        let trigrams: Vec<String> = words.windows(3)
            .map(|w| format!("{} {} {}", w[0], w[1], w[2]))
            .collect();

        let patterns: Vec<&str> = match intent {
            IntentCategory::CodeGeneration => vec!["write a function", "generate code for", "implement the following"],
            IntentCategory::CodeReview => vec!["review this code", "check for bugs", "audit the implementation"],
            IntentCategory::Architecture => vec!["design the architecture", "system design for", "microservice architecture pattern"],
            IntentCategory::Debugging => vec!["fix this bug", "debug the error", "why is this"],
            IntentCategory::Security => vec!["security audit of", "vulnerability in the", "check for vulnerabilities"],
            IntentCategory::Refactoring => vec!["refactor this code", "clean up the", "improve code quality"],
            _ => vec![],
        };

        let mut matches = 0;
        for tg in &trigrams {
            for pat in &patterns {
                if tg.contains(pat) { matches += 1; }
            }
        }
        if trigrams.is_empty() || patterns.is_empty() { return 0.0; }
        (matches as f32 / trigrams.len() as f32 * 3.0).min(1.0)
    }

    // ── 增强算法：任务紧急度估算 ──────────────────────────────

    /// 估算任务紧急度 (0-1)，影响调度优先级和模型选择
    pub fn estimate_urgency(&self, text: &str) -> (f32, String) {
        let lower = text.to_lowercase();
        let mut urgency: f32 = 0.3_f32; // 默认基准

        let urgent_signals = [
            ("urgent", 0.4), ("asap", 0.5), ("紧急", 0.5), ("尽快", 0.4),
            ("critical", 0.5), ("blocking", 0.5), ("crash", 0.6), ("崩溃", 0.6),
            ("production", 0.4), ("生产环境", 0.5), ("down", 0.5), ("outage", 0.5),
            ("hotfix", 0.5), ("immediately", 0.4), ("马上", 0.4),
        ];

        for (signal, weight) in &urgent_signals {
            if lower.contains(signal) { urgency += weight; }
        }

        urgency = urgency.min(1.0_f32);

        let label = if urgency > 0.8 { "🔴 紧急" }
            else if urgency > 0.6 { "🟠 高优先级" }
            else if urgency > 0.4 { "🟡 中优先级" }
            else { "🟢 正常" };

        (urgency, label.into())
    }

    // ── 综合调度决策 (融合所有增强算法) ─────────────────────

    /// 综合调度：TF-IDF + Bayesian + Bigram + Urgency → 最优决策
    pub fn analyze_enhanced(&self, user_message: &str) -> serde_json::Value {
        let (intent, orig_conf) = self.detect_intent(user_message, 0);
        let tfidf_results = self.detect_intent_tfidf(user_message);
        let primary_tfidf = tfidf_results.first();

        // 贝叶斯融合：原始置信度 + TF-IDF 置信度
        let tfidf_conf = primary_tfidf.map(|(_, _, c)| *c).unwrap_or(orig_conf);
        let evidence_strength = (orig_conf + tfidf_conf) / 2.0;
        let bayesian_conf = self.bayesian_confidence(orig_conf, evidence_strength, 0);

        // N-gram 验证
        let bigram_score = self.bigram_match_score(user_message, &intent);
        let trigram_score = self.trigram_match_score(user_message, &intent);
        let ngram_validation = (bigram_score + trigram_score) / 2.0;

        // 融合置信度
        let fused_confidence = (bayesian_conf * 0.4 + tfidf_conf * 0.3 + ngram_validation * 0.3).min(0.99);

        // 紧急度
        let (urgency, urgency_label) = self.estimate_urgency(user_message);

        let agent = self.map_to_agent(&intent);
        let (model, reason) = self.recommend_model(&intent, user_message.len());
        let skill = self.match_skill(user_message, &intent);

        // 如果紧急度高但推荐了慢模型，切换为快速模型
        let final_model = if urgency > 0.7 && model == "deepseek-v4-pro" {
            ("kimi-k2.7-code-highspeed".to_string(), "紧急任务切换至极速模型".to_string())
        } else { (model, reason) };

        serde_json::json!({
            "primary_intent": intent.label(),
            "fused_confidence": format!("{:.1}%", fused_confidence * 100.0),
            "confidence_breakdown": {
                "original": format!("{:.1}%", orig_conf * 100.0),
                "tfidf": format!("{:.1}%", tfidf_conf * 100.0),
                "bayesian": format!("{:.1}%", bayesian_conf * 100.0),
                "ngram_validation": format!("{:.1}%", ngram_validation * 100.0),
            },
            "secondary_intents": tfidf_results.iter().skip(1).take(3)
                .map(|(i, _, c)| serde_json::json!({"intent": i.label(), "confidence": format!("{:.1}%", c * 100.0)}))
                .collect::<Vec<_>>(),
            "recommended_agent": agent.label(),
            "recommended_model": final_model.0,
            "model_reason": final_model.1,
            "matched_skill": skill,
            "urgency": urgency_label,
            "urgency_score": format!("{:.1}", urgency),
            "ngram_scores": {
                "bigram": format!("{:.1}%", bigram_score * 100.0),
                "trigram": format!("{:.1}%", trigram_score * 100.0),
            },
        })
    }
}

impl Default for AgentSchedulingEngine {
    fn default() -> Self { Self::new() }
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tfidf_intent() {
        let engine = AgentSchedulingEngine::new();
        let results = engine.detect_intent_tfidf("write secure code for the API endpoint");
        assert!(!results.is_empty());
        let top = &results[0];
        assert!(top.2 > 0.0);
    }

    #[test]
    fn test_bayesian_update() {
        let engine = AgentSchedulingEngine::new();
        let posterior = engine.bayesian_confidence(0.7, 0.8, 0);
        assert!(posterior > 0.85); // Strong evidence should increase confidence
    }

    #[test]
    fn test_bayesian_with_contradiction() {
        let engine = AgentSchedulingEngine::new();
        let posterior = engine.bayesian_confidence(0.7, 0.8, 3);
        assert!(posterior < 0.8); // Contradictions should decrease confidence
    }

    #[test]
    fn test_bigram_match() {
        let engine = AgentSchedulingEngine::new();
        let score = engine.bigram_match_score("we need to review this code for bugs", &IntentCategory::CodeReview);
        assert!(score > 0.0);
    }

    #[test]
    fn test_trigram_exact_match() {
        let engine = AgentSchedulingEngine::new();
        let score = engine.trigram_match_score("can you fix this bug in the login", &IntentCategory::Debugging);
        assert!(score > 0.0);
    }

    #[test]
    fn test_urgency_critical() {
        let engine = AgentSchedulingEngine::new();
        let (urgency, label) = engine.estimate_urgency("production crash urgent hotfix immediately");
        assert!(urgency > 0.7);
        assert!(label.contains("紧急"));
    }

    #[test]
    fn test_urgency_normal() {
        let engine = AgentSchedulingEngine::new();
        let (urgency, label) = engine.estimate_urgency("write a function to sort arrays");
        assert!(urgency <= 0.4);
        assert!(label.contains("正常"));
    }

    #[test]
    fn test_analyze_enhanced() {
        let engine = AgentSchedulingEngine::new();
        let result = engine.analyze_enhanced("urgent: fix the security vulnerability in our API, production is down");
        let conf = result["fused_confidence"].as_str().unwrap();
        assert!(conf.contains("%"));
        assert_eq!(result["urgency"], "🔴 紧急");
    }
}

// ─── Tauri Commands ──────────────────────────────────────────────

#[tauri::command]
pub fn analyze_task(user_message: String) -> SchedulingResult {
    let engine = AgentSchedulingEngine::new();
    engine.analyze(&user_message)
}
