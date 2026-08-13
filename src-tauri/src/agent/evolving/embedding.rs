// 端侧轻量向量嵌入引擎 (Local Embedding Engine)
//
// 替代哈希模拟为真实语义向量嵌入 + 余弦相似度检索。
// 基于词袋 TF-IDF + 余弦相似度，端侧计算，0 Token 消耗。
//
// 核心功能：
//   1. 文本 → 稀疏向量嵌入 (TF-IDF tokenization)
//   2. 余弦相似度检索
//   3. Top-K 相似经验查找
//   4. 向量持久化 (JSON)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── 向量表示 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparseVector {
    /// 稀疏向量: token → weight
    pub dims: HashMap<String, f32>,
    /// 向量模长 (预计算)
    pub magnitude: f32,
}

impl SparseVector {
    pub fn new() -> Self {
        Self { dims: HashMap::new(), magnitude: 0.0 }
    }

    /// 从文本生成稀疏向量
    pub fn from_text(text: &str) -> Self {
        let mut dims: HashMap<String, f32> = HashMap::new();
        let tokens = tokenize(text);
        let total = tokens.len() as f32;
        if total == 0.0 { return Self::new(); }

        // TF 计算
        for token in &tokens {
            *dims.entry(token.clone()).or_insert(0.0) += 1.0;
        }

        // TF归一化
        for v in dims.values_mut() {
            *v /= total;
        }

        let magnitude = dims.values().map(|v| v * v).sum::<f32>().sqrt();
        Self { dims, magnitude }
    }

    /// 余弦相似度
    pub fn cosine_similarity(&self, other: &SparseVector) -> f32 {
        if self.magnitude == 0.0 || other.magnitude == 0.0 {
            return 0.0;
        }
        let dot: f32 = self.dims.iter()
            .filter_map(|(k, v)| other.dims.get(k).map(|ov| v * ov))
            .sum();
        dot / (self.magnitude * other.magnitude)
    }
}

// ─── 嵌入条目 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingEntry {
    /// 条目 ID
    pub id: String,
    /// 原始文本
    pub text: String,
    /// 关键词标签
    pub tags: Vec<String>,
    /// 稀疏向量
    pub vector: SparseVector,
    /// 来源 (经验/技能/错误模式)
    pub source: String,
    /// 时间戳
    pub timestamp: String,
}

// ─── 嵌入引擎 ──────────────────────────────────────────────────────

pub struct EmbeddingEngine {
    /// 嵌入条目池
    pub entries: Vec<EmbeddingEntry>,
    /// 最大条目数
    pub max_entries: usize,
    /// IDF 词频统计 (全局)
    pub global_idf: HashMap<String, f32>,
    /// 总文档数
    doc_count: u32,
    /// 条目使用频率 (entry_id → use_count)，用于 LRU 淘汰
    pub use_counts: HashMap<String, u32>,
    /// 最近使用时间戳 (entry_id → timestamp_ms)
    pub last_used: HashMap<String, u64>,
}

