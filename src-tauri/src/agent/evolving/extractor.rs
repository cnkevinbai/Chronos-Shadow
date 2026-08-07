// 影子记忆提取器 (Shadow Memory Extractor)
// 白皮书 §3.1：捕获执行轨迹、用户微调、CI/CD 修复日志
//
// 当 Quest 执行完毕或遭遇以下触发条件，影子提取器自动激活：
// 1. 用户断点拦截（前端的 Timeline 手动修改）
// 2. Verifier 自动纠错成功（Self-Healing 闭环）
// 3. 时空逆转触发（Omni-Rewind 撤销幻觉动作）
//
// 提取 "原始错误指令 → 修复动作 → 正确执行结果" 的正反向经验对账单

use serde::{Deserialize, Serialize};

// ─── 类型定义 ──────────────────────────────────────────────────────

/// Delta 经验对账单 — 一次自我学习的原子单元
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaExperience {
    /// 唯一 ID
    pub id: String,
    /// 时间戳
    pub timestamp: String,
    /// 触发类型
    pub trigger: DeltaTrigger,
    /// 原始错误指令（大模型幻觉输出）
    pub original_error: String,
    /// 修复/纠偏动作
    pub correction: String,
    /// 正确执行结果
    pub correct_result: String,
    /// 影响范围（文件路径/操作类型）
    pub scope: String,
    /// 向量嵌入（由 Consolidator 填充）
    #[serde(skip)]
    pub embedding: Option<Vec<f32>>,
}

/// 触发类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeltaTrigger {
    /// 用户手动纠偏
    UserCorrection,
    /// Verifier 自愈成功
    SelfHealing,
    /// 时空逆转触发
    OmniRewind,
    /// 红线熔断恢复
    RedlineFuse,
}

/// 影子提取器
pub struct ShadowExtractor {
    /// 已提取的经验对
    pub deltas: Vec<DeltaExperience>,
    /// 计数器
    counter: u64,
}

impl ShadowExtractor {
    pub fn new() -> Self {
        Self { deltas: Vec::new(), counter: 0 }
    }

    /// 从用户纠偏中提取经验
    pub fn extract_user_correction(
        &mut self,
        original_error: &str,
        correction: &str,
        correct_result: &str,
        scope: &str,
    ) -> DeltaExperience {
        self.counter += 1;
        let delta = DeltaExperience {
            id: format!("delta-{:04}", self.counter),
            timestamp: chrono::Utc::now().to_rfc3339(),
            trigger: DeltaTrigger::UserCorrection,
            original_error: strip_verbose(original_error),
            correction: strip_verbose(correction),
            correct_result: strip_verbose(correct_result),
            scope: scope.into(),
            embedding: None,
        };
        self.deltas.push(delta.clone());
        tracing::info!(
            "[EVOLVING] Extracted user correction delta #{}: {}",
            self.counter, scope
        );
        delta
    }

    /// 从 Verifier 自愈中提取经验
    pub fn extract_self_healing(
        &mut self,
        compile_error: &str,
        fix_action: &str,
        scope: &str,
    ) -> DeltaExperience {
        self.counter += 1;
        let delta = DeltaExperience {
            id: format!("delta-{:04}", self.counter),
            timestamp: chrono::Utc::now().to_rfc3339(),
            trigger: DeltaTrigger::SelfHealing,
            original_error: strip_verbose(compile_error),
            correction: strip_verbose(fix_action),
            correct_result: "Compilation successful after fix".into(),
            scope: scope.into(),
            embedding: None,
        };
        self.deltas.push(delta.clone());
        tracing::info!(
            "[EVOLVING] Extracted self-healing delta #{}: {}",
            self.counter, scope
        );
        delta
    }

    /// 从时空逆转中提取经验
    pub fn extract_rewind(
        &mut self,
        hallucination: &str,
        rewind_point: &str,
        scope: &str,
    ) -> DeltaExperience {
        self.counter += 1;
        let delta = DeltaExperience {
            id: format!("delta-{:04}", self.counter),
            timestamp: chrono::Utc::now().to_rfc3339(),
            trigger: DeltaTrigger::OmniRewind,
            original_error: strip_verbose(hallucination),
            correction: format!("Rewound to checkpoint: {}", rewind_point),
            correct_result: "Environment restored to pre-hallucination state".into(),
            scope: scope.into(),
            embedding: None,
        };
        self.deltas.push(delta.clone());
        tracing::info!(
            "[EVOLVING] Extracted rewind delta #{}: {}",
            self.counter, scope
        );
        delta
    }

    /// 获取未固化的新增经验（供 Consolidator 消费）
    pub fn pending_deltas(&self) -> Vec<&DeltaExperience> {
        self.deltas.iter().filter(|d| d.embedding.is_none()).collect()
    }

    /// 标记经验已固化
    pub fn mark_consolidated(&mut self, delta_id: &str) {
        if let Some(d) = self.deltas.iter_mut().find(|d| d.id == delta_id) {
            d.embedding = Some(vec![]); // placeholder — real vector assigned by Consolidator
        }
    }

    /// 统计
    pub fn stats(&self) -> ExtractorStats {
        ExtractorStats {
            total_deltas: self.deltas.len() as u64,
            pending: self.pending_deltas().len() as u64,
            user_corrections: self.deltas.iter().filter(|d| d.trigger == DeltaTrigger::UserCorrection).count() as u64,
            self_healings: self.deltas.iter().filter(|d| d.trigger == DeltaTrigger::SelfHealing).count() as u64,
            omni_rewinds: self.deltas.iter().filter(|d| d.trigger == DeltaTrigger::OmniRewind).count() as u64,
            redline_fuses: self.deltas.iter().filter(|d| d.trigger == DeltaTrigger::RedlineFuse).count() as u64,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractorStats {
    pub total_deltas: u64,
    pub pending: u64,
    pub user_corrections: u64,
    pub self_healings: u64,
    pub omni_rewinds: u64,
    pub redline_fuses: u64,
}

/// 剥离冗余会话废话，压缩为精炼经验
fn strip_verbose(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.chars().count() > 500 {
        let preview: String = trimmed.chars().take(497).collect();
        format!("{}...", preview)
    } else {
        trimmed.into()
    }
}
