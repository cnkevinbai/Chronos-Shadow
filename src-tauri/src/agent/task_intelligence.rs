// 任务智能引擎 (Task Intelligence Engine)
//
// 核心功能：
//   1. 智能任务分解 — 复杂任务自动拆解为原子子任务 + 依赖关系图
//   2. 并行执行规划 — 识别无依赖子任务并行执行，最大化吞吐
//   3. 复杂度估算 — 基于多维度特征估算任务难度和所需资源
//   4. 自动 Agent 匹配 — 为每个子任务推荐最优 Agent 角色
//   5. 进度追踪 — 实时子任务完成状态 + ETA 预估
//
// 设计原则：
//   1. 自顶向下分解 — 从粗粒度目标逐层分解到原子操作
//   2. 依赖最小化 — 尽可能减少子任务间依赖，提升并行度
//   3. 资源感知 — 估算每个子任务所需 token/时间/成本
//   4. 可中断可恢复 — 支持暂停/恢复/重试

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// ─── 任务复杂度等级 ────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ComplexityLevel {
    Trivial = 1,    // 单步操作
    Simple = 2,     // 2-3 步
    Moderate = 3,   // 4-7 步
    Complex = 4,    // 8-15 步
    VeryComplex = 5, // 15+ 步，需多轮协作
}

impl ComplexityLevel {
    pub fn label(&self) -> &str {
        match self {
            Self::Trivial => "简单",
            Self::Simple => "基础",
            Self::Moderate => "中等",
            Self::Complex => "复杂",
            Self::VeryComplex => "极复杂",
        }
    }

    pub fn estimated_steps(&self) -> usize {
        match self {
            Self::Trivial => 1,
            Self::Simple => 3,
            Self::Moderate => 6,
            Self::Complex => 12,
            Self::VeryComplex => 20,
        }
    }
}

impl std::fmt::Display for ComplexityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

// ─── 任务类型分类 ──────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskCategory {
    CodeImplementation,
    BugFix,
    FeatureDesign,
    Refactoring,
    Testing,
    Documentation,
    Deployment,
    Research,
    DataProcessing,
    SecurityAudit,
}

impl TaskCategory {
    pub fn label(&self) -> &str {
        match self {
            Self::CodeImplementation => "代码实现",
            Self::BugFix => "Bug修复",
            Self::FeatureDesign => "功能设计",
            Self::Refactoring => "代码重构",
            Self::Testing => "测试编写",
            Self::Documentation => "文档编写",
            Self::Deployment => "部署上线",
            Self::Research => "技术调研",
            Self::DataProcessing => "数据处理",
            Self::SecurityAudit => "安全审计",
        }
    }
}

impl std::fmt::Display for TaskCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

// ─── 原子子任务 ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    pub id: String,
    pub title: String,
    pub description: String,
    /// 依赖的前置子任务 ID
    pub dependencies: Vec<String>,
    /// 推荐的 Agent 角色
    pub recommended_agent: String,
    /// 推荐模型
    pub recommended_model: String,
    /// 估算 token 消耗
    pub estimated_tokens: u32,
    /// 估算耗时 (秒)
    pub estimated_duration_secs: u64,
    /// 优先级 (0=最高)
    pub priority: u8,
    /// 执行状态
    pub status: SubTaskStatus,
    /// 分类
    pub category: TaskCategory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubTaskStatus {
    Pending,
    Ready,       // 依赖已满足，可执行
    InProgress,
    Completed,
    Failed(String),
    Skipped,
}

// ─── 任务分解计划 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    pub task_id: String,
    pub original_task: String,
    pub complexity: ComplexityLevel,
    pub category: TaskCategory,
    pub sub_tasks: Vec<SubTask>,
    pub parallel_groups: Vec<Vec<String>>,  // 可并行执行的子任务组
    pub total_estimated_tokens: u64,
    pub total_estimated_duration_secs: u64,
    pub estimated_cost: f64,
    pub created_at: String,
}

/// 风险等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Low => "低",
            Self::Medium => "中",
            Self::High => "高",
            Self::Critical => "极高",
        }
    }
}

