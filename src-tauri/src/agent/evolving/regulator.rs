// 零 Token 经验重用与硬契约热编译器 (Evolving Regulator)
//
// 核心功能：
// - 在每次 Quest 任务分发前，优先在本地检索历史踩坑记忆
// - 命中后免调用远程大模型，端侧本地热编译为硬性物理规则
// - 强制合并写入 CLAUDE.md 缓存头部，触发 DeepSeek V4 Context Caching
// - 削减 100% 的额外规划 Token

use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use crate::agent::evolving::consolidator::EvoDelta;

// ─── 进化调节器 ──────────────────────────────────────────────────

pub struct EvolvingRegulator {
    /// 共享的活跃记忆池（与 Consolidator 共享）
    pub active_memory_pool: Arc<Mutex<HashMap<String, EvoDelta>>>,
    /// CLAUDE.md 路径
    pub claude_md_path: PathBuf,
    /// 热编译统计
    pub stats: RegulatorStats,
}

#[derive(Debug, Clone, Default)]
pub struct RegulatorStats {
    /// 累计拦截次数
    pub total_interceptions: u64,
    /// 热编译次数
    pub contracts_compiled: u64,
    /// 免调用的 Token 估算
    pub tokens_saved: u64,
}

impl EvolvingRegulator {
    pub fn new(
        pool: Arc<Mutex<HashMap<String, EvoDelta>>>,
        sandbox_root: PathBuf,
    ) -> Self {
        Self {
            active_memory_pool: pool,
            claude_md_path: sandbox_root.join("CLAUDE.md"),
            stats: RegulatorStats::default(),
        }
    }

    /// 核心功能：零 Token 经验重用与 CLAUDE.md 契约热编译引擎
    ///
    /// 在呼叫云端前执行拦截，把 AI 曾经犯过的错、踩过的坑，
    /// 直接编译成硬性指令注入 Context 头部。
    ///
    /// 返回 true 表示命中本地经验并完成热编译（无需调用远程模型）
    pub async fn intercept_and_hot_compile_contract(
        &mut self,
        current_context_hash: &str,
    ) -> std::io::Result<bool> {
        let pool = self.active_memory_pool.lock().await;
        self.stats.total_interceptions += 1;

        // 1. 检索端侧记忆库是否命中该场景的"历史挨打记录"
        if let Some(experience) = pool.get(current_context_hash) {
            tracing::info!(
                "[CHRONOS REGULATOR] Local Evolution Hit! Context hash: {}",
                &current_context_hash[..16.min(current_context_hash.len())]
            );

            // 2. 提取经验：全自动生成专属的、大模型不可忤逆的反思契约规则
            let anti_hallucination_rule = format!(
                "\n<!-- CHRONOS-EVOLVED-REGULATION [{}] -->\n\
                 ## ⚠️ Anti-Hallucination Hard Constraint (Zero-Token Local)\n\
                 - **History**: Previous attempt failed with incorrect action\n\
                 - **Forbidden Action**: {}\n\
                 - **Mandatory Correction**: {}\n\
                 - **Token Saved**: {} tokens\n\
                 <!-- /CHRONOS-EVOLVED-REGULATION -->\n",
                experience.experience_id,
                experience.failed_llm_action.trim(),
                experience.correct_human_action.trim(),
                experience.token_sunk_cost_saved,
            );

            // 3. 动态热编译：将硬红线规约直接追加并锁死在 CLAUDE.md 根契约头部
            let mut contract_content =
                std::fs::read_to_string(&self.claude_md_path).unwrap_or_default();

            if !contract_content.contains(&experience.experience_id) {
                contract_content.push_str(&format!(
                    "\n# MEMORY_ID: {}\n",
                    experience.experience_id
                ));
                contract_content.push_str(&anti_hallucination_rule);
                std::fs::write(&self.claude_md_path, &contract_content)?;

                self.stats.contracts_compiled += 1;
                self.stats.tokens_saved += experience.token_sunk_cost_saved as u64;

                tracing::info!(
                    "[CHRONOS REGULATOR] Dynamic Contract Hot-Compiled. \
                     Context Caching maximized via 1-fold discount. \
                     Contracts: {}, Tokens saved: {}",
                    self.stats.contracts_compiled,
                    self.stats.tokens_saved,
                );
                return Ok(true);
            }

            tracing::debug!(
                "[CHRONOS REGULATOR] Experience {} already compiled — skipping",
                experience.experience_id
            );
        }

        Ok(false)
    }

    /// 获取当前 CLAUDE.md 中的契约内容
    pub fn read_active_contracts(&self) -> String {
        std::fs::read_to_string(&self.claude_md_path).unwrap_or_default()
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> &RegulatorStats {
        &self.stats
    }
}
