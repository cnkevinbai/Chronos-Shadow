// 离线知识固化中枢 (Local Consolidator)
// 白皮书 §3.2：本地向量化 + SQLite 持久化 + 零 Token 闭环
//
// 当系统空闲时，Consolidator 在本地被静默唤醒：
// 1. 调用端侧 Embedding 引擎将经验对转化为向量嵌入
// 2. 持久化写入本地 SQLite/Vector 数据库
// 3. 更新技能树索引

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::Mutex;
use std::sync::Arc;
use super::extractor::DeltaExperience;
use super::embedding::EmbeddingEngine;

// ─── 2.0 规格的正反向增量经验对账单 ──────────────────────────────

/// 双向隔离评估用的增量经验 (Delta-Experience Pair) — 2.0 新字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvoDelta {
    /// 经验 ID
    pub experience_id: String,
    /// 触发场景的特征哈希（用于极速比对）
    pub context_trigger_hash: String,
    /// AI 因幻觉犯错的原始动作
    pub failed_llm_action: String,
    /// 经过操作员微调或 Verifier 成功自愈的正确动作
    pub correct_human_action: String,
    /// Token 沉没成本（本次省下的 Token 数）
    pub token_sunk_cost_saved: u32,
    /// 记忆权重分数：随着高频命中自动增发
    pub accuracy_weight: f32,
}

// ─── 类型定义 ──────────────────────────────────────────────────────

/// 固化后的本地技能条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidatedSkill {
    /// 技能 ID
    pub id: String,
    /// 技能名称
    pub name: String,
    /// 技能描述
    pub description: String,
    /// 来源经验 ID
    pub source_delta_id: String,
    /// 适用场景关键词
    pub tags: Vec<String>,
    /// 向量嵌入 (384-dim 简化)
    pub embedding: Vec<f32>,
    /// 置信度
    pub confidence: f32,
    /// 使用次数
    pub use_count: u32,
    /// 创建时间
    pub created_at: String,
}

/// 本地技能数据库
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDatabase {
    pub skills: Vec<ConsolidatedSkill>,
    pub version: u32,
}

// ─── 双向隔离验证沙盒 (2.0) ────────────────────────────────────

/// 本地经验验证与固化器
pub struct LocalConsolidator {
    pub active_memory_pool: Arc<Mutex<HashMap<String, EvoDelta>>>,
}

impl LocalConsolidator {
    pub fn new() -> Self {
        Self {
            active_memory_pool: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 核心功能：双向隔离反思验证
    ///
    /// 严防大模型将偶然写对的代码或幻觉总结为错误的"伪经验"，
    /// 避免后续污染全局科技树。
    pub async fn validate_and_commit_experience(
        &self,
        delta: EvoDelta,
    ) -> Result<bool, String> {
        tracing::info!(
            "[EVOLUTION SHIELD] Double-check validating memory segment: {}",
            delta.experience_id
        );

        // 1. 端侧白盒走查：检测有害或无效片段
        if delta.correct_human_action.contains("rm -rf")
            || delta.correct_human_action.is_empty()
        {
            return Err(
                "检测到有害或无效的负向记忆片段，启动端侧自我防卫截断拦截。".to_string(),
            );
        }

        // 2. 过滤无实质修正的经验
        if delta.failed_llm_action.trim() == delta.correct_human_action.trim() {
            return Ok(false); // 无变化，不值得记录
        }

        // 3. 本地评估通过：写入活跃记忆池
        let mut pool = self.active_memory_pool.lock().await;
        pool.insert(delta.context_trigger_hash.clone(), delta.clone());

        tracing::info!(
            "[EVOLUTION SUCCESS] Memory [id: {}] successfully consolidated into local database fly-wheel.",
            delta.experience_id
        );
        Ok(true)
    }

    /// 获取共享记忆池引用（供 Regulator 使用）
    pub fn memory_pool(&self) -> Arc<Mutex<HashMap<String, EvoDelta>>> {
        self.active_memory_pool.clone()
    }

    /// 持久化活跃记忆池到 SQLite（重启保留学习成果）
    pub fn save_state(&self, dir: &std::path::Path) -> Result<String, String> {
        let conn = open_sqlite(&dir.join("evolution.db"))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS memory (
                context_hash TEXT PRIMARY KEY,
                experience_id TEXT NOT NULL,
                failed_action TEXT NOT NULL,
                correct_action TEXT NOT NULL,
                token_saved INTEGER NOT NULL,
                accuracy_weight REAL NOT NULL
            );"
        ).map_err(|e| e.to_string())?;

        let pool = self.active_memory_pool.blocking_lock();
        let mut stmt = conn.prepare(
            "INSERT OR REPLACE INTO memory VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
        ).map_err(|e| e.to_string())?;
        for (hash, delta) in pool.iter() {
            stmt.execute(rusqlite::params![
                hash,
                delta.experience_id,
                delta.failed_llm_action,
                delta.correct_human_action,
                delta.token_sunk_cost_saved,
                delta.accuracy_weight,
            ]).map_err(|e| e.to_string())?;
        }
        Ok(format!("Evolution memory saved to SQLite: {} experiences", pool.len()))
    }

    /// 从 SQLite 恢复活跃记忆池
    pub fn load_state(&mut self, dir: &std::path::Path) -> Result<String, String> {
        let db = dir.join("evolution.db");
        if !db.exists() { return Ok("No saved evolution memory".into()); }
        let conn = open_sqlite(&db)?;
        let mut stmt = conn.prepare(
            "SELECT context_hash, experience_id, failed_action, correct_action, token_saved, accuracy_weight FROM memory"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, u32>(4)?,
                row.get::<_, f32>(5)?,
            ))
        }).map_err(|e| e.to_string())?;

        let mut pool = self.active_memory_pool.blocking_lock();
        pool.clear();
        let mut count = 0;
        for row in rows {
            if let Ok((hash, id, failed, correct, token, weight)) = row {
                pool.insert(hash.clone(), EvoDelta {
                    experience_id: id,
                    context_trigger_hash: hash,
                    failed_llm_action: failed,
                    correct_human_action: correct,
                    token_sunk_cost_saved: token,
                    accuracy_weight: weight,
                });
                count += 1;
            }
        }
        Ok(format!("Evolution memory loaded from SQLite: {} experiences", count))
    }
}