/// 工作量估算（v2：PERT 三点估算 + 风险加权 + 关键路径）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffortEstimate {
    pub optimistic_secs: u64,
    pub pessimistic_secs: u64,
    pub expected_secs: u64,
    pub risk_level: RiskLevel,
    pub risk_score: f64,
    pub risk_factors: Vec<String>,
    pub critical_path_secs: u64,
    pub tokens_estimate: u64,
    pub cost_estimate: f64,
}

// ─── 分解模板 ──────────────────────────────────────────────────────

struct DecompositionTemplate {
    category: TaskCategory,
    pattern: Vec<(&'static str, &'static str, &'static str, Vec<usize>)>,
    // (title, description, agent, dependency_indices)
}

// ─── 任务智能引擎 ──────────────────────────────────────────────────

pub struct TaskIntelligenceEngine {
    /// 分解模板库
    templates: Vec<DecompositionTemplate>,
}

impl TaskIntelligenceEngine {
    pub fn new() -> Self {
        let templates = vec![
            // ── 代码实现模板 ──
            DecompositionTemplate {
                category: TaskCategory::CodeImplementation,
                pattern: vec![
                    ("需求分析", "理解功能需求，明确输入输出和边界条件", "PM", vec![]),
                    ("技术方案设计", "确定技术选型、数据结构、接口设计", "Architect", vec![0]),
                    ("编写核心逻辑", "实现主要业务逻辑代码", "Coder", vec![1]),
                    ("编写单元测试", "为核心逻辑编写测试用例", "Tester", vec![2]),
                    ("代码审查", "Review 代码质量和规范", "Reviewer", vec![2, 3]),
                    ("集成验证", "运行全量测试，确认无回归", "Verifier", vec![3, 4]),
                ],
            },
            // ── Bug修复模板 ──
            DecompositionTemplate {
                category: TaskCategory::BugFix,
                pattern: vec![
                    ("问题复现", "编写最小复现用例，定位 Bug 根因", "Coder", vec![]),
                    ("根因分析", "分析代码逻辑，确定修复方案", "Coder", vec![0]),
                    ("实施修复", "编写修复代码", "Coder", vec![1]),
                    ("回归测试", "运行相关测试确保修复有效", "Tester", vec![2]),
                    ("审查确认", "Review 修复方案的正确性", "Reviewer", vec![2, 3]),
                ],
            },
            // ── 功能设计模板 ──
            DecompositionTemplate {
                category: TaskCategory::FeatureDesign,
                pattern: vec![
                    ("需求调研", "收集用户需求和使用场景", "PM", vec![]),
                    ("竞品分析", "分析同类产品实现方案", "PM", vec![0]),
                    ("PRD编制", "编写产品需求文档", "PM", vec![1]),
                    ("UI原型设计", "设计用户界面原型", "UIDesigner", vec![2]),
                    ("技术架构设计", "确定后端架构和数据模型", "Architect", vec![2]),
                    ("API接口设计", "定义 REST/GraphQL 接口规范", "Architect", vec![4]),
                ],
            },
            // ── 重构模板 ──
            DecompositionTemplate {
                category: TaskCategory::Refactoring,
                pattern: vec![
                    ("代码审计", "分析现有代码结构和坏味道", "Auditor", vec![]),
                    ("重构方案设计", "制定分步重构计划", "Architect", vec![0]),
                    ("提取模块接口", "定义新模块边界和接口", "Coder", vec![1]),
                    ("迁移核心逻辑", "将旧代码迁移到新模块", "Coder", vec![2]),
                    ("更新测试", "适配测试到新模块结构", "Tester", vec![3]),
                    ("验证一致性", "对比迁移前后行为一致性", "Verifier", vec![3, 4]),
                ],
            },
            // ── 测试模板 ──
            DecompositionTemplate {
                category: TaskCategory::Testing,
                pattern: vec![
                    ("测试策略制定", "确定测试范围和覆盖目标", "Tester", vec![]),
                    ("单元测试编写", "为核心函数编写单元测试", "Tester", vec![0]),
                    ("集成测试编写", "编写模块间集成测试", "Tester", vec![1]),
                    ("E2E测试编写", "编写端到端流程测试", "Tester", vec![2]),
                    ("测试报告", "生成覆盖率报告和改进建议", "Tester", vec![1, 2, 3]),
                ],
            },
            // ── 技术调研模板 ──
            DecompositionTemplate {
                category: TaskCategory::Research,
                pattern: vec![
                    ("问题定义", "明确调研目标和评估标准", "PM", vec![]),
                    ("信息搜集", "搜索官方文档、技术博客、社区讨论", "Scout", vec![0]),
                    ("方案对比", "对比候选方案的优缺点", "Architect", vec![1]),
                    ("原型验证", "搭建最小可行原型验证关键假设", "Coder", vec![2]),
                    ("调研报告", "编写调研结论和推荐方案", "PM", vec![2, 3]),
                ],
            },
            // ── 安全审计模板 ──
            DecompositionTemplate {
                category: TaskCategory::SecurityAudit,
                pattern: vec![
                    ("资产梳理", "列出需要审计的代码模块和依赖", "Auditor", vec![]),
                    ("静态分析", "运行 SAST 工具扫描漏洞", "Auditor", vec![0]),
                    ("依赖审计", "检查第三方依赖的已知漏洞", "Auditor", vec![0]),
                    ("手动审查", "人工审查关键路径代码", "Auditor", vec![1, 2]),
                    ("渗透测试", "对关键接口进行渗透测试", "Auditor", vec![3]),
                    ("审计报告", "汇总发现的风险和建议修复方案", "Auditor", vec![1, 2, 3, 4]),
                ],
            },
        ];

        Self { templates }
    }

