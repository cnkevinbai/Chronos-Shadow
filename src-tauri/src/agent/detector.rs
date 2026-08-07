// detector.rs — 零 Token 本地技能检测引擎 + 集群自适应分配引擎
//
// 核心架构：
// 1. SkillAndMcpDetector — 在 Agent 呼叫云端模型前，端侧 0ms 拦截命中本地 Skill
// 2. ClusterWorkAllocator — 基于黑板模式的分布式状态飞轮令牌分配
//
// 降本闭环：本地确定性 Skill 命中 → 云端 Function Calling 跳过 → 资费归零

use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::json;
use crate::agent::orchestrator::{Blackboard, SdlcState, SdlcEvent};
use crate::agent::redline::AgentAction;

// ─── 注册技能特征 ──────────────────────────────────────────────────

/// 注册的本地专属技能或 MCP 工具的静态特征点描述
pub struct RegisteredSkillFeature {
    pub skill_name: String,
    /// 用于端侧 0-Token 快速比对识别的意图关键字
    pub intent_keyword: String,
    /// 对应的标准 Action Schema 规范
    pub bound_action_schema: String,
}

// ─── 检测统计 ──────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DetectorStats {
    /// 累计拦截次数
    pub total_interceptions: u64,
    /// 累计命中次数
    pub total_hits: u64,
    /// 命中率
    pub hit_rate: f32,
    /// 估算节省 Token 数
    pub tokens_saved: u64,
    /// 估算节省费用 (¥)
    pub estimated_cost_saved: f64,
}

// ─── 零 Token 技能检测引擎 ─────────────────────────────────────────

pub struct SkillAndMcpDetector {
    pub skill_registry: Arc<RwLock<Vec<RegisteredSkillFeature>>>,
    pub stats: Arc<RwLock<DetectorStats>>,
}

impl SkillAndMcpDetector {
    pub fn new() -> Self {
        let mut registry = Vec::new();

        // 预设工业级专属 Skill 特征集
        registry.push(RegisteredSkillFeature {
            skill_name: "context_glue_excelfiller".to_string(),
            intent_keyword: "读取Excel录入网页表单".to_string(),
            bound_action_schema: "execute_skill".to_string(),
        });
        registry.push(RegisteredSkillFeature {
            skill_name: "vlm_diff_inspector".to_string(),
            intent_keyword: "还原度差分走查".to_string(),
            bound_action_schema: "execute_skill".to_string(),
        });
        registry.push(RegisteredSkillFeature {
            skill_name: "win32_handle_texthijacker".to_string(),
            intent_keyword: "Win32句柄文本抓取".to_string(),
            bound_action_schema: "execute_skill".to_string(),
        });
        registry.push(RegisteredSkillFeature {
            skill_name: "chronos_omni_rewind_trigger".to_string(),
            intent_keyword: "时空逆转回滚".to_string(),
            bound_action_schema: "execute_skill".to_string(),
        });
        registry.push(RegisteredSkillFeature {
            skill_name: "omnidesign_matrix".to_string(),
            intent_keyword: "跨端视觉设计".to_string(),
            bound_action_schema: "execute_skill".to_string(),
        });

        let stats = DetectorStats {
            total_interceptions: 0,
            total_hits: 0,
            hit_rate: 0.0,
            tokens_saved: 0,
            estimated_cost_saved: 0.0,
        };

        Self {
            skill_registry: Arc::new(RwLock::new(registry)),
            stats: Arc::new(RwLock::new(stats)),
        }
    }

    /// 核心：零 Token 本地技能需求自动检测调用引擎
    ///
    /// 在 Agent 呼叫云端模型进行推理规划前，在端侧 0ms 强行拦截，
    /// 精准命中本地确定性 Skill。
    ///
    /// 如果命中 → 返回 AgentAction，跳过云端调用
    /// 如果未命中 → 返回 None，放行给云端混合路由
    pub async fn detect_and_intercept_local_skill(
        &self,
        current_requirement: &str,
    ) -> Option<AgentAction> {
        let registry = self.skill_registry.read().await;
        tracing::info!(
            "[DETECTOR SHIELD] Scanning context for 0-Token local intercept..."
        );

        for skill in registry.iter() {
            if current_requirement.contains(&skill.intent_keyword) {
                let mut s = self.stats.write().await;
                s.total_hits += 1;
                s.total_interceptions += 1;
                s.tokens_saved += 2500; // 估算每次拦截省 2500 tokens
                s.estimated_cost_saved += 2500.0 * 0.0001; // ¥0.0001/token

                if s.total_interceptions > 0 {
                    s.hit_rate = s.total_hits as f32 / s.total_interceptions as f32;
                }

                tracing::info!(
                    "[DETECTOR HIT] 🎯 Matched! Intent → local skill: [{}]. Saved ~¥0.25.",
                    skill.skill_name
                );

                return Some(AgentAction::ExecuteSkill {
                    name: skill.skill_name.clone(),
                    args: json!({
                        "auto_triggered": true,
                        "intercept_timestamp": chrono::Utc::now().to_rfc3339(),
                        "detector_saved_tokens": 2500,
                    }),
                });
            }
        }

        // 未命中时的统计更新
        {
            let mut s = self.stats.write().await;
            s.total_interceptions += 1;
            if s.total_interceptions > 0 {
                s.hit_rate = s.total_hits as f32 / s.total_interceptions as f32;
            }
        }

        None
    }

