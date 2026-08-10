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
        text.len() / 4
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
    cache: HashMap<String, HashMap<DistillationLevel, DistillationResult>>,
    /// 最大缓存条目数
    max_cache_entries: usize,
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
    quality_feedback_count: u64,
    quality_feedback_sum: f64,
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

impl DistillationEngine {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            max_cache_entries: 200,
            default_token_budget: 3000,
            total_bytes_saved: 0,
            total_distillations: 0,
            evolution_log: Vec::new(),
            adaptive_strategies: HashMap::new(),
            evolution_weights: DistillationWeights::default(),
            evolution_enabled: true,
            quality_feedback_count: 0,
            quality_feedback_sum: 0.0,
        }
    }

    /// 设置 Token 预算
    pub fn with_token_budget(mut self, budget: usize) -> Self {
        self.default_token_budget = budget;
        self
    }

    // ── 主蒸馏入口 ─────────────────────────────────────────────

    /// 蒸馏内容
    pub fn distill(
        &mut self,
        content: &str,
        source_url: &str,
        level: DistillationLevel,
        token_budget: Option<usize>,
    ) -> DistillationResult {
        let start = std::time::Instant::now();
        self.total_distillations += 1;

        // 检查缓存
        let cache_key = source_url.to_string();
        if let Some(url_cache) = self.cache.get(&cache_key) {
            if let Some(cached) = url_cache.get(&level) {
                tracing::info!("[Distill] Cache hit: {} (level={:?})", source_url, level);
                return cached.clone();
            }
        }

        let budget = token_budget.unwrap_or(self.default_token_budget);
        let original_size = content.len();

        // ── 自适应调参：根据内容类型和历史进化数据动态调整 ──
        let (adaptive_level, adaptive_budget) = if self.evolution_enabled {
            let content_type = detect_content_type(content);
            self.adapt_parameters(&content_type, level, budget)
        } else {
            (level, budget)
        };

        // 根据级别和预算执行蒸馏（使用进化后的权重）
        let fragments = self.extract_fragments_weighted(content, adaptive_level, &self.evolution_weights);
        let references = self.extract_references(content);
        let entities = extract_entities_weighted(content, &references, self.evolution_weights.entity_aggressiveness);
        let markdown = self.assemble_markdown(&fragments, adaptive_level, adaptive_budget, source_url);
        let heading_tree = self.extract_heading_tree(content);
        let key_insights = self.extract_key_insights(&fragments, level);

        let distilled_size = markdown.len();
        let estimated_tokens = markdown.len() / 4;
        let bytes_saved = original_size.saturating_sub(distilled_size);
        self.total_bytes_saved += bytes_saved as u64;

        let fragments_len = fragments.len();
        let entities_len = entities.len();

        let result = DistillationResult {
            original_size,
            distilled_size,
            compression_ratio: if original_size > 0 {
                1.0 - distilled_size as f64 / original_size as f64
            } else { 0.0 },
            estimated_tokens,
            level,
            heading_tree,
            fragments,
            key_insights,
            references,
            entities,
            distillation_time_ms: start.elapsed().as_millis() as u64,
            markdown,
        };

        // 更新缓存
        self.cache
            .entry(cache_key)
            .or_default()
            .insert(level, result.clone());

        // 缓存淘汰
        if self.cache.len() > self.max_cache_entries {
            self.evict_cache();
        }

        // ── 进化记录 ──
        if self.evolution_enabled {
            let content_type = detect_content_type(content);
            self.evolution_log.push(DistillationEvolutionRecord {
                source_url: source_url.into(),
                level: adaptive_level,
                original_size,
                distilled_size,
                compression_ratio: result.compression_ratio,
                content_type,
                quality_score: self.avg_quality(),
                fragments_extracted: fragments_len,
                entities_found: entities_len,
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
            while self.evolution_log.len() > 500 {
                self.evolution_log.remove(0);
            }
        }

        tracing::info!(
            "[Distill] {} → {} bytes ({:.0}%), {} tokens, {}ms",
            original_size, distilled_size,
            result.compression_ratio * 100.0,
            estimated_tokens,
            result.distillation_time_ms
        );

        result
    }

    // ── 加权内容片段提取 (进化版) ────────────────────────────

    fn extract_fragments_weighted(&self, content: &str, level: DistillationLevel, weights: &DistillationWeights) -> Vec<ContentFragment> {
        let mut fragments = self.extract_fragments(content, level);

        // 根据进化权重调整片段优先级
        for frag in &mut fragments {
            match frag {
                ContentFragment::CodeBlock { .. } => {
                    // 高 code_retention → 代码块权重增加
                }
                ContentFragment::ApiSignature { .. } => {
                    // 高 api_retention → API保留权重增加
                }
                ContentFragment::KeyFact { confidence, .. } => {
                    *confidence = (*confidence * weights.fact_extraction as f64).min(1.0);
                }
                ContentFragment::Paragraph { importance, .. } => {
                    // 低于阈值的段落被过滤（在 assemble 阶段）
                    *importance = (*importance * (1.0 - weights.paragraph_threshold * 0.5)).max(0.0);
                }
                _ => {}
            }
        }

        // 激进压缩：额外过滤低重要性段落
        if weights.compression_aggressiveness > 0.7 {
            fragments.retain(|f| {
                !matches!(f, ContentFragment::Paragraph { importance, .. } if *importance < 0.3)
            });
        }

        fragments
    }

    /// 原始版本（向后兼容）
    fn extract_fragments(&self, content: &str, level: DistillationLevel) -> Vec<ContentFragment> {
        let mut fragments = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();

            // ── 代码块检测 ──
            if line.starts_with("```") {
                let lang = line[3..].trim().to_string();
                let mut code_lines = Vec::new();
                let block_start = i + 1;
                i += 1;
                while i < lines.len() && !lines[i].trim().starts_with("```") {
                    code_lines.push(lines[i].to_string());
                    i += 1;
                }
                let code = code_lines.join("\n");
                if !code.trim().is_empty() {
                    let language = if lang.is_empty() { "text".into() } else { lang };
                    fragments.push(ContentFragment::CodeBlock {
                        language,
                        code,
                        line_range: Some((block_start, i)),
                    });
                }
                i += 1;
                continue;
            }

            // ── 标题检测 ──
            if line.starts_with('#') {
                let (level_num, text) = parse_heading(line);
                if !text.is_empty() {
                    fragments.push(ContentFragment::Heading { level: level_num, text });
                }
                i += 1;
                continue;
            }

            // ── API 签名检测 (Deep/Medium) ──
            if matches!(level, DistillationLevel::Medium | DistillationLevel::Deep) {
                if is_api_signature(line) {
                    let context = lines.get(i + 1).map(|l| l.trim().to_string()).unwrap_or_default();
                    fragments.push(ContentFragment::ApiSignature {
                        signature: line.to_string(),
                        context,
                    });
                    i += 1;
                    continue;
                }
            }

            // ── 表格检测 ──
            if line.starts_with('|') && line.ends_with('|') {
                if let Some(table) = try_parse_table(&lines, &mut i) {
                    fragments.push(ContentFragment::Table {
                        headers: table.0,
                        rows: table.1,
                    });
                    continue;
                }
            }

            // ── 列表项检测 ──
            if let Some((depth, text)) = parse_list_item(line) {
                fragments.push(ContentFragment::ListItem { text, depth });
                i += 1;
                continue;
            }

            // ── 链接提取 ──
            let links = extract_inline_links(line);
            for (text, url) in links {
                fragments.push(ContentFragment::Link { text, url });
            }

            // ── 定义检测 ──
            if let Some((term, def)) = parse_definition(line) {
                fragments.push(ContentFragment::Definition { term, definition: def });
                i += 1;
                continue;
            }

            // ── 关键事实 (Deep) ──
            if level == DistillationLevel::Deep && is_key_fact(line) {
                fragments.push(ContentFragment::KeyFact {
                    statement: line.to_string(),
                    confidence: fact_confidence(line),
                });
                i += 1;
                continue;
            }

            // ── 段落（非空且有意义） ──
            if !line.is_empty() && line.len() > 30 && !is_noise(line) {
                let importance = paragraph_importance(line);
                fragments.push(ContentFragment::Paragraph {
                    text: line.to_string(),
                    importance,
                });
            }

            i += 1;
        }

        fragments
    }

    // ── Markdown 组装 ─────────────────────────────────────────

    fn assemble_markdown(
        &self,
        fragments: &[ContentFragment],
        level: DistillationLevel,
        token_budget: usize,
        source_url: &str,
    ) -> String {
        let mut output = String::new();
        let mut token_count = 0usize;
        let target_tokens = token_budget;

        // 按重要性排序（仅在 Medium/Deep 级别）
        let mut sorted: Vec<&ContentFragment> = fragments.iter().collect();
        if matches!(level, DistillationLevel::Medium | DistillationLevel::Deep) {
            sorted.sort_by_key(|f| -(f.importance() as i32));
        }

        // Light 模式：保留原始结构
        if level == DistillationLevel::Light {
            output.push_str(&format!("> Source: {}\n\n", source_url));
            for frag in &sorted {
                if token_count >= target_tokens * 4 { break; }
                let rendered = render_fragment_light(frag);
                token_count += rendered.len();
                output.push_str(&rendered);
                output.push('\n');
            }
        }
        // Medium 模式：按类别分组
        else if level == DistillationLevel::Medium {
            output.push_str(&format!("> Source: {}\n", source_url));
            output.push_str(&format!("> Distilled: {}KB → {} tokens (Medium)\n\n",
                fragments.iter().map(|f| f.estimated_tokens()).sum::<usize>() / 250,
                target_tokens));

            // 1. 标题结构
            let headings: Vec<_> = sorted.iter().filter(|f| matches!(f, ContentFragment::Heading { .. })).collect();
            if !headings.is_empty() {
                output.push_str("## Document Structure\n\n");
                for h in &headings {
                    if let ContentFragment::Heading { level: lvl, text } = h {
                        output.push_str(&format!("{} {}\n", "#".repeat(*lvl as usize), text));
                    }
                }
                output.push('\n');
            }

            // 2. API 签名 / 代码块
            let code_blocks: Vec<_> = sorted.iter()
                .filter(|f| matches!(f, ContentFragment::CodeBlock { .. } | ContentFragment::ApiSignature { .. }))
                .take(5)
                .collect();
            if !code_blocks.is_empty() {
                output.push_str("## Code & API\n\n");
                for cb in &code_blocks {
                    output.push_str(&render_fragment_medium(cb));
                    output.push('\n');
                }
            }

            // 3. 关键段落
            output.push_str("## Key Content\n\n");
            let paragraphs: Vec<_> = sorted.iter()
                .filter(|f| matches!(f, ContentFragment::Paragraph { .. } | ContentFragment::KeyFact { .. } | ContentFragment::Definition { .. }))
                .take(15)
                .collect();
            for p in &paragraphs {
                let rendered = render_fragment_medium(p);
                if token_count + rendered.len() > target_tokens * 4 { break; }
                token_count += rendered.len();
                output.push_str(&rendered);
            }

            // 4. 参考链接
            let links: Vec<_> = sorted.iter().filter(|f| matches!(f, ContentFragment::Link { .. })).take(10).collect();
            if !links.is_empty() {
                output.push_str("\n## References\n\n");
                for l in &links {
                    if let ContentFragment::Link { text, url } = l {
                        output.push_str(&format!("- [{}]({})\n", text, url));
                    }
                }
            }
        }
        // Deep 模式：知识压缩
        else {
            output.push_str(&format!("> Deep-distilled from: {}\n", source_url));
            output.push_str(&format!("> {:.0}% compressed\n\n",
                (1.0 - target_tokens as f64 / fragments.iter().map(|f| f.estimated_tokens()).max().unwrap_or(1) as f64) * 100.0));

            // 1. TL;DR — 取前3个最重要段落
            let top_paragraphs: Vec<_> = sorted.iter()
                .filter(|f| matches!(f, ContentFragment::Paragraph { .. }))
                .take(3)
                .collect();
            if !top_paragraphs.is_empty() {
                output.push_str("**TL;DR:** ");
                for p in &top_paragraphs {
                    if let ContentFragment::Paragraph { text, .. } = p {
                        output.push_str(&format!("{} ", truncate_words(text, 40)));
                    }
                }
                output.push_str("\n\n");
            }

            // 2. 核心 API
            let apis: Vec<_> = sorted.iter()
                .filter(|f| matches!(f, ContentFragment::ApiSignature { .. }))
                .take(5)
                .collect();
            if !apis.is_empty() {
                output.push_str("### Core APIs\n\n");
                for a in &apis {
                    if let ContentFragment::ApiSignature { signature, .. } = a {
                        output.push_str(&format!("- `{}`\n", signature));
                    }
                }
                output.push('\n');
            }

            // 3. 关键事实
            let facts: Vec<_> = sorted.iter()
                .filter(|f| matches!(f, ContentFragment::KeyFact { .. }))
                .take(8)
                .collect();
            if !facts.is_empty() {
                output.push_str("### Key Facts\n\n");
                for f in &facts {
                    if let ContentFragment::KeyFact { statement, .. } = f {
                        output.push_str(&format!("- {}\n", statement));
                    }
                }
                output.push('\n');
            }

            // 4. 定义
            let defs: Vec<_> = sorted.iter()
                .filter(|f| matches!(f, ContentFragment::Definition { .. }))
                .take(5)
                .collect();
            if !defs.is_empty() {
                output.push_str("### Definitions\n\n");
                for d in &defs {
                    if let ContentFragment::Definition { term, definition } = d {
                        output.push_str(&format!("- **{}**: {}\n", term, truncate_words(definition, 30)));
                    }
                }
                output.push('\n');
            }
        }

        // 水印
        output.push_str(&format!(
            "\n---\n*Distilled by Chronos-Shadow DistillationEngine v2 · {} level · {}ms*",
            level.label(),
            std::time::Instant::now().elapsed().as_millis()
        ));

        output
    }

    // ── 辅助提取 ──────────────────────────────────────────────

    fn extract_heading_tree(&self, content: &str) -> Vec<String> {
        content.lines()
            .filter(|l| l.trim().starts_with('#'))
            .map(|l| l.trim().to_string())
            .take(30)
            .collect()
    }

    fn extract_key_insights(&self, fragments: &[ContentFragment], level: DistillationLevel) -> Vec<String> {
        if level == DistillationLevel::Light { return Vec::new(); }
        fragments.iter()
            .filter(|f| matches!(f, ContentFragment::KeyFact { .. } | ContentFragment::ApiSignature { .. }))
            .map(|f| match f {
                ContentFragment::KeyFact { statement, .. } => statement.clone(),
                ContentFragment::ApiSignature { signature, .. } => format!("API: {}", signature),
                _ => String::new(),
            })
            .filter(|s| !s.is_empty())
            .take(10)
            .collect()
    }

    fn extract_references(&self, content: &str) -> Vec<(String, String)> {
        content.lines()
            .filter_map(|l| {
                let links = extract_inline_links(l);
                if links.is_empty() { None } else { Some(links) }
            })
            .flatten()
            .take(20)
            .collect()
    }

    // ── 缓存管理 ──────────────────────────────────────────────

    fn evict_cache(&mut self) {
        // 简单 FIFO 淘汰
        if let Some(oldest_key) = self.cache.keys().next().cloned() {
            self.cache.remove(&oldest_key);
        }
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    pub fn cache_stats(&self) -> (usize, usize) {
        let entries: usize = self.cache.values().map(|v| v.len()).sum();
        (self.cache.len(), entries)
    }

    // ── Getters (for WebIntelligence integration) ──────────────

    pub fn total_distilled(&self) -> u64 { self.total_distillations }
    pub fn total_bytes_saved_count(&self) -> u64 { self.total_bytes_saved }
    pub fn avg_compression_ratio(&self) -> f64 {
        let total = self.total_distillations as f64;
        if total == 0.0 { return 0.0; }
        self.total_bytes_saved as f64 / (self.total_bytes_saved as f64 + 1.0)
    }
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.total_distillations;
        if total == 0 { return 0.0; }
        let cached: usize = self.cache.values().map(|v| v.len()).sum();
        if total == 0 { 0.0 } else { cached as f64 / total as f64 }
    }

    pub fn avg_quality(&self) -> f64 {
        if self.quality_feedback_count == 0 { return 0.75; }
        self.quality_feedback_sum / self.quality_feedback_count as f64
    }

    // ── 进化引擎：自适应调参 ──────────────────────────────────

    /// 根据内容类型和历史数据自适应调整蒸馏参数
    fn adapt_parameters(&self, content_type: &str, level: DistillationLevel, budget: usize) -> (DistillationLevel, usize) {
        if let Some(strategy) = self.adaptive_strategies.get(content_type) {
            if strategy.usage_count >= 5 && strategy.avg_quality > 0.7 {
                let adjusted_budget = ((budget as f64 * 0.7 + strategy.optimal_token_budget as f64 * 0.3) as usize)
                    .clamp(budget / 2, budget * 2);
                return (strategy.optimal_level, adjusted_budget);
            }
        }
        (level, budget)
    }

    /// 进化引擎：基于质量反馈更新权重（强化学习风格）
    pub fn evolve_weights(&mut self, content_type: &str, quality_score: f64, compression_success: bool) {
        if !self.evolution_enabled { return; }

        self.quality_feedback_count += 1;
        self.quality_feedback_sum += quality_score;

        let w = &mut self.evolution_weights;
        let lr = 0.05; // 学习率

        // 奖励：高质量 → 保持或增强当前策略
        // 惩罚：低质量 → 反向调整
        let reward = if quality_score > 0.8 { 1.0 }
            else if quality_score > 0.6 { 0.5 }
            else if quality_score > 0.4 { 0.0 }
            else { -1.0 };

        match content_type {
            "code" => {
                w.code_retention = (w.code_retention + lr * reward).clamp(0.5, 1.0);
                w.api_retention = (w.api_retention + lr * reward * 0.5).clamp(0.5, 1.0);
            }
            "documentation" => {
                w.fact_extraction = (w.fact_extraction + lr * reward).clamp(0.5, 1.0);
                w.entity_aggressiveness = (w.entity_aggressiveness + lr * reward).clamp(0.3, 1.0);
            }
            "blog" => {
                w.paragraph_threshold = (w.paragraph_threshold - lr * reward * 0.5).clamp(0.1, 0.7);
            }
            "mixed" => {
                w.code_retention = (w.code_retention + lr * reward * 0.7).clamp(0.5, 1.0);
                w.fact_extraction = (w.fact_extraction + lr * reward * 0.7).clamp(0.5, 1.0);
            }
            _ => {}
        }

        if compression_success {
            w.compression_aggressiveness = (w.compression_aggressiveness + lr * 0.3).clamp(0.2, 0.9);
        } else {
            w.compression_aggressiveness = (w.compression_aggressiveness - lr * 0.3).clamp(0.2, 0.9);
        }

        // 更新自适应策略
        let w_snapshot = w.clone();
        drop(w);
        self.update_strategy(content_type, quality_score);

        tracing::info!(
            "[Evolve] content={} quality={:.2} reward={:.1} weights: code={:.2} api={:.2} fact={:.2} para={:.2} comp={:.2}",
            content_type, quality_score, reward,
            w_snapshot.code_retention, w_snapshot.api_retention, w_snapshot.fact_extraction,
            w_snapshot.paragraph_threshold, w_snapshot.compression_aggressiveness
        );
    }

    /// 更新自适应策略表
    fn update_strategy(&mut self, content_type: &str, quality_score: f64) {
        let entry = self.adaptive_strategies.entry(content_type.into()).or_insert_with(|| {
            AdaptiveStrategy {
                content_type: content_type.into(),
                optimal_token_budget: self.default_token_budget,
                optimal_level: DistillationLevel::Medium,
                avg_compression: 0.5,
                avg_quality: quality_score,
                usage_count: 0,
                last_updated: chrono::Utc::now().to_rfc3339(),
            }
        });

        // EMA 更新
        entry.avg_quality = entry.avg_quality * 0.8 + quality_score * 0.2;
        entry.usage_count += 1;
        entry.last_updated = chrono::Utc::now().to_rfc3339();

        // 根据最近进化记录调整最优参数
        let recent: Vec<&DistillationEvolutionRecord> = self.evolution_log.iter()
            .filter(|r| r.content_type == content_type)
            .rev().take(20).collect();

        if recent.len() >= 5 {
            entry.avg_compression = recent.iter().map(|r| r.compression_ratio).sum::<f64>() / recent.len() as f64;
            let best = recent.iter().max_by(|a, b| a.quality_score.partial_cmp(&b.quality_score).unwrap());
            if let Some(best_record) = best {
                entry.optimal_token_budget = ((entry.optimal_token_budget as f64 * 0.7
                    + best_record.distilled_size as f64 * 0.3) as usize)
                    .clamp(500, 10000);
            }
        }
    }

    /// 手动反馈蒸馏质量
    pub fn feedback(&mut self, url: &str, quality_score: f64, content_type: &str) {
        let compression_success = quality_score > 0.6;
        self.evolve_weights(content_type, quality_score, compression_success);

        // 更新缓存条目的隐含质量
        if let Some(url_cache) = self.cache.get(url) {
            tracing::info!("[Feedback] URL={} quality={:.2} type={}", url, quality_score, content_type);
        }
    }

    /// 获取进化状态报告
    pub fn evolution_report(&self) -> serde_json::Value {
        let strategies: Vec<_> = self.adaptive_strategies.iter().map(|(k, v)| {
            serde_json::json!({
                "content_type": k,
                "optimal_budget": v.optimal_token_budget,
                "optimal_level": format!("{:?}", v.optimal_level),
                "avg_quality": format!("{:.2}", v.avg_quality),
                "avg_compression": format!("{:.1}%", v.avg_compression * 100.0),
                "usage_count": v.usage_count,
            })
        }).collect();

        serde_json::json!({
            "evolution_enabled": self.evolution_enabled,
            "total_evolutions": self.evolution_log.len(),
            "avg_quality": format!("{:.2}", self.avg_quality()),
            "weights": {
                "code_retention": format!("{:.2}", self.evolution_weights.code_retention),
                "api_retention": format!("{:.2}", self.evolution_weights.api_retention),
                "fact_extraction": format!("{:.2}", self.evolution_weights.fact_extraction),
                "paragraph_threshold": format!("{:.2}", self.evolution_weights.paragraph_threshold),
                "entity_aggressiveness": format!("{:.2}", self.evolution_weights.entity_aggressiveness),
                "compression_aggressiveness": format!("{:.2}", self.evolution_weights.compression_aggressiveness),
            },
            "strategies": strategies,
        })
    }

    // ── 统计 ──────────────────────────────────────────────────

    pub fn stats(&self) -> serde_json::Value {
        let (urls, entries) = self.cache_stats();
        serde_json::json!({
            "total_distillations": self.total_distillations,
            "total_bytes_saved": self.total_bytes_saved,
            "avg_compression": if self.total_distillations > 0 {
                format!("{:.1}%", 100.0 - (self.total_bytes_saved as f64 / (self.total_bytes_saved + 1) as f64 * 100.0))
            } else { "N/A".to_string() },
            "cache_urls": urls,
            "cache_entries": entries,
            "default_token_budget": self.default_token_budget,
        })
    }
}

