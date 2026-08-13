// 大模型防幻觉深度检测引擎 (Chronos Anti-Hallucination Guard) v2
//
// 十维检测体系：
//   1. 置信度模式检测    — 捕获 "I think" / "probably" 等不确定性语言
//   2. 虚构 API 检测      — 识别编造的库函数和不存在的方法
//   3. 代码一致性校验    — 检测无意义的代码拼接
//   4. 内部矛盾检测      — 同一回复内的自相矛盾
//   5. 不可执行命令检测  — 危险的系统命令
//   6. 过时版本检测      — 已废弃的技术栈引用
//   7. 🆕 假编程检测     — 占位符代码/TODO桩/空函数体/伪代码
//   8. 🆕 假完成检测     — 虚报完成/无实质输出/承诺未兑现
//   9. 🆕 空文件夹检测   — mkdir无文件创建/空脚手架
//  10. 🆕 编造谎言检测   — 虚构数据/假基准测试/虚假版本号
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

/// 自适应阈值配置
pub struct GuardConfig {
    /// 最小置信度扣分 (避免过敏感)
    pub min_penalty: u32,
    /// 最大总扣分上限
    pub max_total_penalty: u32,
    /// 历史误报率 (用于校准)
    pub false_positive_rate: f32,
    /// 模型行为画像: 该模型的历史幻觉倾向 (0.0-1.0)
    pub model_hallucination_profile: f32,
}

impl Default for GuardConfig {
    fn default() -> Self {
        Self { min_penalty: 2, max_total_penalty: 80, false_positive_rate: 0.1, model_hallucination_profile: 0.5 }
    }
}

pub struct HallucinationGuard {
    config: GuardConfig,
    /// 历史检测统计: (总检测次数, 确认幻觉次数)
    detection_history: (u64, u64),
    /// 误报统计: (总报告数, 用户标记为误报数)
    false_positive_history: (u64, u64),
    /// 各维度检测灵敏度 (0.5-1.5, 1.0=默认)
    pub detection_sensitivity: HallucinationSensitivity,
}

/// 各维度自适应检测灵敏度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HallucinationSensitivity {
    pub confidence_markers: f64,   // 置信度检测
    pub fake_apis: f64,            // 虚构API
    pub code_consistency: f64,     // 代码一致性
    pub contradictions: f64,       // 内部矛盾
    pub impossible_commands: f64,  // 危险命令
    pub outdated_refs: f64,        // 过时引用
    pub fake_programming: f64,     // 假编程
    pub fake_completion: f64,      // 假完成
    pub empty_scaffold: f64,       // 空文件夹
    pub fabricated_facts: f64,     // 编造谎言
}

impl Default for HallucinationSensitivity {
    fn default() -> Self {
        Self {
            confidence_markers: 1.0,
            fake_apis: 1.0,
            code_consistency: 1.0,
            contradictions: 1.0,
            impossible_commands: 1.0,
            outdated_refs: 1.0,
            fake_programming: 1.0,
            fake_completion: 1.0,
            empty_scaffold: 1.0,
            fabricated_facts: 1.0,
        }
    }
}

impl HallucinationGuard {
    pub fn new() -> Self {
        Self {
            config: GuardConfig::default(),
            detection_history: (0, 0),
            false_positive_history: (0, 0),
            detection_sensitivity: HallucinationSensitivity::default(),
        }
    }

    /// 创建带模型画像的检测器
    pub fn with_profile(model_hallucination_rate: f32) -> Self {
        Self {
            config: GuardConfig { model_hallucination_profile: model_hallucination_rate, ..Default::default() },
            detection_history: (0, 0),
            false_positive_history: (0, 0),
            detection_sensitivity: HallucinationSensitivity::default(),
        }
    }

    // ── 进化反馈接口 ──────────────────────────────────────────

    /// 接收用户反馈：标记某次检测是否为误报
    pub fn record_feedback(&mut self, is_false_positive: bool) {
        self.detection_history.0 += 1;
        if !is_false_positive {
            self.detection_history.1 += 1; // 真正的幻觉
        } else {
            self.false_positive_history.0 += 1;
            self.false_positive_history.1 += 1;
        }

        // 自适应调整灵敏度
        let fp_rate = self.false_positive_rate();
        self.adapt_sensitivity(fp_rate);
    }