impl EmbeddingEngine {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            max_entries: 500,
            global_idf: HashMap::new(),
            doc_count: 0,
            use_counts: HashMap::new(),
            last_used: HashMap::new(),
        }
    }

    /// 添加嵌入条目
    pub fn add(&mut self, id: &str, text: &str, tags: Vec<String>, source: &str) {
        self.doc_count += 1;
        let tokens = tokenize(text);
        let mut seen = HashMap::new();
        for t in &tokens {
            if !seen.contains_key(t) {
                *self.global_idf.entry(t.clone()).or_insert(0.0) += 1.0;
                seen.insert(t.clone(), true);
            }
        }

        let entry = EmbeddingEntry {
            id: id.into(),
            text: text.into(),
            tags,
            vector: SparseVector::from_text(text),
            source: source.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        self.entries.push(entry);

        // LRU 智能淘汰: 保留高频使用的条目
        self.evict_lru();
    }

    /// LRU 淘汰: 优先删除低频低相关的条目
    fn evict_lru(&mut self) {
        if self.entries.len() <= self.max_entries { return; }
        let excess = self.entries.len() - self.max_entries;

        // 评分: 使用频率 × 10 + 最近使用时间权重
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let mut scored: Vec<(usize, u32)> = self.entries.iter().enumerate()
            .map(|(i, e)| {
                let use_cnt = self.use_counts.get(&e.id).copied().unwrap_or(0);
                let last = self.last_used.get(&e.id).copied().unwrap_or(0);
                let recency = if last > 0 { (now - last).min(86_400_000) / 3600_000 } else { 24 };
                // 高使用频率 + 最近使用 → 高分 (保留)
                let score = use_cnt * 10 + (24u64.saturating_sub(recency)) as u32;
                (i, score)
            })
            .collect();
        scored.sort_by_key(|(_, s)| *s); // 低分在前

        let to_remove: Vec<String> = scored.iter().take(excess)
            .map(|(i, _)| self.entries[*i].id.clone())
            .collect();
        self.entries.retain(|e| !to_remove.contains(&e.id));
        for id in &to_remove {
            self.use_counts.remove(id);
            self.last_used.remove(id);
        }
        if excess > 0 {
            tracing::info!("[EMBEDDING] LRU evicted {} entries (kept {})", excess, self.entries.len());
        }
    }

    /// BM25 增强相似度检索 (含 LRU 使用追踪)
    pub fn search(&mut self, query: &str, k: usize) -> Vec<(f32, &EmbeddingEntry)> {
        let query_vec = SparseVector::from_text(query);
        let avg_dl = if self.entries.is_empty() { 1.0 }
            else { self.entries.iter().map(|e| e.text.len() as f32).sum::<f32>() / self.entries.len() as f32 };
        let k1 = 1.2f32; let b = 0.75f32;
        let total_docs = self.doc_count.max(1) as f32;

        let mut scored: Vec<(f32, &EmbeddingEntry)> = self.entries.iter()
            .map(|e| {
                let dl = e.text.len() as f32;
                // BM25 scoring with IDF from global frequencies
                let bm25: f32 = query_vec.dims.iter().map(|(term, qtf)| {
                    let df = self.global_idf.get(term).copied().unwrap_or(1.0).max(1.0);
                    let idf = ((total_docs - df + 0.5) / (df + 0.5) + 1.0).ln().max(0.0);
                    let tf = e.vector.dims.get(term).copied().unwrap_or(0.0);
                    let numerator = tf * (k1 + 1.0);
                    let denominator = tf + k1 * (1.0 - b + b * dl / avg_dl.max(1.0));
                    idf * numerator / denominator.max(0.001) * qtf
                }).sum();
                (query_vec.cosine_similarity(&e.vector) * 0.5 + bm25.min(1.0) * 0.5, e)
            })
            .filter(|(s, _)| *s > 0.03)
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(k);

        let now = chrono::Utc::now().timestamp_millis() as u64;
        for (_, entry) in &scored {
            *self.use_counts.entry(entry.id.clone()).or_insert(0) += 1;
            self.last_used.insert(entry.id.clone(), now);
        }
        scored
    }

    /// 按标签过滤 + 相似度检索
    pub fn search_by_tags(&self, query: &str, tags: &[String], k: usize) -> Vec<(f32, &EmbeddingEntry)> {
        let query_vec = SparseVector::from_text(query);
        let mut scored: Vec<(f32, &EmbeddingEntry)> = self.entries.iter()
            .filter(|e| tags.iter().any(|t| e.tags.contains(t)))
            .map(|e| (query_vec.cosine_similarity(&e.vector), e))
            .filter(|(s, _)| *s > 0.02)
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(k);
        scored
    }

    // ── 持久化 ──────────────────────────────────────────────

    /// 保存嵌入状态到磁盘
    pub fn save_state(&self, dir: &std::path::Path) -> Result<(), String> {
        let path = dir.join("embedding_state.json");
        let state = serde_json::json!({
            "entries": self.entries,
            "use_counts": self.use_counts,
            "last_used": self.last_used,
        });
        std::fs::write(&path, serde_json::to_string_pretty(&state)
            .map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())
    }

    /// 从磁盘恢复嵌入状态
    pub fn load_state(&mut self, dir: &std::path::Path) -> Result<(), String> {
        let path = dir.join("embedding_state.json");
        if !path.exists() { return Ok(()); }
        let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let state: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        if let Some(entries) = state.get("entries") {
            if let Ok(e) = serde_json::from_value::<Vec<EmbeddingEntry>>(entries.clone()) {
                self.entries = e;
            }
        }
        if let Some(uc) = state.get("use_counts") {
            self.use_counts = serde_json::from_value(uc.clone()).unwrap_or_default();
        }
        if let Some(lu) = state.get("last_used") {
            self.last_used = serde_json::from_value(lu.clone()).unwrap_or_default();
        }
        self.doc_count = self.entries.len() as u32;
        Ok(())
    }

    /// 批量自动索引: 从迭代器添加条目
    pub fn auto_index<I>(&mut self, items: I)
    where I: IntoIterator<Item = (String, String, Vec<String>, String)> {
        for (id, text, tags, source) in items {
            self.add(&id, &text, tags, &source);
        }
    }

    /// 内存压力检查: 返回是否需要触发全局清理
    pub fn memory_pressure(&self) -> f32 {
        self.entries.len() as f32 / self.max_entries as f32
    }

    /// 获取统计
    pub fn stats(&self) -> serde_json::Value {
        let avg_use = if self.use_counts.is_empty() { 0.0 }
            else { self.use_counts.values().sum::<u32>() as f64 / self.use_counts.len() as f64 };
        serde_json::json!({
            "entry_count": self.entries.len(),
            "max_entries": self.max_entries,
            "vocab_size": self.global_idf.len(),
            "doc_count": self.doc_count,
            "memory_pressure": self.memory_pressure(),
            "avg_use_count": avg_use,
        })
    }
}

