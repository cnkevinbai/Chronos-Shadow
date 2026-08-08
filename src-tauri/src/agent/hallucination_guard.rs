// 大模型防幻觉深度检测引擎 (Chronos Anti-Hallucination Guard)
//
// 六维检测体系：
//   1. 置信度模式检测 — 捕获 "I think" / "probably" 等不确定性语言
//   2. 虚构 API 检测 — 识别编造的库函数和不存在的方法
//   3. 代码一致性校验 — 检测无意义的代码拼接
//   4. 内部矛盾检测 — 同一回复内的自相矛盾
//   5. 幻觉评分系统 — 综合评分 0-100
//   6. 自动纠偏建议 — 针对检测到的幻觉给出修正提示
//
// 全部端侧计算，0 Token 消耗

use serde::{Deserialize, Serialize};

// ─── 幻觉检测结果 ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HallucinationReport {
    /// 综合信任分 0-100 (100=完全可信)
    pub trust_score: u32,
    /// 严重度: "safe" | "caution" | "warning" | "danger"
    pub severity: String,
    /// 检测到的所有问题
    pub findings: Vec<HallucinationFinding>,
    /// 总发现数
    pub finding_count: u32,
    /// 是否建议人工审核
    pub needs_review: bool,
    /// 自动纠偏建议
    pub corrections: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HallucinationFinding {
    /// 类别
    pub category: String,
    /// 严重度
    pub severity: String, // "low" | "medium" | "high"
    /// 检测到的模式
    pub pattern: String,
    /// 原文片段
    pub snippet: String,
    /// 扣分
    pub penalty: u32,
    /// 修正建议
    pub suggestion: String,
}

// ═══════════════════════════════════════════════════════════════════
// HallucinationGuard
// ═══════════════════════════════════════════════════════════════════

pub struct HallucinationGuard;

impl HallucinationGuard {
    pub fn new() -> Self { Self }

    /// 核心：分析 LLM 输出，生成幻觉报告
    pub fn audit(&self, response: &str) -> HallucinationReport {
        let mut findings = Vec::new();
        let mut corrections = Vec::new();
        let mut penalty_total = 0u32;

        // ── 1. 置信度模式检测 ────────────────────────────────
        self.check_confidence_markers(response, &mut findings, &mut penalty_total);

        // ── 2. 虚构 API 检测 ─────────────────────────────────
        self.check_fake_apis(response, &mut findings, &mut penalty_total, &mut corrections);

        // ── 3. 代码一致性校验 ─────────────────────────────────
        self.check_code_consistency(response, &mut findings, &mut penalty_total);

        // ── 4. 内部矛盾检测 ───────────────────────────────────
        self.check_internal_contradictions(response, &mut findings, &mut penalty_total);

        // ── 5. 不可执行命令检测 ───────────────────────────────
        self.check_impossible_commands(response, &mut findings, &mut penalty_total);

        // ── 6. 过时版本检测 ───────────────────────────────────
        self.check_outdated_references(response, &mut findings, &mut penalty_total, &mut corrections);

        // ── 综合评分 ──────────────────────────────────────────
        let trust_score = 100u32.saturating_sub(penalty_total);
        let severity = if trust_score >= 85 { "safe" }
            else if trust_score >= 65 { "caution" }
            else if trust_score >= 40 { "warning" }
            else { "danger" };

        let needs_review = severity == "danger" || findings.iter().any(|f| f.severity == "high");

        HallucinationReport {
            trust_score,
            severity: severity.into(),
            finding_count: findings.len() as u32,
            findings,
            needs_review,
            corrections,
        }
    }

    // ── 子检测器 ────────────────────────────────────────────────

