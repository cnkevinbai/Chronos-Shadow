// 蒸馏引擎 (Distillation Engine)
//
// 从原始网页/文档内容中提取结构化知识，多级蒸馏压缩，
// 确保喂给大模型的上下文既精炼又信息密度最大化。
//
// 设计理念：
//   1. 结构感知 — 识别标题层级、代码块、API签名、表格、列表等
//   2. 分级蒸馏 — Light (保留结构) / Medium (语义提取) / Deep (知识压缩)
//   3. Token预算精确 — 按目标 token 数裁剪，而非简单字节截断
//   4. 来源锚定 — 每条提取信息标注原文位置，防幻觉追溯
//   5. 缓存加速 — 相同 URL 不重复蒸馏

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::helpers::estimate_tokens;

// ─── 蒸馏级别 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DistillationLevel {
    /// 轻量：保留完整结构，仅去噪 (script/style/nav/footer/广告)
    Light,
    /// 中量：语义提取，保留标题+代码+关键段落+链接
    Medium,
    /// 深度：知识压缩，仅保留核心事实+API签名+结论
    Deep,
}

impl DistillationLevel {
    pub fn label(&self) -> &str {
        match self {
            Self::Light => "Light (结构保留)",
            Self::Medium => "Medium (语义提取)",
            Self::Deep => "Deep (知识压缩)",
        }
    }

    /// 目标压缩率（相对于原始内容）
    pub fn target_compression(&self) -> f64 {
        match self {
            Self::Light => 0.5,   // 保留 50%
            Self::Medium => 0.15, // 保留 15%
            Self::Deep => 0.04,   // 保留 4%
        }
    }
}

// ─── 提取的内容片段类型 ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentFragment {
    /// 标题（级别，文本）
    Heading { level: u8, text: String },
    /// 代码块（语言，代码，行号范围）
    CodeBlock { language: String, code: String, line_range: Option<(usize, usize)> },
    /// API 签名（函数/方法声明）
    ApiSignature { signature: String, context: String },
    /// 关键事实（陈述，置信度 0-1）
    KeyFact { statement: String, confidence: f64 },
    /// 表格（表头，行数据）
    Table { headers: Vec<String>, rows: Vec<Vec<String>> },
    /// 链接（文本，URL）
    Link { text: String, url: String },
    /// 列表项
    ListItem { text: String, depth: u8 },
    /// 定义/术语
    Definition { term: String, definition: String },
    /// 段落（保留的完整段落）
    Paragraph { text: String, importance: f64 },
    /// 原始文本块（兜底）
    RawText { text: String },
}

impl ContentFragment {
    /// 估算 fragment 的 token 数（粗略：1 token ≈ 4 字符）
    pub fn estimated_tokens(&self) -> usize {
        let text = match self {
            Self::Heading { text, .. } => text,
            Self::CodeBlock { code, .. } => code,
            Self::ApiSignature { signature, context } => &format!("{} ({})", signature, context),
            Self::KeyFact { statement, .. } => statement,
            Self::Table { headers, rows } => &format!("{:?}{:?}", headers, rows),
            Self::Link { text, url } => &format!("{} [{}]", text, url),
            Self::ListItem { text, .. } => text,
            Self::Definition { term, definition } => &format!("{}: {}", term, definition),
            Self::Paragraph { text, .. } => text,
            Self::RawText { text } => text,
        };
        estimate_tokens(text)
    }

    /// 重要性评分 0-10
    pub fn importance(&self) -> u8 {
        match self {
            Self::Heading { level, .. } => (10 - level).min(10) as u8,
            Self::CodeBlock { .. } => 8,
            Self::ApiSignature { .. } => 9,
            Self::KeyFact { confidence, .. } => (*confidence * 10.0) as u8,
            Self::Table { .. } => 6,
            Self::Link { .. } => 3,
            Self::ListItem { .. } => 4,
            Self::Definition { .. } => 7,
            Self::Paragraph { importance, .. } => (*importance * 10.0) as u8,
            Self::RawText { .. } => 1,
        }
    }
}

// ─── 实体提取结果 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    pub entity_type: EntityType,
    pub name: String,
    pub context: String,
    pub occurrences: u32,
}

/// 实体关系（v2：创新化 — 从平面实体到关系图）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRelation {
    pub source: String,
    pub target: String,
    pub relation: String, // co_occur / depends_on / deprecates
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityType {
    Version,        // 版本号: "1.75.0", "v2.0"
    Date,           // 日期: "2024-01-15"
    Crate,          // Rust crate: "tokio", "serde"
    Package,        // npm/pip package: "react", "django"
    Function,       // 函数名
    TypeName,       // 类型名
    Repository,     // GitHub repo: "user/repo"
    Email,          // 邮箱地址
    License,        // 许可证: "MIT", "Apache-2.0"
    Deprecated,     // 已废弃标记
    Breaking,       // 破坏性变更标记
}