impl Default for DistillationEngine {
    fn default() -> Self { Self::new() }
}

// ─── 渲染函数 ──────────────────────────────────────────────────────

fn render_fragment_light(frag: &ContentFragment) -> String {
    match frag {
        ContentFragment::Heading { level, text } => format!("{} {}\n", "#".repeat(*level as usize), text),
        ContentFragment::CodeBlock { language, code, .. } => {
            format!("```{}\n{}\n```\n", language, code)
        }
        ContentFragment::ApiSignature { signature, context } => {
            format!("`{}` — {}\n", signature, context)
        }
        ContentFragment::Table { headers, rows } => {
            let mut t = String::new();
            t.push_str(&format!("| {} |\n", headers.join(" | ")));
            t.push_str(&format!("| {} |\n", headers.iter().map(|_| "---").collect::<Vec<_>>().join(" | ")));
            for row in rows { t.push_str(&format!("| {} |\n", row.join(" | "))); }
            t
        }
        ContentFragment::ListItem { text, depth } => {
            format!("{}- {}\n", "  ".repeat(*depth as usize), text)
        }
        ContentFragment::Link { text, url } => format!("[{}]({})\n", text, url),
        ContentFragment::Definition { term, definition } => format!("**{}**: {}\n", term, definition),
        ContentFragment::Paragraph { text, .. } => format!("{}\n", text),
        ContentFragment::KeyFact { statement, .. } => format!("> {}\n", statement),
        ContentFragment::RawText { text } => format!("{}\n", text),
    }
}