    /// 注册新技能特征
    pub async fn register_skill_feature(&self, skill: RegisteredSkillFeature) {
        let mut registry = self.skill_registry.write().await;
        registry.push(skill);
    }

    /// 获取检测统计
    pub async fn get_stats(&self) -> DetectorStats {
        self.stats.read().await.clone()
    }
}

// ─── 集群自适应工作分配引擎 ────────────────────────────────────────

pub struct ClusterWorkAllocator {
    pub blackboard: Arc<RwLock<Blackboard>>,
    pub detector: Arc<SkillAndMcpDetector>,
}

impl ClusterWorkAllocator {
    pub fn new(blackboard: Arc<RwLock<Blackboard>>) -> Self {
        Self {
            blackboard,
            detector: Arc::new(SkillAndMcpDetector::new()),
        }
    }

    /// 状态飞轮总控：根据黑板状态与物理主机标签，全自动安全派发工作执行令牌
    #[allow(dead_code)]
    pub async fn dispatch_allocated_work(
        &self,
        current_event: &SdlcEvent,
    ) -> Result<(), String> {
        let mut bb = self.blackboard.write().await;
        tracing::info!(
            "[ALLOCATOR Central] Processing allocation for event: {:?}",
            current_event
        );

        match current_event {
            SdlcEvent::RequirementSubmitted(req) => {
                // 1. 自适应判定：优先调用检测引擎在端侧拦截确定性 Skill
                if let Some(intercepted) = self
                    .detector
                    .detect_and_intercept_local_skill(req)
                    .await
                {
                    tracing::info!(
                        "[ALLOCATOR] Intercepted! Action: {:?}. Bypassing cloud pipeline.",
                        intercepted
                    );
                    bb.current_state = SdlcState::Verifying;
                    return Ok(());
                }

                // 2. 状态飞轮流转：独占性分配令牌给 PM Agent
                bb.current_state = SdlcState::Designing;
                tracing::info!(
                    "[ALLOCATOR] Token-Ring locked → [Cluster PM Agent]"
                );
            }
            SdlcEvent::DesignFinalized => {
                bb.current_state = SdlcState::Planning;
                tracing::info!(
                    "[ALLOCATOR] Token-Ring locked → [Planner Agent]"
                );
            }
            SdlcEvent::PlanGenerated => {
                bb.current_state = SdlcState::Coding;
                tracing::info!(
                    "[ALLOCATOR] Token-Ring locked → [Coder Subagents] (parallel sandbox)"
                );
            }
            SdlcEvent::CodingCompleted => {
                bb.current_state = SdlcState::Auditing;
                tracing::info!(
                    "[ALLOCATOR] Token-Ring locked → [Auditor Agent]"
                );
            }
            SdlcEvent::AuditPassed => {
                bb.current_state = SdlcState::Verifying;
                tracing::info!(
                    "[ALLOCATOR] Token-Ring locked → [Verifier Agent]"
                );
            }
            SdlcEvent::VerificationDone => {
                bb.current_state = SdlcState::Completed;
                tracing::info!("[ALLOCATOR] Pipeline completed.");
            }
        }

        Ok(())
    }

    /// 获取黑板状态快照
    #[allow(dead_code)]
    pub async fn get_blackboard_state(&self) -> SdlcState {
        self.blackboard.read().await.current_state.clone()
    }
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detector_hit() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let detector = SkillAndMcpDetector::new();
            let result = detector
                .detect_and_intercept_local_skill("请帮我读取Excel录入网页表单的数据")
                .await;
            assert!(result.is_some());
            if let Some(AgentAction::ExecuteSkill { name, .. }) = result {
                assert_eq!(name, "context_glue_excelfiller");
            }
        });
    }

    #[test]
    fn test_detector_miss() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let detector = SkillAndMcpDetector::new();
            let result = detector
                .detect_and_intercept_local_skill("写一个Python脚本")
                .await;
            assert!(result.is_none());
        });
    }

    #[test]
    fn test_detector_stats() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let detector = SkillAndMcpDetector::new();
            detector
                .detect_and_intercept_local_skill("读取Excel录入网页表单")
                .await;
            detector
                .detect_and_intercept_local_skill("Python脚本")
                .await;
            let stats = detector.get_stats().await;
            assert_eq!(stats.total_interceptions, 2);
            assert_eq!(stats.total_hits, 1);
            assert!(stats.estimated_cost_saved > 0.0);
        });
    }

    #[test]
    fn test_allocator_dispatch() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let bb = Arc::new(RwLock::new(Blackboard::new()));
            let allocator = ClusterWorkAllocator::new(bb.clone());
            allocator
                .dispatch_allocated_work(&SdlcEvent::RequirementSubmitted(
                    "普通任务".into(),
                ))
                .await
                .unwrap();
            assert_eq!(
                bb.read().await.current_state,
                SdlcState::Designing
            );
        });
    }

    #[test]
    fn test_allocator_intercept_bypass_pipeline() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let bb = Arc::new(RwLock::new(Blackboard::new()));
            let allocator = ClusterWorkAllocator::new(bb.clone());
            allocator
                .dispatch_allocated_work(&SdlcEvent::RequirementSubmitted(
                    "读取Excel录入网页表单的数据".into(),
                ))
                .await
                .unwrap();
            // 被检测引擎拦截 → 直接跳到 Verifying 状态
            assert_eq!(
                bb.read().await.current_state,
                SdlcState::Verifying
            );
        });
    }
}