impl EntityType {
    pub fn label(&self) -> &str {
        match self {
            Self::Version => "版本",
            Self::Date => "日期",
            Self::Crate => "Crate",
            Self::Package => "包",
            Self::Function => "函数",
            Self::TypeName => "类型",
            Self::Repository => "仓库",
            Self::Email => "邮箱",
            Self::License => "许可证",
            Self::Deprecated => "⚠️ 废弃",
            Self::Breaking => "🔴 破坏性变更",
        }
    }
}

// ─── 蒸馏结果 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistillationResult {
    /// 原始内容大小（字节）
    pub original_size: usize,
    /// 蒸馏后大小（字节）
    pub distilled_size: usize,
    /// 压缩率
    pub compression_ratio: f64,
    /// 估算 token 数
    pub estimated_tokens: usize,
    /// 蒸馏级别
    pub level: DistillationLevel,
    /// 提取的标题层级
    pub heading_tree: Vec<String>,
    /// 提取的内容片段
    pub fragments: Vec<ContentFragment>,
    /// 关键发现（仅 Medium/Deep）
    pub key_insights: Vec<String>,
    /// 引用的外部链接
    pub references: Vec<(String, String)>,
    /// 提取的实体（版本/日期/包名等）
    pub entities: Vec<ExtractedEntity>,
    /// 蒸馏耗时（毫秒）
    pub distillation_time_ms: u64,
    /// 生成的 Markdown 文本
    pub markdown: String,
}

// ─── 蒸馏引擎 ──────────────────────────────────────────────────────

/// 蒸馏进化记录 — 追踪每次蒸馏的效果，用于自我改进
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistillationEvolutionRecord {
    pub source_url: String,
    pub level: DistillationLevel,
    pub original_size: usize,
    pub distilled_size: usize,
    pub compression_ratio: f64,
    pub content_type: String,       // "documentation" | "blog" | "code" | "mixed"
    pub quality_score: f64,         // 0-1 用户反馈质量
    pub fragments_extracted: usize,
    pub entities_found: usize,
    pub timestamp: String,
}

/// 自适应策略 — 根据内容类型和历史效果自动调参
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveStrategy {
    pub content_type: String,
    /// 最优 Token 预算
    pub optimal_token_budget: usize,
    /// 最优蒸馏级别
    pub optimal_level: DistillationLevel,
    /// 平均压缩率
    pub avg_compression: f64,
    /// 平均质量分
    pub avg_quality: f64,
    /// 使用次数
    pub usage_count: u64,
    /// 最后更新时间
    pub last_updated: String,
}

pub struct DistillationEngine {
    /// 缓存：URL → (DistillationLevel, DistillationResult)
    pub(super) cache: HashMap<String, HashMap<DistillationLevel, DistillationResult>>,
    /// 最大缓存条目数
    pub(super) max_cache_entries: usize,
    /// 默认 Token 预算
    pub default_token_budget: usize,
    /// 累计节省字节数
    pub total_bytes_saved: u64,
    /// 累计蒸馏次数
    pub total_distillations: u64,

    // ── 进化系统 ──
    /// 进化记录（最近500条）
    pub evolution_log: Vec<DistillationEvolutionRecord>,
    /// 自适应策略表：content_type → 最优参数
    pub adaptive_strategies: HashMap<String, AdaptiveStrategy>,
    /// 进化权重（强化学习风格）
    pub evolution_weights: DistillationWeights,
    /// 进化启用
    pub evolution_enabled: bool,
    /// 质量反馈累积
    pub(super) quality_feedback_count: u64,
    pub(super) quality_feedback_sum: f64,
}

/// 可进化的蒸馏权重
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistillationWeights {
    /// 代码块保留权重 (0-1)
    pub code_retention: f64,
    /// API签名保留权重
    pub api_retention: f64,
    /// 事实提取权重
    pub fact_extraction: f64,
    /// 段落重要性阈值 (低于此值丢弃)
    pub paragraph_threshold: f64,
    /// 实体提取激进程度 (越高提取越多)
    pub entity_aggressiveness: f64,
    /// 缓存TTL因子 (×基础TTL)
    pub cache_ttl_factor: f64,
    /// 压缩激进程度 (越高压缩越狠)
    pub compression_aggressiveness: f64,
}

impl Default for DistillationWeights {
    fn default() -> Self {
        Self {
            code_retention: 0.9,
            api_retention: 0.85,
            fact_extraction: 0.8,
            paragraph_threshold: 0.3,
            entity_aggressiveness: 0.7,
            cache_ttl_factor: 1.0,
            compression_aggressiveness: 0.5,
        }
    }
}

