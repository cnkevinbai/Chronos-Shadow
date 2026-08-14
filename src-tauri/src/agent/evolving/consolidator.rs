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

/// 本地技能数据库 (模拟 SQLite)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDatabase {
    pub skills: Vec<ConsolidatedSkill>,
    pub version: u32,
}

// ─── 双向隔离验证沙盒 (2.0) ────────────────────────────────────

/// 本地经验验证与固化器
pub struct LocalConsolidator {
    pub db_path: std::path::PathBuf,
    pub active_memory_pool: Arc<Mutex<HashMap<String, EvoDelta>>>,
}

impl LocalConsolidator {
    pub fn new(sandbox_root: &std::path::Path) -> Self {
        let db = sandbox_root.join(".chronos_storage/evolution.db");
        Self {
            db_path: db,
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

    /// 持久化活跃记忆池（重启保留学习成果）
    pub fn save_state(&self, dir: &std::path::Path) -> Result<String, String> {
        let path = dir.join("evolution_memory.json");
        let pool = self.active_memory_pool.blocking_lock();
        let json = serde_json::to_string_pretty(&*pool).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
        Ok(format!("Evolution memory saved: {} experiences", pool.len()))
    }

    /// 从磁盘恢复活跃记忆池
    pub fn load_state(&mut self, dir: &std::path::Path) -> Result<String, String> {
        let path = dir.join("evolution_memory.json");
        if !path.exists() { return Ok("No saved evolution memory".into()); }
        let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let pool: HashMap<String, EvoDelta> = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        let mut guard = self.active_memory_pool.blocking_lock();
        *guard = pool;
        let count = guard.len();
        Ok(format!("Evolution memory loaded: {} experiences", count))
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

        let embedding = self.mock_embed(&delta.correction);

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

        // Fallback: 使用旧的 mock_embed + cosine 搜索
        let query_emb = self.mock_embed(query);
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

    /// 持久化固化技能库 + 嵌入状态
    pub fn save_state(&self, dir: &std::path::Path) -> Result<String, String> {
        let path = dir.join("evolution_skills.json");
        let json = serde_json::to_string_pretty(&self.db).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
        let _ = self.embedding.save_state(dir);
        Ok(format!("Consolidator saved: {} skills", self.db.skills.len()))
    }

    /// 从磁盘恢复固化技能库 + 嵌入状态
    pub fn load_state(&mut self, dir: &std::path::Path) -> Result<String, String> {
        let path = dir.join("evolution_skills.json");
        if path.exists() {
            let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            self.db = serde_json::from_str::<SkillDatabase>(&json).map_err(|e| e.to_string())?;
        }
        let _ = self.embedding.load_state(dir);
        Ok(format!("Consolidator loaded: {} skills", self.db.skills.len()))
    }

    /// 模拟向量嵌入（384-dim 归一化伪随机向量）
    fn mock_embed(&self, text: &str) -> Vec<f32> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let bytes = text.as_bytes();
        let mut vec = Vec::with_capacity(self.embedding_dim);

        for i in 0..self.embedding_dim {
            let mut hasher = DefaultHasher::new();
            bytes.hash(&mut hasher);
            i.hash(&mut hasher);
            let h = hasher.finish();
            // Normalize to [-1, 1]
            let val = ((h as f64 / u64::MAX as f64) * 2.0 - 1.0) as f32;
            vec.push(val);
        }

        // L2 normalize
        let norm: f32 = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            vec.iter_mut().for_each(|v| *v /= norm);
        }

        vec
    }
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