    /// 当前误报率
    pub fn false_positive_rate(&self) -> f64 {
        if self.false_positive_history.0 == 0 { return 0.1; }
        self.false_positive_history.1 as f64 / self.false_positive_history.0 as f64
    }

    /// 检测准确率
    pub fn accuracy(&self) -> f64 {
        if self.detection_history.0 == 0 { return 0.9; }
        self.detection_history.1 as f64 / self.detection_history.0 as f64
    }

    /// 自适应调整灵敏度：误报率高→降低灵敏度，漏报率高→提高灵敏度
    fn adapt_sensitivity(&mut self, fp_rate: f64) {
        let lr = 0.08;
        let target_fp = 0.15; // 目标误报率 15%

        let adjustment = if fp_rate > target_fp + 0.1 {
            -lr * 1.5 // 误报太多，降低灵敏度
        } else if fp_rate > target_fp {
            -lr * 0.5 // 轻微降低
        } else if fp_rate < 0.05 {
            lr * 1.0 // 太保守，提高灵敏度
        } else {
            0.0 // 在目标范围内
        };

        if adjustment != 0.0 {
            let s = &mut self.detection_sensitivity;
            let clamp = |v: f64| v.clamp(0.5, 1.5);
            s.confidence_markers = clamp(s.confidence_markers + adjustment);
            s.fake_apis = clamp(s.fake_apis + adjustment);
            s.fake_programming = clamp(s.fake_programming + adjustment);
            s.fake_completion = clamp(s.fake_completion + adjustment);
            s.fabricated_facts = clamp(s.fabricated_facts + adjustment);
            s.code_consistency = clamp(s.code_consistency + adjustment * 0.5);
        }
    }

    /// 应用灵敏度到惩罚值
    pub fn apply_sensitivity(&self, category: &str, base_penalty: u32) -> u32 {
        let factor = match category {
            "置信度" => self.detection_sensitivity.confidence_markers,
            "虚构API" => self.detection_sensitivity.fake_apis,
            "代码一致性" => self.detection_sensitivity.code_consistency,
            "内部矛盾" | "版本混淆" => self.detection_sensitivity.contradictions,
            "危险命令" => self.detection_sensitivity.impossible_commands,
            "过时引用" => self.detection_sensitivity.outdated_refs,
            "假编程" => self.detection_sensitivity.fake_programming,
            "假完成" => self.detection_sensitivity.fake_completion,
            "空文件夹" => self.detection_sensitivity.empty_scaffold,
            "编造谎言" => self.detection_sensitivity.fabricated_facts,
            _ => 1.0,
        };
        (base_penalty as f64 * factor).round() as u32
    }