// ─── 技能固化中枢 (原版，向后兼容) ─────────────────────────────────

/// 知识固化中枢
pub struct Consolidator {
    /// 本地技能数据库
    pub db: SkillDatabase,
    /// 是否启用
    pub enabled: bool,
    /// Embedding 维度
    pub embedding_dim: usize,
    /// 嵌入引擎 — 真实语义相似度检索
    pub embedding: EmbeddingEngine,
}

impl Consolidator {
    pub fn new() -> Self {
        Self {
            db: SkillDatabase { skills: Vec::new(), version: 1 },
            enabled: true,
            embedding_dim: 384,
            embedding: EmbeddingEngine::new(),
        }
    }

    /// 将经验对固化为本地技能 (含向量嵌入)
    pub fn consolidate(&mut self, delta: &DeltaExperience) -> ConsolidatedSkill {
        let id = format!("skill-{:04}", self.db.skills.len() + 1);

        let mut tags: Vec<String> = delta.scope
            .split(&['/', '.', ' ', ':', ','][..])
            .filter(|s| !s.is_empty() && s.len() > 2)
            .map(|s| s.to_lowercase())
            .collect();
        tags.push(format!("{:?}", delta.trigger).to_lowercase());

        let embedding = self.feature_hash_embed(&delta.correction);

        // 添加到真实嵌入引擎
        self.embedding.add(&id, &delta.correction, tags.clone(), "consolidator");

        let skill = ConsolidatedSkill {
            id: id.clone(),
            name: format!("{}-{}", delta.scope, delta.trigger_name()),
            description: format!(
                "Learned from {}: {} → {}",
                delta.trigger_name(), delta.original_error, delta.correction
            ),
            source_delta_id: delta.id.clone(),
            tags,
            embedding,
            confidence: 0.75,
            use_count: 0,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        self.db.skills.push(skill.clone());
        self.db.version += 1;
        skill
    }

    /// 批量固化
    pub fn consolidate_batch(&mut self, deltas: &[DeltaExperience]) -> Vec<ConsolidatedSkill> {
        deltas.iter().map(|d| self.consolidate(d)).collect()
    }

    /// 检索相似历史经验（零 Token 经验重用）
    pub fn find_similar(&mut self, query: &str, top_k: usize) -> Vec<ConsolidatedSkill> {
        // 优先使用真实嵌入引擎搜索
        let embedding_results = self.embedding.search(query, top_k);
        if !embedding_results.is_empty() {
            let results: Vec<ConsolidatedSkill> = embedding_results.iter()
                .filter_map(|(_, entry)| {
                    self.db.skills.iter().find(|s| s.id == entry.id).cloned()
                })
                .collect();
            if !results.is_empty() {
                return results;
            }
        }

        // Fallback: 使用 feature-hashing 嵌入 + cosine 搜索
        let query_emb = self.feature_hash_embed(query);
        let mut scored: Vec<(f64, &ConsolidatedSkill)> = self.db.skills.iter()
            .map(|s| (cosine_similarity(&query_emb, &s.embedding), s))
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Collect result IDs first (drop immutable borrow)
        let top_ids: Vec<String> = scored.iter()
            .take(top_k)
            .filter(|(score, _)| *score > 0.5)
            .map(|(_, s)| s.id.clone())
            .collect();

        // Update use counts (mutable borrow)
        for id in &top_ids {
            if let Some(s) = self.db.skills.iter_mut().find(|s| &s.id == id) {
                s.use_count += 1;
            }
        }

        // Return cloned results
        let results: Vec<ConsolidatedSkill> = top_ids.iter()
            .filter_map(|id| self.db.skills.iter().find(|s| &s.id == id).cloned())
            .collect();

        tracing::info!(
            "[CONSOLIDATOR] Similarity search: '{}' → {} results (top_k={})",
            &query[..50.min(query.len())], results.len(), top_k
        );

        results
    }

    /// 生成硬性限制规则（注入 LLM Prompt 头部）
    pub fn generate_hard_constraints(&mut self, query: &str) -> String {
        let similar = self.find_similar(query, 3);
        if similar.is_empty() {
            return String::new();
        }

        let mut constraints = String::from("## ⚡ Evolution Hard Constraints (Zero-Token Local Experience)\n\n");
        for (i, skill) in similar.iter().enumerate() {
            constraints.push_str(&format!(
                "{}. **{}**: {}\n   - Tags: {}\n   - Used: {} times\n\n",
                i + 1,
                skill.name,
                skill.description,
                skill.tags.join(", "),
                skill.use_count,
            ));
        }
        constraints.push_str("⚠️ The above constraints are derived from local learned experience. Follow them strictly to avoid repeating past errors.\n");
        constraints
    }

    /// 统计
    pub fn stats(&self) -> ConsolidatorStats {
        let total_uses: u32 = self.db.skills.iter().map(|s| s.use_count).sum();
        ConsolidatorStats {
            total_skills: self.db.skills.len() as u64,
            db_version: self.db.version,
            total_skill_uses: total_uses as u64,
            estimated_tokens_saved: total_uses as u64 * 500, // ~500 tokens per reuse
        }
    }

    /// 持久化固化技能库到 SQLite + 嵌入状态
    pub fn save_state(&self, dir: &std::path::Path) -> Result<String, String> {
        let conn = open_sqlite(&dir.join("evolution_skills.db"))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS skills (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                source_delta_id TEXT NOT NULL,
                tags TEXT NOT NULL,
                embedding TEXT NOT NULL,
                confidence REAL NOT NULL,
                use_count INTEGER NOT NULL,
                created_at TEXT NOT NULL
            );"
        ).map_err(|e| e.to_string())?;

        let mut stmt = conn.prepare(
            "INSERT OR REPLACE INTO skills VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
        ).map_err(|e| e.to_string())?;
        for s in &self.db.skills {
            let tags = serde_json::to_string(&s.tags).unwrap_or_default();
            let embedding = serde_json::to_string(&s.embedding).unwrap_or_default();
            stmt.execute(rusqlite::params![
                s.id, s.name, s.description, s.source_delta_id,
                tags, embedding, s.confidence, s.use_count, s.created_at,
            ]).map_err(|e| e.to_string())?;
        }
        if let Err(e) = self.embedding.save_state(dir) {
            tracing::warn!("[CONSOLIDATOR] Embedding save failed: {}", e);
        }
        Ok(format!("Consolidator saved to SQLite: {} skills", self.db.skills.len()))
    }