    // ── 复杂度估算 ────────────────────────────────────────────

    /// 基于多维度特征估算任务复杂度
    pub fn estimate_complexity(&self, task: &str) -> (ComplexityLevel, f64) {
        let lower = task.to_lowercase();
        let len = task.chars().count();
        let mut score = 0.0f64;

        // 维度1: 任务描述长度
        if len > 500 { score += 1.5; }
        else if len > 200 { score += 1.0; }
        else if len > 50 { score += 0.5; }

        // 维度2: 关键词复杂度信号
        let complex_signals = [
            ("architecture", 1.5), ("refactor", 1.5), ("migrate", 1.5),
            ("security", 1.5), ("performance", 1.0), ("multi", 1.0),
            ("full-stack", 1.5), ("end-to-end", 1.5), ("pipeline", 1.0),
            ("架构", 1.5), ("重构", 1.5), ("迁移", 1.5), ("安全", 1.5),
            ("性能优化", 1.5), ("全栈", 1.5), ("微服务", 2.0),
            ("distributed", 2.0), ("concurrent", 1.5), ("async", 1.0),
        ];
        for (signal, weight) in &complex_signals {
            if lower.contains(signal) {
                score += weight;
            }
        }

        // 维度3: 技术栈多样性
        let tech_keywords = ["rust", "typescript", "python", "react", "api", "database",
            "docker", "kubernetes", "sql", "graphql", "redis", "kafka"];
        let tech_count = tech_keywords.iter().filter(|k| lower.contains(*k)).count();
        score += (tech_count as f64 * 0.3).min(1.5);

        // 维度4: 是否涉及多个领域
        let domain_signals = ["frontend", "backend", "database", "devops", "security", "mobile"];
        let domain_count = domain_signals.iter().filter(|k| lower.contains(*k)).count();
        score += (domain_count as f64 * 0.4).min(2.0);

        let level = if score >= 4.0 { ComplexityLevel::VeryComplex }
            else if score >= 2.5 { ComplexityLevel::Complex }
            else if score >= 1.5 { ComplexityLevel::Moderate }
            else if score >= 0.5 { ComplexityLevel::Simple }
            else { ComplexityLevel::Trivial };

        let confidence = (score / 5.0).min(0.95);
        (level, confidence)
    }

