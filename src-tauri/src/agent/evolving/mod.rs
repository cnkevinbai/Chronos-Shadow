// Agent 自我学习进化成长系统 (Self-Evolving Learning Core)
// 白皮书 §3：影子记忆提取 + 离线知识固化 + 零Token经验重用
// 2.0 升级：双向隔离验证 + CLAUDE.md 硬契约热编译器

pub mod extractor;
pub mod consolidator;
pub mod regulator;
pub mod agent_quality;
pub mod embedding;

use extractor::{ExtractorStats, ShadowExtractor};
use consolidator::{ConsolidatedSkill, Consolidator, ConsolidatorStats, LocalConsolidator, EvoDelta};
use regulator::EvolvingRegulator;
use serde::{Deserialize, Serialize};


/// 进化引擎 — 统一封装提取器 + 固化器 + 隔离验证 + 热编译器
pub struct EvolutionEngine {
    pub extractor: ShadowExtractor,
    pub consolidator: Consolidator,
    pub local_consolidator: LocalConsolidator,
    pub regulator: Option<EvolvingRegulator>,
    pub state: EvolutionState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvolutionState {
    Idle,
    Extracting,
    Consolidating,
    Ready,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionStats {
    pub total_deltas: u64,
    pub consolidated_skills: u64,
    pub tokens_saved_by_reuse: u64,
    pub state: String,
    pub extractor: ExtractorStats,
    pub consolidator: ConsolidatorStats,
}

impl EvolutionEngine {
    pub fn new(sandbox_root: Option<std::path::PathBuf>) -> Self {
        let root = sandbox_root.unwrap_or_else(|| std::path::PathBuf::from("."));
        let local_cons = LocalConsolidator::new(&root);
        let pool = local_cons.memory_pool();
        Self {
            extractor: ShadowExtractor::new(),
            consolidator: Consolidator::new(),
            local_consolidator: local_cons,
            regulator: Some(EvolvingRegulator::new(pool, root)),
            state: EvolutionState::Idle,
        }
    }

    /// 提取用户纠偏 → 自动固化
    pub fn learn_from_correction(
        &mut self,
        error: &str,
        fix: &str,
        result: &str,
        scope: &str,
    ) -> ConsolidatedSkill {
        let delta = self.extractor.extract_user_correction(error, fix, result, scope);
        let skill = self.consolidator.consolidate(&delta);
        self.extractor.mark_consolidated(&delta.id);
        skill
    }

    /// 提取自愈经验 → 固化
    pub fn learn_from_healing(&mut self, error: &str, fix: &str, scope: &str) -> ConsolidatedSkill {
        let delta = self.extractor.extract_self_healing(error, fix, scope);
        let skill = self.consolidator.consolidate(&delta);
        self.extractor.mark_consolidated(&delta.id);
        skill
    }

    /// 提取回滚经验 → 固化
    pub fn learn_from_rewind(&mut self, hallucination: &str, point: &str, scope: &str) -> ConsolidatedSkill {
        let delta = self.extractor.extract_rewind(hallucination, point, scope);
        let skill = self.consolidator.consolidate(&delta);
        self.extractor.mark_consolidated(&delta.id);
        skill
    }

    /// 检索相似经验
    pub fn find_similar(&mut self, query: &str) -> Vec<ConsolidatedSkill> {
        self.consolidator.find_similar(query, 3)
    }

    /// 生成硬约束
    pub fn generate_hard_constraints(&mut self, query: &str) -> String {
        self.consolidator.generate_hard_constraints(query)
    }

    /// 2.0: 双向隔离验证提交经验
    pub async fn validate_and_commit(&self, delta: EvoDelta) -> Result<bool, String> {
        self.local_consolidator.validate_and_commit_experience(delta).await
    }

    /// 2.0: 零Token经验拦截 + 热编译契约
    pub async fn intercept_context(&mut self, hash: &str) -> std::io::Result<bool> {
        if let Some(ref mut reg) = self.regulator {
            reg.intercept_and_hot_compile_contract(hash).await
        } else {
            Ok(false)
        }
    }

    /// 2.0: 获取进化统计（含 2.0 字段）
    pub fn evolution_status(&self) -> serde_json::Value {
        let pool_size = self.local_consolidator.active_memory_pool
            .try_lock()
            .map(|p| p.len())
            .unwrap_or(0);
        let reg_stats = self.regulator.as_ref().map(|r| r.get_stats());
        serde_json::json!({
            "state": format!("{:?}", self.state),
            "memory_pool_size": pool_size,
            "total_interceptions": reg_stats.map(|s| s.total_interceptions).unwrap_or(0),
            "contracts_compiled": reg_stats.map(|s| s.contracts_compiled).unwrap_or(0),
            "total_tokens_saved": reg_stats.map(|s| s.tokens_saved).unwrap_or(0),
            "skills_consolidated": self.consolidator.stats().total_skills,
        })
    }

    /// 统计
    pub fn stats(&self) -> EvolutionStats {
        EvolutionStats {
            total_deltas: self.extractor.stats().total_deltas,
            consolidated_skills: self.consolidator.stats().total_skills,
            tokens_saved_by_reuse: self.consolidator.stats().estimated_tokens_saved,
            state: format!("{:?}", self.state),
            extractor: self.extractor.stats(),
            consolidator: self.consolidator.stats(),
        }
    }

    /// 持久化进化引擎学习成果（固化技能 + 记忆池 + 嵌入 + 调节器统计）
    pub fn save_state(&self, dir: &std::path::Path) -> Result<String, String> {
        let _ = self.consolidator.save_state(dir);
        let _ = self.local_consolidator.save_state(dir);
        if let Some(ref reg) = self.regulator {
            let path = dir.join("evolution_regulator.json");
            let state = serde_json::json!({
                "total_interceptions": reg.stats.total_interceptions,
                "contracts_compiled": reg.stats.contracts_compiled,
                "tokens_saved": reg.stats.tokens_saved,
            });
            let _ = std::fs::write(&path, serde_json::to_string_pretty(&state).map_err(|e| e.to_string())?);
        }
        Ok("EvolutionEngine state saved".into())
    }

    /// 从磁盘恢复进化引擎学习成果
    pub fn load_state(&mut self, dir: &std::path::Path) -> Result<String, String> {
        let _ = self.consolidator.load_state(dir);
        let _ = self.local_consolidator.load_state(dir);
        if let Some(ref mut reg) = self.regulator {
            let path = dir.join("evolution_regulator.json");
            if path.exists() {
                let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                let state: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
                if let Some(v) = state.get("total_interceptions").and_then(|v| v.as_u64()) { reg.stats.total_interceptions = v; }
                if let Some(v) = state.get("contracts_compiled").and_then(|v| v.as_u64()) { reg.stats.contracts_compiled = v; }
                if let Some(v) = state.get("tokens_saved").and_then(|v| v.as_u64()) { reg.stats.tokens_saved = v; }
            }
        }
        Ok("EvolutionEngine state loaded".into())
    }
}

impl Default for EvolutionEngine {
    fn default() -> Self { Self::new(Some(std::path::PathBuf::from("."))) }
}

// ─── Tauri Commands ──────────────────────────────────────────────

#[tauri::command]
pub async fn evo_validate_experience(
    state: tauri::State<'_, crate::state::AppState>,
    experience_id: String,
    context_hash: String,
    failed_action: String,
    correct_action: String,
    token_saved: u32,
) -> Result<bool, String> {
    let delta = EvoDelta {
        experience_id,
        context_trigger_hash: context_hash,
        failed_llm_action: failed_action,
        correct_human_action: correct_action,
        token_sunk_cost_saved: token_saved,
        accuracy_weight: 1.0,
    };
    state.evolution.lock().await.validate_and_commit(delta).await
}

#[tauri::command]
pub async fn evo_intercept_context(
    state: tauri::State<'_, crate::state::AppState>,
    context_hash: String,
) -> Result<bool, String> {
    let mut engine = state.evolution.lock().await;
    engine.intercept_context(&context_hash).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_evolution_stats(state: tauri::State<crate::state::AppState>) -> serde_json::Value {
    let engine = state.evolution.try_lock();
    let base = engine.as_ref().map(|e| e.evolution_status()).unwrap_or(serde_json::json!({
        "state": "locked",
        "memory_pool_size": 0,
        "total_interceptions": 0,
        "contracts_compiled": 0,
        "total_tokens_saved": 0,
        "skills_consolidated": 0,
    }));
    let skill_engine = state.skill_engine.lock().unwrap();
    let mut result = base;
    result["total_skills"] = serde_json::json!(skill_engine.list_all().len());
    result["active_skills"] = serde_json::json!(skill_engine.active_skills().len());
    result
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::consolidator::EvoDelta;
    use super::extractor::{DeltaExperience, DeltaTrigger};

    #[test]
    fn test_evolution_engine_persistence_roundtrip() {
        let dir = std::env::temp_dir().join("chronos_evo_engine_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut engine = EvolutionEngine::new(Some(dir.clone()));
        // 固化一条技能
        let delta = DeltaExperience {
            id: "delta-1".into(),
            timestamp: "2026-01-01T00:00:00Z".into(),
            trigger: DeltaTrigger::UserCorrection,
            original_error: "unwrap()".into(),
            correction: "use ? operator".into(),
            correct_result: "compiled".into(),
            scope: "src/main".into(),
            embedding: None,
        };
        engine.consolidator.consolidate(&delta);
        // 写入一条记忆
        {
            let mut pool = engine.local_consolidator.active_memory_pool.blocking_lock();
            pool.insert("hash-1".into(), EvoDelta {
                experience_id: "exp-1".into(),
                context_trigger_hash: "hash-1".into(),
                failed_llm_action: "unwrap()".into(),
                correct_human_action: "? operator".into(),
                token_sunk_cost_saved: 50,
                accuracy_weight: 0.7,
            });
        }

        engine.save_state(&dir).unwrap();

        let mut engine2 = EvolutionEngine::new(Some(dir.clone()));
        engine2.load_state(&dir).unwrap();

        assert_eq!(engine2.consolidator.db.skills.len(), 1);
        assert_eq!(engine2.consolidator.db.skills[0].id, "skill-0001");
        let pool = engine2.local_consolidator.active_memory_pool.blocking_lock();
        assert_eq!(pool.len(), 1);
        assert!(pool.contains_key("hash-1"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