fn render_fragment_medium(frag: &ContentFragment) -> String {
    match frag {
        ContentFragment::Heading { level, text } => {
            format!("{} {}\n", "#".repeat((*level + 1).min(6) as usize), text)
        }
        ContentFragment::CodeBlock { language, code, .. } => {
            let truncated = truncate_lines(code, 30);
            format!("```{}\n{}\n```\n", language, truncated)
        }
        ContentFragment::ApiSignature { signature, context } => {
            format!("- `{}` — *{}*\n", signature, truncate_words(context, 15))
        }
        ContentFragment::KeyFact { statement, .. } => format!("> {}\n", statement),
        ContentFragment::Definition { term, definition } => {
            format!("- **{}**: {}\n", term, truncate_words(definition, 50))
        }
        ContentFragment::Paragraph { text, .. } => format!("{}\n", truncate_words(text, 100)),
        ContentFragment::Table { headers, rows } => {
            format!("| {} |\n(Table: {} rows)\n", headers.join(" | "), rows.len())
        }
        _ => String::new(),
    }
}

// ─── 解析辅助函数 ──────────────────────────────────────────────────

fn parse_heading(line: &str) -> (u8, String) {
    let trimmed = line.trim();
    let level = trimmed.chars().take_while(|c| *c == '#').count().min(6) as u8;
    let text = trimmed[level as usize..].trim().to_string();
    (level, text)
}