    /// 分类任务
    pub fn categorize(&self, task: &str) -> TaskCategory {
        let lower = task.to_lowercase();
        let mut scores: HashMap<&str, f64> = HashMap::new();

        let signals = [
            ("implement,write,code,generate,build,create,编写,实现,生成,创建,构建", "code"),
            ("bug,fix,error,crash,broken,修复,bug,错误,崩溃", "bug"),
            ("design,feature,plan,设计,功能,规划", "feature"),
            ("refactor,rewrite,cleanup,restructure,重构,重写,清理", "refactor"),
            ("test,coverage,spec,测试,覆盖率", "test"),
            ("docs,readme,document,文档,注释,说明", "docs"),
            ("deploy,release,ci,cd,docker,部署,发布", "deploy"),
            ("research,compare,evaluate,调研,对比,评估", "research"),
            ("data,analyze,process,数据,分析,处理", "data"),
            ("audit,security,vulnerability,审计,安全,漏洞", "security"),
        ];

        for (keywords, cat) in &signals {
            for kw in keywords.split(',') {
                if lower.contains(kw.trim()) {
                    *scores.entry(cat).or_insert(0.0) += 1.0;
                }
            }
        }

        scores.into_iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(cat, _)| match cat {
                "code" => TaskCategory::CodeImplementation,
                "bug" => TaskCategory::BugFix,
                "feature" => TaskCategory::FeatureDesign,
                "refactor" => TaskCategory::Refactoring,
                "test" => TaskCategory::Testing,
                "docs" => TaskCategory::Documentation,
                "deploy" => TaskCategory::Deployment,
                "research" => TaskCategory::Research,
                "data" => TaskCategory::DataProcessing,
                "security" => TaskCategory::SecurityAudit,
                _ => TaskCategory::CodeImplementation,
            })
            .unwrap_or(TaskCategory::CodeImplementation)
    }

    // ── 智能任务分解 ────────────────────────────────────────

    /// 将复杂任务分解为原子子任务
    pub fn decompose(&self, task: &str) -> TaskPlan {
        let (complexity, _) = self.estimate_complexity(task);
        let category = self.categorize(task);
        let now = chrono::Utc::now().to_rfc3339();

        // 匹配模板
        let template = self.templates.iter()
            .find(|t| t.category == category);

        let sub_tasks: Vec<SubTask> = if let Some(tpl) = template {
            tpl.pattern.iter().enumerate().map(|(i, (title, desc, agent, deps))| {
                SubTask {
                    id: format!("sub-{:02}", i + 1),
                    title: title.to_string(),
                    description: desc.to_string(),
                    dependencies: deps.iter().map(|d| format!("sub-{:02}", d + 1)).collect(),
                    recommended_agent: agent.to_string(),
                    recommended_model: self.recommend_model_for_agent(agent),
                    estimated_tokens: self.estimate_tokens(title, desc),
                    estimated_duration_secs: self.estimate_duration(title),
                    priority: i as u8,
                    status: if deps.is_empty() { SubTaskStatus::Ready } else { SubTaskStatus::Pending },
                    category: category.clone(),
                }
            }).collect()
        } else {
            // 无匹配模板 → 生成通用分解
            let steps = complexity.estimated_steps();
            (0..steps).map(|i| {
                SubTask {
                    id: format!("sub-{:02}", i + 1),
                    title: format!("步骤 {}", i + 1),
                    description: format!("执行任务的第 {} 步", i + 1),
                    dependencies: if i > 0 { vec![format!("sub-{:02}", i)] } else { vec![] },
                    recommended_agent: "Coder".into(),
                    recommended_model: "deepseek-v4-flash".into(),
                    estimated_tokens: 500,
                    estimated_duration_secs: 30,
                    priority: i as u8,
                    status: if i == 0 { SubTaskStatus::Ready } else { SubTaskStatus::Pending },
                    category: TaskCategory::CodeImplementation,
                }
            }).collect()
        };

        // 识别并行执行组
        let parallel_groups = self.find_parallel_groups(&sub_tasks);

        let total_tokens: u64 = sub_tasks.iter().map(|s| s.estimated_tokens as u64).sum();
        let total_duration: u64 = parallel_groups.iter()
            .map(|group| {
                group.iter()
                    .filter_map(|id| sub_tasks.iter().find(|s| &s.id == id))
                    .map(|s| s.estimated_duration_secs)
                    .max()
                    .unwrap_or(0)
            })
            .sum();
        let estimated_cost = total_tokens as f64 * 0.000001; // ~¥1/1M tokens

        TaskPlan {
            task_id: format!("plan-{}", chrono::Utc::now().timestamp_millis()),
            original_task: task.to_string(),
            complexity,
            category,
            sub_tasks,
            parallel_groups,
            total_estimated_tokens: total_tokens,
            total_estimated_duration_secs: total_duration,
            estimated_cost,
            created_at: now,
        }
    }

    // ── 并行组检测 ────────────────────────────────────────