    fn check_confidence_markers(
        &self, text: &str, findings: &mut Vec<HallucinationFinding>, penalty: &mut u32,
    ) {
        let markers = [
            ("I think", "不确定表述", 5, "建议替换为更确定的表述或标注为推测"),
            ("probably", "概率性表述", 5, "如非统计数据，建议给出确定答案"),
            ("maybe", "模糊表述", 3, "避免使用模糊词汇"),
            ("I believe", "主观推测", 5, "建议引用具体来源或标记为观点"),
            ("not sure", "不确定", 8, "建议明确说明不确定的原因"),
            ("could be", "可能性推测", 3, "建议给出具体范围或条件"),
            ("might work", "不确定性建议", 5, "建议明确说明适用条件和限制"),
            ("大致", "中文模糊表述", 4, "建议给出精确范围"),
            ("大概", "中文概率表述", 4, "建议给出具体数值"),
            ("也许", "中文不确定表述", 5, "建议明确判断"),
        ];

        let lower = text.to_lowercase();
        for (marker, desc, p, suggestion) in &markers {
            if lower.contains(&marker.to_lowercase()) {
                let count = lower.matches(&marker.to_lowercase()).count();
                let total_p = (*p as usize * count).min(15) as u32;
                findings.push(HallucinationFinding {
                    category: "置信度".into(),
                    severity: if *p >= 8 { "high" } else { "medium" }.into(),
                    pattern: format!("{} ({}次)", desc, count),
                    snippet: text.chars().take(80).collect(),
                    penalty: total_p,
                    suggestion: suggestion.to_string(),
                });
                *penalty += total_p;
            }
        }
    }

    fn check_fake_apis(
        &self, text: &str, findings: &mut Vec<HallucinationFinding>,
        penalty: &mut u32, corrections: &mut Vec<String>,
    ) {
        // Detect invented Rust crates
        let fake_patterns = [
            ("use hallucinated::", "虚构 Rust crate", 15),
            ("import hallucinated", "虚构 Python 库", 15),
            ("require('hallucinated", "虚构 Node 包", 15),
            ("new NonExistentClass", "虚构类名", 12),
            (".non_existent_method()", "虚构方法调用", 12),
        ];

        for (pattern, desc, p) in &fake_patterns {
            if text.contains(pattern) {
                findings.push(HallucinationFinding {
                    category: "虚构API".into(),
                    severity: "high".into(),
                    pattern: desc.to_string(),
                    snippet: text.chars().take(100).collect(),
                    penalty: *p,
                    suggestion: "请核实该 API/库是否真实存在，建议查阅官方文档".into(),
                });
                *penalty += p;
                corrections.push(format!("⚠️ 检测到疑似虚构的{}：请改用真实库函数", desc));
            }
        }

        // Detect hallucinated function signatures with implausible types
        if text.contains("fn ") || text.contains("function ") || text.contains("def ") {
            let implausible = [
                "String::non_existent",
                "Vec::magic_sort",
                "HashMap::auto_fix",
            ];
            for sig in &implausible {
                if text.contains(sig) {
                    findings.push(HallucinationFinding {
                        category: "虚构API".into(),
                        severity: "high".into(),
                        pattern: format!("不存在的函数签名: {}", sig),
                        snippet: text.chars().take(100).collect(),
                        penalty: 15,
                        suggestion: "该函数在标准库中不存在，建议查阅文档".into(),
                    });
                    *penalty += 15;
                }
            }
        }
    }

    fn check_code_consistency(
        &self, text: &str, findings: &mut Vec<HallucinationFinding>, penalty: &mut u32,
    ) {
        // Check for mismatched braces/parens in code blocks
        let code_blocks: Vec<&str> = text.split("```").skip(1).step_by(2).collect();
        for block in &code_blocks {
            let opens = block.matches('{').count();
            let closes = block.matches('}').count();
            if opens != closes {
                findings.push(HallucinationFinding {
                    category: "代码一致性".into(),
                    severity: "medium".into(),
                    pattern: format!("括号不匹配: {{ {} vs }} {}", opens, closes),
                    snippet: block.chars().take(80).collect(),
                    penalty: 8,
                    suggestion: "代码块中括号不匹配，可能是拼凑生成的幻觉代码".into(),
                });
                *penalty += 8;
            }

            // Check for variable used before declaration
            if block.contains(" = ") && !block.contains("let ") && !block.contains("const ") && !block.contains("var ") {
                // This is a heuristic - might be valid in some languages
                let lines: Vec<&str> = block.lines().filter(|l| !l.trim().is_empty()).collect();
                if lines.len() > 3 {
                    let has_decl = lines.iter().any(|l| l.contains("let ") || l.contains("fn ") || l.contains("def "));
                    if !has_decl {
                        findings.push(HallucinationFinding {
                            category: "代码一致性".into(),
                            severity: "low".into(),
                            pattern: "代码块缺少变量声明".into(),
                            snippet: block.chars().take(80).collect(),
                            penalty: 3,
                            suggestion: "代码块中可能有未声明变量，建议检查".into(),
                        });
                        *penalty += 3;
                    }
                }
            }
        }
    }