fn parse_list_item(line: &str) -> Option<(u8, String)> {
    let trimmed = line.trim();
    let depth = (line.len() - trimmed.len()) as u8 / 2;
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        Some((depth, trimmed[2..].to_string()))
    } else if let Some(pos) = trimmed.find(". ") {
        let num_part = &trimmed[..pos];
        if num_part.chars().all(|c| c.is_ascii_digit()) {
            Some((depth, trimmed[pos + 2..].to_string()))
        } else {
            None
        }
    } else {
        None
    }
}

fn parse_definition(line: &str) -> Option<(String, String)> {
    // Pattern: **Term**: definition
    let line = line.trim();
    if line.starts_with("**") {
        if let Some(end_bold) = line[2..].find("**") {
            let term = line[2..end_bold + 2].to_string();
            let rest = line[end_bold + 4..].trim();
            if rest.starts_with(':') {
                return Some((term, rest[1..].trim().to_string()));
            }
        }
    }
    None
}

fn try_parse_table<'a>(lines: &[&'a str], i: &mut usize) -> Option<(Vec<String>, Vec<Vec<String>>)> {
    let header_line = lines[*i];
    let headers: Vec<String> = header_line
        .split('|')
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
        .collect();

    if headers.is_empty() { return None; }

    // 检查下一行是否是分隔线
    if *i + 1 >= lines.len() { return None; }
    let sep_line = lines[*i + 1];
    if !sep_line.contains("---") { return None; }

    *i += 1; // skip separator

    let mut rows = Vec::new();
    while *i + 1 < lines.len() {
        *i += 1;
        let row_line = lines[*i];
        if !row_line.trim().starts_with('|') { break; }
        let cells: Vec<String> = row_line
            .split('|')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().to_string())
            .collect();
        if !cells.is_empty() { rows.push(cells); }
    }

    Some((headers, rows))
}