    /// 使用拓扑排序识别可并行执行的子任务组
    fn find_parallel_groups(&self, sub_tasks: &[SubTask]) -> Vec<Vec<String>> {
        let mut groups: Vec<Vec<String>> = Vec::new();
        let mut completed: HashSet<String> = HashSet::new();
        let mut remaining: HashSet<String> = sub_tasks.iter().map(|s| s.id.clone()).collect();

        while !remaining.is_empty() {
            let ready: Vec<String> = remaining.iter()
                .filter(|id| {
                    if let Some(task) = sub_tasks.iter().find(|s| &s.id == *id) {
                        task.dependencies.iter().all(|dep| completed.contains(dep))
                    } else { false }
                })
                .cloned()
                .collect();

            if ready.is_empty() {
                // 死锁或无依赖→直接添加剩余
                let rest: Vec<String> = remaining.iter().cloned().collect();
                if !rest.is_empty() {
                    groups.push(rest.clone());
                    for id in &rest { completed.insert(id.clone()); }
                }
                break;
            }

            for id in &ready {
                remaining.remove(id);
                completed.insert(id.clone());
            }
            groups.push(ready);
        }

        groups
    }

    /// v2: 科学化工作量估算 — PERT 三点估算 + 系统化风险识别 + 创新化关键路径
    pub fn estimate_effort(&self, task: &str) -> EffortEstimate {
        let plan = self.decompose(task);
        let most_likely = plan.total_estimated_duration_secs.max(60);
        let optimistic = (most_likely as f64 * 0.6) as u64;
        let pessimistic = (most_likely as f64 * 1.8) as u64;
        let expected = (optimistic + 4 * most_likely + pessimistic) / 6;

        // 系统化风险识别（多维度信号）
        let lower = task.to_lowercase();
        let mut factors = Vec::new();
        let mut risk = 0.0;
        for (kw, w, label) in [
            ("security", 1.5, "安全敏感"),
            ("migrate", 1.5, "数据迁移"),
            ("production", 2.0, "生产环境"),
            ("delete", 1.5, "破坏性操作"),
            ("distributed", 1.5, "分布式"),
            ("auth", 1.0, "认证授权"),
        ] {
            if lower.contains(kw) {
                risk += w;
                factors.push(label.to_string());
            }
        }
        risk += (plan.complexity as u8 as f64 - 1.0) * 0.5;
        let risk_level = if risk >= 4.0 {
            RiskLevel::Critical
        } else if risk >= 2.5 {
            RiskLevel::High
        } else if risk >= 1.0 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        let critical = self.critical_path(&plan.sub_tasks);
        let tokens = plan.total_estimated_tokens.max(1);
        let cost = tokens as f64 * 0.000001 * (1.0 + risk * 0.1);

        EffortEstimate {
            optimistic_secs: optimistic,
            pessimistic_secs: pessimistic,
            expected_secs: expected,
            risk_level,
            risk_score: risk,
            risk_factors: factors,
            critical_path_secs: critical,
            tokens_estimate: tokens,
            cost_estimate: cost,
        }
    }

    /// 关键路径：从无依赖任务到终点的最长累计时长（创新化）
    fn critical_path(&self, sub_tasks: &[SubTask]) -> u64 {
        let mut memo: HashMap<String, u64> = HashMap::new();
        fn dfs(id: &str, tasks: &[SubTask], memo: &mut HashMap<String, u64>) -> u64 {
            if let Some(&v) = memo.get(id) {
                return v;
            }
            let t = tasks.iter().find(|s| s.id == id);
            let own = t.map(|s| s.estimated_duration_secs).unwrap_or(0);
            let dep_max = t
                .map(|s| {
                    s.dependencies
                        .iter()
                        .map(|d| dfs(d, tasks, memo))
                        .max()
                        .unwrap_or(0)
                })
                .unwrap_or(0);
            let total = own + dep_max;
            memo.insert(id.to_string(), total);
            total
        }
        sub_tasks
            .iter()
            .map(|s| dfs(&s.id, sub_tasks, &mut memo))
            .max()
            .unwrap_or(0)
    }

    // ── 辅助 ────────────────────────────────────────────────