    /// 获取进化指标 (供 EvolutionBus 使用)
    pub fn evolution_metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "accuracy": self.accuracy(),
            "false_positive_rate": self.false_positive_rate(),
            "total_detections": self.detection_history.0,
            "confirmed_hallucinations": self.detection_history.1,
            "sensitivity": {
                "confidence": self.detection_sensitivity.confidence_markers,
                "fake_apis": self.detection_sensitivity.fake_apis,
                "fake_programming": self.detection_sensitivity.fake_programming,
                "fabricated_facts": self.detection_sensitivity.fabricated_facts,
            },
        })
    }

    /// 自适应惩罚: 基于模型历史画像动态调整扣分力度
    fn adaptive_penalty(&self, base: u32, severity: &str) -> u32 {
        let profile_mult = if self.config.model_hallucination_profile > 0.7 { 1.5 }
            else if self.config.model_hallucination_profile < 0.3 { 0.7 }
            else { 1.0 };
        let severity_mult = match severity { "high" => 1.2, "medium" => 1.0, _ => 0.8 };
        let adjusted = (base as f32 * profile_mult * severity_mult) as u32;
        adjusted.max(self.config.min_penalty)
    }

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

        // ── 7. 假编程检测 ─────────────────────────────────────
        self.check_fake_programming(response, &mut findings, &mut penalty_total, &mut corrections);

        // ── 8. 假完成检测 ─────────────────────────────────────
        self.check_fake_completion(response, &mut findings, &mut penalty_total, &mut corrections);

        // ── 9. 空文件夹检测 ───────────────────────────────────
        self.check_empty_scaffold(response, &mut findings, &mut penalty_total, &mut corrections);

        // ── 10. 编造谎言检测 ──────────────────────────────────
        self.check_fabricated_facts(response, &mut findings, &mut penalty_total, &mut corrections);

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
                let base_p = (*p as usize * count).min(15) as u32;
                let total_p = self.adaptive_penalty(base_p, if *p >= 8 { "high" } else { "medium" });
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

    // ── 7. 假编程检测 ─────────────────────────────────────────

    fn check_fake_programming(
        &self, text: &str, findings: &mut Vec<HallucinationFinding>,
        penalty: &mut u32, corrections: &mut Vec<String>,
    ) {
        let lower = text.to_lowercase();

        // 占位符代码模式: // TODO, // FIXME, // implement later, # stub, pass
        let placeholders = [
            ("// todo", "TODO 占位符", 8),
            ("// fixme", "FIXME 未修复标记", 10),
            ("// implement later", "延后实现的占位", 12),
            ("// stub", "桩代码占位", 12),
            ("# stub", "Python 桩代码", 12),
            ("pass  # TODO", "空实现占位", 10),
            ("unimplemented!()", "Rust 未实现宏", 15),
            ("todo!()", "Rust 未完成宏", 12),
            ("throw new Error(\"Not implemented\")", "未实现异常", 12),
            ("raise NotImplementedError", "Python 未实现异常", 12),
        ];

        for (pattern, desc, p) in &placeholders {
            if lower.contains(pattern) {
                let count = lower.matches(pattern).count();
                let total_p = (*p as usize * count).min(20) as u32;
                findings.push(HallucinationFinding {
                    category: "假编程".into(),
                    severity: if *p >= 12 { "high" } else { "medium" }.into(),
                    pattern: format!("{} ({}处)", desc, count),
                    snippet: text.chars().take(120).collect(),
                    penalty: total_p,
                    suggestion: "检测到占位符代码，模型可能未真正完成编程任务。请要求提供完整可运行代码".into(),
                });
                *penalty += total_p;
                corrections.push(format!("⚠️ 假编程风险: 发现 {} 处 {}，代码可能无法运行", count, desc));
            }
        }

        // 伪代码模式: 用自然语言描述代替真实代码
        let pseudocode_markers = [
            "pseudocode", "pseudo code", "// ... rest of implementation",
            "// similar to above", "// same pattern continues",
            "/* ... */", "<-- insert logic here -->",
        ];
        for marker in &pseudocode_markers {
            if lower.contains(&marker.to_lowercase()) {
                findings.push(HallucinationFinding {
                    category: "假编程".into(),
                    severity: "high".into(),
                    pattern: format!("伪代码标记: {}", marker),
                    snippet: text.chars().take(120).collect(),
                    penalty: 14,
                    suggestion: "检测到伪代码标记，模型用自然语言替代了真实代码实现".into(),
                });
                *penalty += 14;
                corrections.push(format!("⚠️ 伪代码风险: 发现 '{}' 标记，请要求提供真实可运行的代码", marker));
                break; // 一处伪代码标记即可
            }
        }

        // 空函数体 / 空类: fn name() { } 或 def name(): pass
        let empty_fn_patterns = [
            ("fn ", "{ }", "Rust 空函数体"),
            ("fn ", "{}", "Rust 空函数体(无空格)"),
            (":\n    pass", "", "Python 空函数 pass"),
            ("function ", "{}", "JavaScript 空函数"),
            ("=> {}", "", "箭头函数空体"),
        ];
        for (prefix, suffix, desc) in &empty_fn_patterns {
            if lower.contains(&prefix.to_lowercase()) && lower.contains(&suffix.to_lowercase()) {
                findings.push(HallucinationFinding {
                    category: "假编程".into(),
                    severity: "high".into(),
                    pattern: format!("空函数体: {}", desc),
                    snippet: text.chars().take(120).collect(),
                    penalty: 15,
                    suggestion: "检测到空函数体，模型声称编写了代码但实际为空实现".into(),
                });
                *penalty += 15;
                corrections.push(format!("⚠️ 空函数体: {}", desc));
                break;
            }
        }
    }

    // ── 8. 假完成检测 ─────────────────────────────────────────

    fn check_fake_completion(
        &self, text: &str, findings: &mut Vec<HallucinationFinding>,
        penalty: &mut u32, corrections: &mut Vec<String>,
    ) {
        let lower = text.to_lowercase();

        // 虚报完成模式: "Done!" 但无代码/无实质输出
        let done_patterns = [
            "done!", "all done!", "completed!", "finished!",
            "task completed", "everything is set up",
            "已完成", "全部完成", "大功告成",
        ];

        let has_code = text.contains("```") || text.contains("fn ") || text.contains("def ")
            || text.contains("function ") || text.contains("class ") || text.contains("import ");
        let has_file_list = text.contains(".rs") || text.contains(".ts") || text.contains(".py")
            || text.contains(".js") || text.contains(".tsx");

        let has_substance = has_code || has_file_list || text.len() > 500;

        for pattern in &done_patterns {
            if lower.contains(pattern) && !has_substance {
                findings.push(HallucinationFinding {
                    category: "假完成".into(),
                    severity: "high".into(),
                    pattern: format!("虚报完成: '{}' 但无实质代码/文件输出", pattern),
                    snippet: text.chars().take(120).collect(),
                    penalty: 18,
                    suggestion: "模型声称任务完成但未提供任何代码或文件，这是典型的假完成幻觉".into(),
                });
                *penalty += 18;
                corrections.push(format!("🚨 假完成风险: 模型回复 '{}' 但无实质输出，请要求提供具体代码/文件", pattern));
                break;
            }
        }

        // 过度承诺但无内容: "I will create..." / "Let me implement..." 后面无代码
        let promise_prefixes = [
            "i will create", "i will implement", "i'll write", "let me build",
            "i'm going to code", "我将创建", "我来实现", "我为你编写",
        ];
        for prefix in &promise_prefixes {
            if lower.contains(prefix) && !has_code && text.len() < 300 {
                findings.push(HallucinationFinding {
                    category: "假完成".into(),
                    severity: "medium".into(),
                    pattern: format!("空头承诺: '{}' 后无实际代码", prefix),
                    snippet: text.chars().take(120).collect(),
                    penalty: 10,
                    suggestion: "模型做出承诺但未交付代码，可能是假完成幻觉".into(),
                });
                *penalty += 10;
                corrections.push(format!("⚠️ 空头承诺: 声称 '{}' 但未提供代码", prefix));
                break;
            }
        }
    }

    // ── 9. 空文件夹检测 ───────────────────────────────────────

    fn check_empty_scaffold(
        &self, text: &str, findings: &mut Vec<HallucinationFinding>,
        penalty: &mut u32, corrections: &mut Vec<String>,
    ) {
        let lower = text.to_lowercase();

        // mkdir 创建目录但无对应的文件创建
        let has_mkdir = lower.contains("mkdir ") || lower.contains("create_dir")
            || lower.contains("fs::create_dir") || lower.contains("os.makedirs");
        let has_file_create = lower.contains("touch ") || lower.contains("write")
            || lower.contains("fs::write") || lower.contains("echo ") && lower.contains("> ");

        if has_mkdir && !has_file_create {
            findings.push(HallucinationFinding {
                category: "空文件夹".into(),
                severity: "high".into(),
                pattern: "仅创建目录但无文件写入".into(),
                snippet: text.chars().take(120).collect(),
                penalty: 16,
                suggestion: "检测到 mkdir 但无对应文件创建，模型可能在构建空脚手架而非真实项目".into(),
            });
            *penalty += 16;
            corrections.push("🚨 空文件夹风险: 只创建了目录结构但没有任何文件内容，这是典型的空壳交付".into());
        }

        // 脚手架生成但全是空文件
        let scaffold_markers = ["create the following structure", "project structure:",
            "file tree:", "目录结构:", "项目结构:"];
        for marker in &scaffold_markers {
            if lower.contains(&marker.to_lowercase()) && !has_file_create && text.len() < 400 {
                findings.push(HallucinationFinding {
                    category: "空文件夹".into(),
                    severity: "medium".into(),
                    pattern: format!("空脚手架: 描述了 '{}' 但无文件内容", marker),
                    snippet: text.chars().take(120).collect(),
                    penalty: 10,
                    suggestion: "模型描述了目录结构但所有文件为空，这是空壳项目".into(),
                });
                *penalty += 10;
                corrections.push("⚠️ 空壳交付: 描述了项目结构但未提供文件内容".into());
                break;
            }
        }
    }

    // ── 10. 编造谎言检测 ─────────────────────────────────────

    fn check_fabricated_facts(
        &self, text: &str, findings: &mut Vec<HallucinationFinding>,
        penalty: &mut u32, corrections: &mut Vec<String>,
    ) {
        let lower = text.to_lowercase();

        // 虚构基准测试数据: "benchmark shows", "performance improved by X%", "X% faster"
        let bench_patterns = [
            ("benchmark shows", "虚构基准测试"),
            ("performance improved by", "虚假性能数据"),
            ("reduces latency by", "虚假延迟数据"),
            ("improves throughput by", "虚假吞吐量数据"),
            ("% faster than", "虚假速度对比"),
            ("% reduction in", "虚假指标改善"),
            ("测试显示性能提升", "中文虚假性能声明"),
        ];
        for (pattern, desc) in &bench_patterns {
            if lower.contains(pattern) {
                // 检查是否有实际数据来源引用
                let has_source = lower.contains("according to") || lower.contains("based on")
                    || lower.contains("来源") || lower.contains("参考") || lower.contains("https://");
                if !has_source {
                    findings.push(HallucinationFinding {
                        category: "编造谎言".into(),
                        severity: "high".into(),
                        pattern: format!("{}: 无来源引用的性能声明", desc),
                        snippet: text.chars().take(120).collect(),
                        penalty: 15,
                        suggestion: "模型声称有性能数据但未提供来源，可能是编造的基准测试结果".into(),
                    });
                    *penalty += 15;
                    corrections.push(format!("🚨 虚构数据: '{}' 无引用来源，可能是编造的性能数据", desc));
                    break;
                }
            }
        }

        // 编造版本号: "version 5.0" 但实际不存在
        let fake_version_patterns = [
            "react 20", "react 21", "vue 4", "vue 5", "angular 20",
            "rust 2.0", "python 4", "python 5",
            "typescript 6", "typescript 7",
            "node 25", "node 30",
            "webpack 6", "vite 6",
        ];
        for pattern in &fake_version_patterns {
            if lower.contains(pattern) {
                findings.push(HallucinationFinding {
                    category: "编造谎言".into(),
                    severity: "high".into(),
                    pattern: format!("虚构版本号: {}", pattern),
                    snippet: text.chars().take(120).collect(),
                    penalty: 18,
                    suggestion: format!("'{}' 版本不存在，模型编造了不存在的技术版本", pattern).into(),
                });
                *penalty += 18;
                corrections.push(format!("🚨 虚构版本: '{}' 不存在，请查阅官方版本发布记录", pattern));
                break;
            }
        }

        // 虚假声明检测: "it is proven", "studies show", "research indicates"
        let fake_authority = [
            ("it is proven that", "虚假权威声明"),
            ("studies show that", "无引用研究声称"),
            ("research indicates", "无来源研究引用"),
            ("according to experts", "虚假专家背书"),
            ("业界公认", "无来源共识声称"),
            ("实践证明", "无证据实践声明"),
            ("大量研究表明", "虚假文献引用"),
        ];
        for (pattern, desc) in &fake_authority {
            if lower.contains(pattern) {
                let has_citation = text.contains("http") || text.contains("doi:")
                    || text.contains("arXiv") || text.contains("(") && text.contains(")")
                    && text.contains("20"); // 年份引用
                if !has_citation {
                    findings.push(HallucinationFinding {
                        category: "编造谎言".into(),
                        severity: "medium".into(),
                        pattern: format!("{}: '{}'", desc, pattern),
                        snippet: text.chars().take(120).collect(),
                        penalty: 12,
                        suggestion: "模型使用了权威性语言但未提供引用来源，可能是编造的背书".into(),
                    });
                    *penalty += 12;
                    corrections.push(format!("⚠️ 虚假背书: 使用了 '{}' 但无引用来源", pattern));
                    break;
                }
            }
        }

        // 编造具体数字: 精确到小数点的统计数字但无来源
        let mut fake_stats_count = 0u32;
        for word in text.split_whitespace() {
            let trimmed = word.trim_end_matches(&['.', ',', ';', ':', ')', '】', '）']);
            if let Some(pct_pos) = trimmed.find('%') {
                let before_pct = &trimmed[..pct_pos];
                if before_pct.contains('.') && before_pct.chars().filter(|c| c.is_ascii_digit()).count() >= 2 {
                    fake_stats_count += 1;
                }
            }
        }
        if fake_stats_count >= 3 {
            findings.push(HallucinationFinding {
                category: "编造谎言".into(),
                severity: "medium".into(),
                pattern: format!("疑似编造统计数据: 多处精确百分比({}处)", fake_stats_count),
                snippet: text.chars().take(120).collect(),
                penalty: 10,
                suggestion: "多处精确统计数据未注明来源，可能是模型编造的幻觉数据".into(),
            });
            *penalty += 10;
            corrections.push("⚠️ 可疑数据: 多处精确百分比无来源引用，可能为编造".into());
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

    // ── 假编程检测测试 ──────────────────────────────────────────

    #[test]
    fn test_fake_programming_todo() {
        let guard = HallucinationGuard::new();
        let report = guard.audit("fn main() {\n    // TODO: implement this later\n    unimplemented!();\n}");
        assert!(report.findings.iter().any(|f| f.category == "假编程"));
        assert!(report.trust_score < 80);
    }

    #[test]
    fn test_fake_programming_pseudocode() {
        let guard = HallucinationGuard::new();
        let report = guard.audit("Here is the pseudocode for the algorithm:\n// similar to above pattern");
        assert!(report.findings.iter().any(|f| f.category == "假编程"));
    }

    #[test]
    fn test_fake_programming_empty_fn() {
        let guard = HallucinationGuard::new();
        let report = guard.audit("fn process_data() { }");
        assert!(report.findings.iter().any(|f| f.category == "假编程"));
    }

    // ── 假完成检测测试 ──────────────────────────────────────────

    #[test]
    fn test_fake_completion() {
        let guard = HallucinationGuard::new();
        let report = guard.audit("Done! Everything is set up.");
        assert!(report.findings.iter().any(|f| f.category == "假完成"));
        assert!(report.trust_score < 85);
    }

    #[test]
    fn test_real_completion_with_code() {
        let guard = HallucinationGuard::new();
        let report = guard.audit("Done! Here is the code:\n```rust\nfn main() {}\n```");
        // 有实质代码输出，不应被标记为假完成
        assert!(!report.findings.iter().any(|f| f.category == "假完成"));
    }

    // ── 空文件夹检测测试 ────────────────────────────────────────

    #[test]
    fn test_empty_scaffold() {
        let guard = HallucinationGuard::new();
        let report = guard.audit("Create the directories:\nmkdir src\nmkdir tests\nThat's it!");
        assert!(report.findings.iter().any(|f| f.category == "空文件夹"));
    }

    #[test]
    fn test_scaffold_with_files() {
        let guard = HallucinationGuard::new();
        let report = guard.audit("mkdir src && echo 'fn main() {}' > src/main.rs");
        // 有文件创建，不应标记为空文件夹
        assert!(!report.findings.iter().any(|f| f.category == "空文件夹"));
    }

    // ── 编造谎言检测测试 ────────────────────────────────────────

    #[test]
    fn test_fake_benchmark() {
        let guard = HallucinationGuard::new();
        let report = guard.audit("Our benchmark shows a 45.2% performance improvement over the previous version.");
        assert!(report.findings.iter().any(|f| f.category == "编造谎言"));
    }

    #[test]
    fn test_fake_version() {
        let guard = HallucinationGuard::new();
        let report = guard.audit("You should upgrade to React 21 for the best experience.");
        assert!(report.findings.iter().any(|f| f.category == "编造谎言"));
    }

    #[test]
    fn test_fake_authority() {
        let guard = HallucinationGuard::new();
        let report = guard.audit("Studies show that this approach is the best. According to experts, you should use it.");
        assert!(report.findings.iter().any(|f| f.category == "编造谎言"));
    }

    #[test]
    fn test_fake_stats_percentage() {
        let guard = HallucinationGuard::new();
        let report = guard.audit("The results show: 45.2% faster, 33.7% less memory, 21.5% fewer bugs.");
        assert!(report.findings.iter().any(|f| f.category == "编造谎言"));
    }
}

// ─── Tauri Commands ──────────────────────────────────────────────

#[tauri::command]
pub fn audit_hallucination(response: String) -> HallucinationReport {
    let guard = HallucinationGuard::new();
    guard.audit(&response)
}