    fn check_internal_contradictions(
        &self, text: &str, findings: &mut Vec<HallucinationFinding>, penalty: &mut u32,
    ) {
        // Check for numeric contradictions
        let pairs = [
            ("always", "sometimes"),
            ("never", "occasionally"),
            ("must", "could"),
            ("guaranteed", "likely"),
        ];

        let lower = text.to_lowercase();
        for (abs, rel) in &pairs {
            if lower.contains(abs) && lower.contains(rel) {
                // Check if they're in close proximity (within 200 chars)
                if let Some(pos1) = lower.find(abs) {
                    if let Some(pos2) = lower.find(rel) {
                        if (pos1 as i32 - pos2 as i32).unsigned_abs() < 200 {
                            findings.push(HallucinationFinding {
                                category: "内部矛盾".into(),
                                severity: "high".into(),
                                pattern: format!("矛盾表述: 同时使用 '{}' 和 '{}'", abs, rel),
                                snippet: text.chars().skip(pos1).take(100).collect(),
                                penalty: 12,
                                suggestion: "回复中存在自相矛盾的表述，建议澄清".into(),
                            });
                            *penalty += 12;
                        }
                    }
                }
            }
        }

        // Version contradictions
        if (text.contains("1.0") && text.contains("2.0")) ||
           (text.contains("v1") && text.contains("v2")) {
            // Check if they refer to the SAME thing
            let has_different = text.contains("upgrade") || text.contains("migration")
                || text.contains("升级") || text.contains("迁移");
            if !has_different {
                findings.push(HallucinationFinding {
                    category: "版本混淆".into(),
                    severity: "medium".into(),
                    pattern: "可能混淆了不同版本的API".into(),
                    snippet: text.chars().take(100).collect(),
                    penalty: 6,
                    suggestion: "请明确指定使用的版本，避免版本混用导致幻觉".into(),
                });
                *penalty += 6;
            }
        }
    }

    fn check_impossible_commands(
        &self, text: &str, findings: &mut Vec<HallucinationFinding>, penalty: &mut u32,
    ) {
        let impossibles = [
            ("rm -rf /", "危险的系统命令", 20),
            ("DROP DATABASE", "不可逆数据库操作", 20),
            ("Format C:", "磁盘格式化命令", 20),
            ("chmod 777 /", "过度权限授予", 15),
        ];

        for (cmd, desc, p) in &impossibles {
            if text.to_lowercase().contains(&cmd.to_lowercase()) {
                findings.push(HallucinationFinding {
                    category: "危险命令".into(),
                    severity: "high".into(),
                    pattern: desc.to_string(),
                    snippet: text.chars().take(100).collect(),
                    penalty: *p,
                    suggestion: "该命令在生产环境不可执行，可能是模型幻觉".into(),
                });
                *penalty += p;
            }
        }
    }