    /// 从 SQLite 恢复固化技能库 + 嵌入状态
    pub fn load_state(&mut self, dir: &std::path::Path) -> Result<String, String> {
        let db = dir.join("evolution_skills.db");
        if db.exists() {
            let conn = open_sqlite(&db)?;
            let mut stmt = conn.prepare(
                "SELECT id, name, description, source_delta_id, tags, embedding, confidence, use_count, created_at FROM skills"
            ).map_err(|e| e.to_string())?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, f32>(6)?,
                    row.get::<_, u32>(7)?,
                    row.get::<_, String>(8)?,
                ))
            }).map_err(|e| e.to_string())?;
            self.db.skills.clear();
            for row in rows {
                if let Ok((id, name, desc, src, tags_json, emb_json, conf, use_count, created)) = row {
                    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
                    let embedding: Vec<f32> = serde_json::from_str(&emb_json).unwrap_or_default();
                    self.db.skills.push(ConsolidatedSkill {
                        id, name, description: desc, source_delta_id: src, tags, embedding,
                        confidence: conf, use_count, created_at: created,
                    });
                }
            }
        }
        if let Err(e) = self.embedding.load_state(dir) {
            tracing::warn!("[CONSOLIDATOR] Embedding load failed: {}", e);
        }
        Ok(format!("Consolidator loaded from SQLite: {} skills", self.db.skills.len()))
    }

    /// 特征哈希嵌入（hashing trick）— 语义有意义：相似文本共享 token → 相似向量。
    ///
    /// 替代旧的 DefaultHasher 伪随机向量（非语义、跨 Rust 版本不稳定）。
    /// 使用稳定的 FNV-1a 哈希把每个 token 映射到固定维度并累加符号，
    /// 余弦相似度对语义相似的文本具备区分度。
    fn feature_hash_embed(&self, text: &str) -> Vec<f32> {
        let mut vec = vec![0.0f32; self.embedding_dim];
        if self.embedding_dim == 0 {
            return vec;
        }

        for token in embed_tokens(text) {
            let h = fnv1a(token.as_bytes());
            let idx = (h % self.embedding_dim as u64) as usize;
            let sign = if (h >> 63) & 1 == 0 { 1.0f32 } else { -1.0f32 };
            vec[idx] += sign;
        }

        // L2 normalize
        let norm: f32 = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut vec {
                *v /= norm;
            }
        }
        vec
    }
}