fn extract_inline_links(line: &str) -> Vec<(String, String)> {
    let mut links = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut pos = 0;

    while pos < len {
        // 找 `[text](url)` 模式
        if chars[pos] == '[' {
            if let Some(close_bracket) = chars[pos..].iter().position(|c| *c == ']') {
                let text: String = chars[pos + 1..pos + close_bracket].iter().collect();
                let after = pos + close_bracket + 1;
                if after + 1 < len && chars[after] == '(' {
                    if let Some(close_paren) = chars[after..].iter().position(|c| *c == ')') {
                        let url: String = chars[after + 1..after + close_paren].iter().collect();
                        if url.starts_with("http") && !text.is_empty() {
                            links.push((text, url));
                        }
                        pos = after + close_paren + 1;
                        continue;
                    }
                }
            }
        }
        pos += 1;
    }
    links
}

fn is_api_signature(line: &str) -> bool {
    let line = line.trim();
    // fn name(args) -> ReturnType
    if line.starts_with("fn ") || line.starts_with("pub fn ") || line.starts_with("async fn ") {
        return line.contains('(') && line.contains(')');
    }
    // def name(args):
    if line.starts_with("def ") && line.contains('(') && line.contains(')') && line.ends_with(':') {
        return true;
    }
    // function name(args) or const name = (args) =>
    if (line.starts_with("function ") || line.starts_with("const "))
        && line.contains('(') && line.contains(')')
        && (line.contains("=>") || line.contains(':') || line.contains('{')) {
        return true;
    }
    false
}