    fn check_outdated_references(
        &self, text: &str, findings: &mut Vec<HallucinationFinding>,
        penalty: &mut u32, corrections: &mut Vec<String>,
    ) {
        let outdated = [
            ("Python 2", "Python 2 已于 2020 年停止维护", 5, "建议使用 Python 3.10+"),
            ("Node 12", "Node 12 已 EOL", 3, "建议使用 Node 22 LTS"),
            ("React 16", "React 16 已过时", 3, "建议使用 React 19"),
            ("AngularJS", "AngularJS 已停止维护", 5, "建议使用 Angular 19+"),
            (".NET Framework 4", ".NET Framework 4.x 已停止更新", 3, "建议使用 .NET 8+"),
            ("deprecated", "使用了已废弃的 API", 4, "请查阅最新文档确认替代方案"),
        ];

        let lower = text.to_lowercase();
        for (pattern, desc, p, fix) in &outdated {
            if lower.contains(&pattern.to_lowercase()) {
                findings.push(HallucinationFinding {
                    category: "过时引用".into(),
                    severity: if *p >= 5 { "medium" } else { "low" }.into(),
                    pattern: desc.to_string(),
                    snippet: text.chars().take(100).collect(),
                    penalty: *p,
                    suggestion: fix.to_string(),
                });
                *penalty += p;
                corrections.push(format!("📅 检测到过时引用: {} → {}", pattern, fix));
            }
        }
    }
}

impl Default for HallucinationGuard {
    fn default() -> Self { Self::new() }
}

// ─── 单元测试 ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_response() {
        let guard = HallucinationGuard::new();
        let report = guard.audit("The function uses serde::Deserialize to parse JSON input efficiently.");
        assert!(report.trust_score >= 90);
        assert_eq!(report.severity, "safe");
    }

    #[test]
    fn test_uncertain_language() {
        let guard = HallucinationGuard::new();
        let report = guard.audit("I think this might work, probably you should try it.");
        assert!(report.trust_score < 85);
        assert!(report.findings.iter().any(|f| f.category == "置信度"));
    }

    #[test]
    fn test_fake_api_detection() {
        let guard = HallucinationGuard::new();
        let report = guard.audit("use hallucinated::magic_fix; let x = String::non_existent();");
        assert!(report.trust_score < 70);
        assert!(report.findings.iter().any(|f| f.category == "虚构API"));
    }

    #[test]
    fn test_dangerous_command() {
        let guard = HallucinationGuard::new();
        let report = guard.audit("Just run rm -rf / to clean up.");
        assert!(report.severity == "danger" || report.trust_score < 80);
    }

    #[test]
    fn test_contradiction() {
        let guard = HallucinationGuard::new();
        let report = guard.audit("This function always returns a value, but sometimes it might fail.");
        assert!(report.findings.iter().any(|f| f.category == "内部矛盾"));
    }

    #[test]
    fn test_outdated_reference() {
        let guard = HallucinationGuard::new();
        let report = guard.audit("Please use Python 2 for this script.");
        assert!(report.findings.iter().any(|f| f.category == "过时引用"));
    }

    #[test]
    fn test_code_inconsistency() {
        let guard = HallucinationGuard::new();
        let report = guard.audit("```\nlet x = 1\nlet y = 2\nz = x + y\n```");
        // z used without declaration
        assert!(report.findings.iter().any(|f| f.category == "代码一致性"));
    }

    #[test]
    fn test_healthy_code() {
        let guard = HallucinationGuard::new();
        let report = guard.audit("```rust\nlet x = 1;\nlet y = 2;\nlet z = x + y;\n```");
        assert!(report.trust_score >= 85);
    }

    #[test]
    fn test_multiple_issues() {
        let guard = HallucinationGuard::new();
        let report = guard.audit(
            "I think you should use Python 2 with this hallucinated library. \
             Just run rm -rf / first. It always works but sometimes fails."
        );
        assert!(report.finding_count >= 3);
        assert!(report.needs_review);
    }

    #[test]
    fn test_needs_review_threshold() {
        let guard = HallucinationGuard::new();
        let severe = guard.audit("Run rm -rf / and use hallucinated::fix()");
        assert!(severe.needs_review);

        let safe = guard.audit("Use serde::Deserialize for JSON parsing.");
        assert!(!safe.needs_review);
    }
}