    fn recommend_model_for_agent(&self, agent: &str) -> String {
        match agent {
            "PM" => "kimi-k3".into(),
            "Architect" => "deepseek-v4-pro".into(),
            "UIDesigner" => "glm-5v-turbo".into(),
            "Coder" => "deepseek-v4-flash".into(),
            "Auditor" => "deepseek-v4-flash".into(),
            "Reviewer" => "deepseek-v4-flash".into(),
            "Tester" => "deepseek-v4-flash".into(),
            "Verifier" => "glm-5.2".into(),
            "Scout" => "glm-5.2".into(),
            _ => "deepseek-v4-flash".into(),
        }
    }

    fn estimate_tokens(&self, title: &str, desc: &str) -> u32 {
        (title.len() + desc.len()) as u32 / 4 + 200
    }

    fn estimate_duration(&self, title: &str) -> u64 {
        let lower = title.to_lowercase();
        if lower.contains("test") || lower.contains("测试") { return 120; }
        if lower.contains("review") || lower.contains("审查") || lower.contains("审计") { return 90; }
        if lower.contains("design") || lower.contains("设计") { return 60; }
        if lower.contains("code") || lower.contains("代码") || lower.contains("实现") { return 60; }
        30
    }
}

impl Default for TaskIntelligenceEngine {
    fn default() -> Self { Self::new() }
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complexity_simple() {
        let engine = TaskIntelligenceEngine::new();
        let (level, _) = engine.estimate_complexity("fix typo in readme");
        assert_eq!(level, ComplexityLevel::Trivial);
    }

    #[test]
    fn test_complexity_complex() {
        let engine = TaskIntelligenceEngine::new();
        let (level, _) = engine.estimate_complexity(
            "refactor the entire microservice architecture to use async Rust with distributed tracing and add comprehensive security audit"
        );
        assert!(matches!(level, ComplexityLevel::Complex | ComplexityLevel::VeryComplex));
    }

    #[test]
    fn test_categorize_bug() {
        let engine = TaskIntelligenceEngine::new();
        assert_eq!(engine.categorize("fix the crash bug in login page"), TaskCategory::BugFix);
    }

    #[test]
    fn test_categorize_feature() {
        let engine = TaskIntelligenceEngine::new();
        assert_eq!(engine.categorize("design a new user dashboard feature with charts"), TaskCategory::FeatureDesign);
    }

    #[test]
    fn test_decompose_bug_fix() {
        let engine = TaskIntelligenceEngine::new();
        let plan = engine.decompose("Fix the null pointer crash in the login handler");
        assert!(plan.sub_tasks.len() >= 4);
        assert!(plan.parallel_groups.len() >= 1);
        // First sub-task should have no dependencies
        assert!(plan.sub_tasks[0].dependencies.is_empty());
    }

    #[test]
    fn test_decompose_research() {
        let engine = TaskIntelligenceEngine::new();
        let plan = engine.decompose("Research best Rust web frameworks for our API project");
        assert!(plan.sub_tasks.len() >= 5);
        assert!(plan.total_estimated_duration_secs > 0);
    }

    #[test]
    fn test_parallel_groups() {
        let engine = TaskIntelligenceEngine::new();
        let plan = engine.decompose("Implement user authentication system with tests");
        // Should have parallel groups (e.g., tests can run in parallel after code)
        let total_steps: usize = plan.parallel_groups.iter().map(|g| g.len()).sum();
        assert_eq!(total_steps, plan.sub_tasks.len());
    }
}

// ─── Tauri Commands ──────────────────────────────────────────────

#[tauri::command]
pub fn task_decompose(
    state: tauri::State<crate::state::AppState>,
    task: String,
) -> Result<serde_json::Value, String> {
    let engine = state.task_intelligence.lock().unwrap();
    let plan = engine.decompose(&task);
    Ok(serde_json::to_value(&plan).map_err(|e| e.to_string())?)
}

#[tauri::command]
pub fn task_estimate_complexity(
    state: tauri::State<crate::state::AppState>,
    task: String,
) -> Result<serde_json::Value, String> {
    let engine = state.task_intelligence.lock().unwrap();
    let (level, confidence) = engine.estimate_complexity(&task);
    let category = engine.categorize(&task);
    Ok(serde_json::json!({
        "complexity": level.label(),
        "level": level as u8,
        "confidence": confidence,
        "estimated_steps": level.estimated_steps(),
        "category": category.label(),
    }))
}