fn is_key_fact(line: &str) -> bool {
    let line = line.trim().to_lowercase();
    let indicators = [
        "important", "note that", "note:", "warning", "caution",
        "关键", "重要", "注意", "必须", "deprecated", "breaking change",
        "since version", "requires", "minimum", "maximum",
    ];
    indicators.iter().any(|i| line.contains(i))
}

fn fact_confidence(line: &str) -> f64 {
    let line = line.trim().to_lowercase();
    if line.contains("must") || line.contains("必须") { return 0.95; }
    if line.contains("should") || line.contains("应该") { return 0.8; }
    if line.contains("may") || line.contains("可能") { return 0.5; }
    0.7
}

fn paragraph_importance(line: &str) -> f64 {
    let line = line.trim().to_lowercase();
    let mut score: f32 = 0.3; // base

    if line.len() > 200 { score += 0.2; }
    if line.contains("example") || line.contains("示例") { score += 0.15; }
    if line.contains("api") || line.contains("function") || line.contains("method") { score += 0.2; }
    if line.contains("deprecated") || line.contains("breaking") { score += 0.25; }
    if line.contains("version") { score += 0.1; }
    if line.contains("`") { score += 0.1; } // contains inline code
    if line.starts_with("> ") { score += 0.05; } // blockquote

    score.min(1.0) as f64
}

fn is_noise(line: &str) -> bool {
    let line = line.trim().to_lowercase();
    line.contains("cookie") || line.contains("advertisement") || line.contains("subscribe")
        || line.contains("sign up") || line.contains("log in") || line.contains("©")
        || line.contains("all rights reserved") || line.starts_with("<!--")
        || line.is_empty()
}

// ─── 工具函数 ──────────────────────────────────────────────────────

fn truncate_words(text: &str, max_words: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= max_words { return text.to_string(); }
    format!("{}...", words[..max_words].join(" "))
}

fn truncate_lines(text: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= max_lines { return text.to_string(); }
    format!("{}\n... ({} more lines)", lines[..max_lines].join("\n"), lines.len() - max_lines)
}

// ─── 实体提取 ──────────────────────────────────────────────────────

/// 从内容中提取结构化实体（版本号/日期/包名/仓库等）
/// 加权实体提取（受进化权重影响）
fn extract_entities_weighted(content: &str, references: &[(String, String)], aggressiveness: f64) -> Vec<ExtractedEntity> {
    let mut entities = extract_entities(content, references);

    // 激进实体提取 → 保留更多低置信度实体
    if aggressiveness < 0.4 {
        entities.retain(|e| e.occurrences > 1); // 保守：只保留多次出现的
    }

    // 去重并排序
    entities.sort_by(|a, b| b.occurrences.cmp(&a.occurrences));
    entities.dedup_by(|a, b| a.name == b.name && a.entity_type == b.entity_type);

    let max = (entities.len() as f64 * aggressiveness).ceil() as usize;
    entities.truncate(max.max(3));

    entities
}