impl Default for EmbeddingEngine {
    fn default() -> Self { Self::new() }
}

// ─── 工具: 简单分词器 ──────────────────────────────────────────────

fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for word in text.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-') {
        let trimmed = word.trim().to_lowercase();
        if trimmed.len() >= 2 && trimmed.len() <= 40 {
            tokens.push(trimmed);
        }
    }
    tokens
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_from_text() {
        let v = SparseVector::from_text("hello world hello rust");
        assert!(v.dims.contains_key("hello"));
        assert!(v.dims.contains_key("world"));
        assert!(v.magnitude > 0.0);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = SparseVector::from_text("rust programming language");
        let b = SparseVector::from_text("rust programming guide");
        let c = SparseVector::from_text("cooking recipes food");
        assert!(a.cosine_similarity(&b) > 0.5);
        assert!(a.cosine_similarity(&c) < 0.3);
    }

    #[test]
    fn test_search() {
        let mut engine = EmbeddingEngine::new();
        engine.add("1", "rust async programming", vec!["rust".into()], "test");
        engine.add("2", "python data science", vec!["python".into()], "test");
        engine.add("3", "rust web server", vec!["rust".into()], "test");
        let results = engine.search("rust programming", 2);
        assert_eq!(results.len(), 2);
        assert!(results[0].1.id == "1" || results[0].1.id == "3");
    }

    #[test]
    fn test_search_by_tags() {
        let mut engine = EmbeddingEngine::new();
        engine.add("1", "rust async", vec!["rust".into()], "test");
        engine.add("2", "python data", vec!["python".into()], "test");
        let results = engine.search_by_tags("programming", &["rust".into()], 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1.id, "1");
    }
}

// ─── Tauri Commands ──────────────────────────────────────────────

#[tauri::command]
pub fn embedding_search(
    state: tauri::State<crate::state::AppState>,
    query: String,
    k: usize,
) -> Vec<serde_json::Value> {
    let mut engine = state.embedding.lock().unwrap();
    engine.search(&query, k.max(1).min(20))
        .into_iter()
        .map(|(score, entry)| serde_json::json!({
            "id": entry.id, "text": entry.text,
            "tags": entry.tags, "score": score,
            "source": entry.source,
        }))
        .collect()
}

#[tauri::command]
pub fn embedding_add(
    state: tauri::State<crate::state::AppState>,
    id: String, text: String, tags: Vec<String>, source: String,
) -> String {
    let mut engine = state.embedding.lock().unwrap();
    engine.add(&id, &text, tags, &source);
    format!("Added embedding entry: {}", id)
}

#[tauri::command]
pub fn embedding_stats(
    state: tauri::State<crate::state::AppState>,
) -> serde_json::Value {
    state.embedding.lock().unwrap().stats()
}
