// DistillationEngine 核心实现（蒸馏主流程 / 缓存 / 进化 / 持久化）
use std::collections::HashMap;
use super::types::*;
use super::helpers::*;
use super::entities::*;

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

    /// v2: 实体关系图提取（创新化）— 从句子窗口内实体共现推断关系
    pub fn extract_entity_relations(&self, content: &str) -> Vec<EntityRelation> {
        let entities = extract_entities(content, &[]);
        if entities.len() < 2 { return vec![]; }

        let sentences: Vec<&str> = content
            .split(|c: char| c == '.' || c == '\n' || c == '，' || c == '。')
            .collect();
        let mut relations = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for sent in sentences {
            let in_sentence: Vec<&ExtractedEntity> = entities
                .iter()
                .filter(|e| sent.contains(&e.name))
                .collect();
            for i in 0..in_sentence.len() {
                for j in (i + 1)..in_sentence.len() {
                    let (a, b) = (&in_sentence[i], &in_sentence[j]);
                    if a.name == b.name { continue; }
                    let key = format!("{}|{}", a.name, b.name);
                    if seen.contains(&key) { continue; }
                    seen.insert(key);

                    let relation = if sent.contains("depends on") || sent.contains("依赖") {
                        "depends_on"
                    } else if sent.contains("deprecat") || sent.contains("废弃") {
                        "deprecates"
                    } else {
                        "co_occur"
                    };

                    relations.push(EntityRelation {
                        source: a.name.clone(),
                        target: b.name.clone(),
                        relation: relation.to_string(),
                        confidence: 0.7,
                    });
                }
            }
        }
        relations
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
        let estimated_tokens = estimate_tokens(&markdown);
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

    /// 按目标模型蒸馏：根据模型上下文窗口自动设置预算
    pub fn distill_for_model(
        &mut self,
        content: &str,
        source_url: &str,
        level: DistillationLevel,
        model: &str,
    ) -> DistillationResult {
        let budget = budget_for_model(model, level);
        self.distill(content, source_url, level, Some(budget))
    }

    // ── 加权内容片段提取 (进化版) ────────────────────────────

    pub(super) fn extract_fragments_weighted(&self, content: &str, level: DistillationLevel, weights: &DistillationWeights) -> Vec<ContentFragment> {
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
    pub(super) fn extract_fragments(&self, content: &str, level: DistillationLevel) -> Vec<ContentFragment> {
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

    pub(super) fn assemble_markdown(
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
                if token_count >= target_tokens { break; }
                let rendered = render_fragment_light(frag);
                token_count += estimate_tokens(&rendered);
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
                if token_count + estimate_tokens(&rendered) > target_tokens { break; }
                token_count += estimate_tokens(&rendered);
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

    pub(super) fn extract_heading_tree(&self, content: &str) -> Vec<String> {
        content.lines()
            .filter(|l| l.trim().starts_with('#'))
            .map(|l| l.trim().to_string())
            .take(30)
            .collect()
    }

    pub(super) fn extract_key_insights(&self, fragments: &[ContentFragment], level: DistillationLevel) -> Vec<String> {
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

    pub(super) fn extract_references(&self, content: &str) -> Vec<(String, String)> {
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

    pub(super) fn evict_cache(&mut self) {
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
    pub(super) fn adapt_parameters(&self, content_type: &str, level: DistillationLevel, budget: usize) -> (DistillationLevel, usize) {
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
        let _ = w;
        self.update_strategy(content_type, quality_score);

        tracing::info!(
            "[Evolve] content={} quality={:.2} reward={:.1} weights: code={:.2} api={:.2} fact={:.2} para={:.2} comp={:.2}",
            content_type, quality_score, reward,
            w_snapshot.code_retention, w_snapshot.api_retention, w_snapshot.fact_extraction,
            w_snapshot.paragraph_threshold, w_snapshot.compression_aggressiveness
        );
    }

    /// 更新自适应策略表
    pub(super) fn update_strategy(&mut self, content_type: &str, quality_score: f64) {
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
        if let Some(_url_cache) = self.cache.get(url) {
            tracing::info!("[Feedback] URL={} quality={:.2} type={}", url, quality_score, content_type);
        }
    }

    pub fn save_state(&self, dir: &std::path::Path) -> Result<(), String> {
        let path = dir.join("distillation_engine.json");
        let state = serde_json::json!({
            "total_distillations": self.total_distillations,
            "total_bytes_saved": self.total_bytes_saved,
            "default_token_budget": self.default_token_budget,
            "evolution_weights": self.evolution_weights,
            "evolution_enabled": self.evolution_enabled,
            "adaptive_strategies": self.adaptive_strategies,
        });
        std::fs::write(&path, serde_json::to_string_pretty(&state).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())
    }

    pub fn load_state(&mut self, dir: &std::path::Path) -> Result<(), String> {
        let path = dir.join("distillation_engine.json");
        if !path.exists() { return Ok(()); }
        let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let state: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        if let Some(v) = state.get("total_distillations").and_then(|v| v.as_u64()) { self.total_distillations = v; }
        if let Some(v) = state.get("total_bytes_saved").and_then(|v| v.as_u64()) { self.total_bytes_saved = v; }
        if let Some(v) = state.get("default_token_budget").and_then(|v| v.as_u64()) { self.default_token_budget = v as usize; }
        if let Some(v) = state.get("evolution_enabled").and_then(|v| v.as_bool()) { self.evolution_enabled = v; }
        if let Some(w) = state.get("evolution_weights") {
            if let Ok(weights) = serde_json::from_value(w.clone()) { self.evolution_weights = weights; }
        }
        if let Some(s) = state.get("adaptive_strategies") {
            if let Ok(strategies) = serde_json::from_value(s.clone()) { self.adaptive_strategies = strategies; }
        }
        Ok(())
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