/// 稳定的 FNV-1a 64-bit 哈希（跨 Rust 版本稳定，用于特征哈希嵌入）
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// 特征哈希用分词器（与 embedding.rs 的 tokenize 规则一致）
fn embed_tokens(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .filter_map(|w| {
            let t = w.trim().to_lowercase();
            if t.len() >= 2 && t.len() <= 40 { Some(t) } else { None }
        })
        .collect()
}

impl Default for Consolidator {
    fn default() -> Self { Self::new() }
}

impl DeltaExperience {
    fn trigger_name(&self) -> &str {
        match self.trigger {
            super::extractor::DeltaTrigger::UserCorrection => "UserCorrection",
            super::extractor::DeltaTrigger::SelfHealing => "SelfHealing",
            super::extractor::DeltaTrigger::OmniRewind => "OmniRewind",
            super::extractor::DeltaTrigger::RedlineFuse => "RedlineFuse",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidatorStats {
    pub total_skills: u64,
    pub db_version: u32,
    pub total_skill_uses: u64,
    pub estimated_tokens_saved: u64,
}

/// 余弦相似度
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() { return 0.0; }
    let dot: f64 = a.iter().zip(b).map(|(x, y)| (*x as f64) * (*y as f64)).sum();
    let norm_a: f64 = a.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 { 0.0 } else { dot / (norm_a * norm_b) }
}

/// 打开（必要时创建父目录）SQLite 数据库连接
fn open_sqlite(path: &std::path::Path) -> Result<rusqlite::Connection, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    rusqlite::Connection::open(path).map_err(|e| e.to_string())
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_delta(id: &str) -> EvoDelta {
        EvoDelta {
            experience_id: id.into(),
            context_trigger_hash: format!("hash-{}", id),
            failed_llm_action: "unwrap()".into(),
            correct_human_action: "? 运算符".into(),
            token_sunk_cost_saved: 100,
            accuracy_weight: 0.8,
        }
    }

    #[test]
    fn test_local_consolidator_memory_roundtrip() {
        let dir = std::env::temp_dir().join("chronos_evo_mem_test");
        std::fs::create_dir_all(&dir).unwrap();

        let lc = LocalConsolidator::new();
        {
            let mut pool = lc.active_memory_pool.blocking_lock();
            pool.insert("hash-exp-1".into(), sample_delta("exp-1"));
        }
        lc.save_state(&dir).unwrap();

        let mut lc2 = LocalConsolidator::new();
        lc2.load_state(&dir).unwrap();
        let pool = lc2.active_memory_pool.blocking_lock();
        assert_eq!(pool.len(), 1);
        assert!(pool.contains_key("hash-exp-1"));
        assert_eq!(pool["hash-exp-1"].correct_human_action, "? 运算符");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_consolidator_skills_roundtrip() {
        let dir = std::env::temp_dir().join("chronos_evo_skills_test");
        std::fs::create_dir_all(&dir).unwrap();

        let mut c = Consolidator::new();
        c.db.skills.push(ConsolidatedSkill {
            id: "skill-0001".into(),
            name: "test-skill".into(),
            description: "learned".into(),
            source_delta_id: "delta-1".into(),
            tags: vec!["test".into()],
            embedding: vec![0.1, 0.2, 0.3],
            confidence: 0.75,
            use_count: 0,
            created_at: "2026-01-01T00:00:00Z".into(),
        });
        c.save_state(&dir).unwrap();

        let mut c2 = Consolidator::new();
        c2.load_state(&dir).unwrap();
        assert_eq!(c2.db.skills.len(), 1);
        assert_eq!(c2.db.skills[0].id, "skill-0001");
        assert_eq!(c2.db.skills[0].embedding.len(), 3);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