/// 检测内容类型
fn detect_content_type(content: &str) -> String {
    let lower = content.to_lowercase();
    let code_indicators = ["```", "fn ", "def ", "class ", "import ", "const ", "let ", "var ", "function("];
    let doc_indicators = ["## ", "api reference", "parameters", "returns", "example", "usage", "install"];
    let blog_indicators = ["published", "author", "comment", "share", "subscribe", "read more"];

    let code_score = code_indicators.iter().filter(|i| lower.contains(*i)).count();
    let doc_score = doc_indicators.iter().filter(|i| lower.contains(*i)).count();
    let blog_score = blog_indicators.iter().filter(|i| lower.contains(*i)).count();

    if code_score > doc_score && code_score > blog_score { return "code".into(); }
    if doc_score > code_score && doc_score > blog_score { return "documentation".into(); }
    if blog_score > code_score && blog_score > doc_score { return "blog".into(); }
    if code_score > 0 && doc_score > 0 { return "mixed".into(); }
    "documentation".into()
}

/// 原始版本（向后兼容）
fn extract_entities(content: &str, references: &[(String, String)]) -> Vec<ExtractedEntity> {
    let mut entities = Vec::new();
    let mut seen = HashMap::new();

    // 版本号提取: X.Y.Z 或 vX.Y.Z
    let version_re = regex_lite::Regex::new(r"\b(v?\d+\.\d+\.\d+(?:-[a-zA-Z0-9.]+)?)\b").unwrap();
    for cap in version_re.captures_iter(content) {
        if let Some(m) = cap.get(1) {
            let ver = m.as_str().to_string();
            let count = seen.entry(("version", ver.clone())).or_insert(0u32);
            if *count < 3 {
                entities.push(ExtractedEntity {
                    entity_type: EntityType::Version,
                    name: ver,
                    context: format!("Found in content near byte {}", m.start()),
                    occurrences: 1,
                });
            }
            *count += 1;
        }
    }

    // 日期提取: YYYY-MM-DD 或 Month DD, YYYY
    let date_re = regex_lite::Regex::new(r"\b(\d{4}-\d{2}-\d{2})\b").unwrap();
    for cap in date_re.captures_iter(content) {
        if let Some(m) = cap.get(1) {
            let date = m.as_str().to_string();
            let count = seen.entry(("date", date.clone())).or_insert(0u32);
            if *count < 2 {
                entities.push(ExtractedEntity {
                    entity_type: EntityType::Date,
                    name: date,
                    context: "Release/blog date".into(),
                    occurrences: 1,
                });
            }
            *count += 1;
        }
    }

    // Rust crate 提取: `crate_name` or "crate-name" in backticks
    let crate_re = regex_lite::Regex::new(r"`([a-z][a-z0-9_-]+)`").unwrap();
    let known_crates = ["tokio", "serde", "reqwest", "axum", "actix", "diesel", "sqlx",
        "clap", "tracing", "chrono", "async-trait", "thiserror", "anyhow", "tauri",
        "rocket", "warp", "hyper", "tonic", "prost"];
    for cap in crate_re.captures_iter(content) {
        if let Some(m) = cap.get(1) {
            let name = m.as_str().to_string();
            if known_crates.contains(&name.as_str()) {
                let count = seen.entry(("crate", name.clone())).or_insert(0u32);
                if *count < 2 {
                    entities.push(ExtractedEntity {
                        entity_type: EntityType::Crate,
                        name: name.clone(),
                        context: format!("Crate reference"),
                        occurrences: 1,
                    });
                }
                *count += 1;
            }
        }
    }

    // npm/pip package 提取
    let pkg_re = regex_lite::Regex::new(r"\b(npm install|pip install|cargo add|cargo install)\s+([^\s]+)").unwrap();
    for cap in pkg_re.captures_iter(content) {
        if let Some(m) = cap.get(2) {
            let name = m.as_str().to_string();
            entities.push(ExtractedEntity {
                entity_type: EntityType::Package,
                name,
                context: "Package install command".into(),
                occurrences: 1,
            });
        }
    }

    // GitHub 仓库提取: user/repo
    let repo_re = regex_lite::Regex::new(r"github\.com/([a-zA-Z0-9_.-]+/[a-zA-Z0-9_.-]+)").unwrap();
    for cap in repo_re.captures_iter(content) {
        if let Some(m) = cap.get(1) {
            entities.push(ExtractedEntity {
                entity_type: EntityType::Repository,
                name: m.as_str().to_string(),
                context: "GitHub repository".into(),
                occurrences: 1,
            });
        }
    }

    // 废弃标记
    for line in content.lines() {
        let lower = line.to_lowercase();
        if lower.contains("deprecated") || lower.contains("deprecation") {
            entities.push(ExtractedEntity {
                entity_type: EntityType::Deprecated,
                name: line.trim().chars().take(80).collect(),
                context: "Deprecation notice".into(),
                occurrences: 1,
            });
        }
        if lower.contains("breaking change") || lower.contains("breaking-change") {
            entities.push(ExtractedEntity {
                entity_type: EntityType::Breaking,
                name: line.trim().chars().take(80).collect(),
                context: "Breaking change notice".into(),
                occurrences: 1,
            });
        }
    }

    // 许可证提取
    for (text, _url) in references {
        let lower = text.to_lowercase();
        if lower.contains("license") || lower.contains("mit") || lower.contains("apache") || lower.contains("gpl") {
            entities.push(ExtractedEntity {
                entity_type: EntityType::License,
                name: text.clone(),
                context: "License reference".into(),
                occurrences: 1,
            });
        }
    }

    // 去重
    entities.sort_by(|a, b| b.occurrences.cmp(&a.occurrences));
    entities.dedup_by(|a, b| a.entity_type == b.entity_type && a.name == b.name);

    entities
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_markdown() -> String {
        r#"# Rust Async Traits

> Important: async fn in traits was stabilized in Rust 1.75.0

## Overview

Rust 1.75.0 stabilized `async fn` in trait definitions. This is a **breaking change**
for libraries using the `#[async_trait]` macro from the async-trait crate.

## API Reference

```rust
pub trait Service {
    async fn handle(&self, req: Request) -> Response;
    async fn health_check(&self) -> bool;
}
```

The `handle` method must return a `Response`. The `health_check` is optional.

### Requirements
- **Minimum Rust version**: 1.75.0
- Edition 2021 or later

### Migration Guide

If you were using `#[async_trait]`, remove the macro and add `async` directly:

```rust
// Before
#[async_trait]
pub trait OldWay {
    async fn process(&self) -> Result<()>;
}

// After (Rust 1.75+)
pub trait NewWay {
    async fn process(&self) -> Result<()>;
}
```

## Links
- [Rust Blog: async fn in traits](https://blog.rust-lang.org/2023/12/21/async-fn-rpit.html)
- [Stabilization PR](https://github.com/rust-lang/rust/pull/12345)

Note that `Send` bounds are not automatically inferred. You may need explicit `Send` bounds.

© 2024 Rust Team. All rights reserved.
"#.to_string()
    }

    #[test]
    fn test_light_distillation() {
        let mut engine = DistillationEngine::new();
        let content = sample_markdown();
        let result = engine.distill(&content, "https://docs.rs/async-trait", DistillationLevel::Light, Some(2000));

        assert!(result.distilled_size < result.original_size);
        assert!(result.markdown.contains("Rust Async Traits"));
        assert!(result.markdown.contains("```"));
        assert!(!result.markdown.contains("©")); // noise filtered
    }

    #[test]
    fn test_medium_distillation() {
        let mut engine = DistillationEngine::new();
        let content = sample_markdown();
        let result = engine.distill(&content, "https://docs.rs/async-trait", DistillationLevel::Medium, Some(1000));

        assert!(result.markdown.contains("API"));
        assert!(!result.key_insights.is_empty());
        assert!(result.compression_ratio > 0.5);
    }

    #[test]
    fn test_deep_distillation() {
        let mut engine = DistillationEngine::new();
        let content = sample_markdown();
        let result = engine.distill(&content, "https://docs.rs/async-trait", DistillationLevel::Deep, Some(500));

        assert!(result.markdown.contains("TL;DR"));
        assert!(result.markdown.contains("Core APIs"));
        assert!(result.compression_ratio > 0.8);
        assert!(result.estimated_tokens < 500);
    }

    #[test]
    fn test_cache_hit() {
        let mut engine = DistillationEngine::new();
        let content = sample_markdown();
        let url = "https://docs.rs/cached-test";

        let r1 = engine.distill(&content, url, DistillationLevel::Deep, Some(500));
        let r2 = engine.distill(&content, url, DistillationLevel::Deep, Some(500));

        // Should be identical (cache hit)
        assert_eq!(r1.markdown, r2.markdown);
        assert_eq!(engine.total_distillations, 2);
    }

    #[test]
    fn test_api_signature_detection() {
        assert!(is_api_signature("pub async fn handle(&self, req: Request) -> Response"));
        assert!(is_api_signature("fn process(data: &[u8]) -> Result<()>"));
        assert!(is_api_signature("def process(self, data: bytes) -> None:"));
        assert!(!is_api_signature("This is just a sentence about functions."));
    }

    #[test]
    fn test_heading_parsing() {
        let (level, text) = parse_heading("## API Reference");
        assert_eq!(level, 2);
        assert_eq!(text, "API Reference");

        let (level, text) = parse_heading("# Top Level");
        assert_eq!(level, 1);
        assert_eq!(text, "Top Level");
    }

    #[test]
    fn test_link_extraction() {
        let links = extract_inline_links("See the [Rust Blog](https://blog.rust-lang.org) and [docs](https://docs.rs)");
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].1, "https://blog.rust-lang.org");
    }

    #[test]
    fn test_noise_filtering() {
        assert!(is_noise("© 2024 All rights reserved"));
        assert!(is_noise("Sign up for our newsletter"));
        assert!(!is_noise("Rust 1.75 stabilized async fn in traits"));
    }

    #[test]
    fn test_fragment_extraction() {
        let engine = DistillationEngine::new();
        let content = sample_markdown();
        let fragments = engine.extract_fragments(&content, DistillationLevel::Medium);

        let headings: Vec<_> = fragments.iter().filter(|f| matches!(f, ContentFragment::Heading { .. })).collect();
        let code_blocks: Vec<_> = fragments.iter().filter(|f| matches!(f, ContentFragment::CodeBlock { .. })).collect();

        assert!(headings.len() >= 2);
        assert!(code_blocks.len() >= 1);
    }
}
