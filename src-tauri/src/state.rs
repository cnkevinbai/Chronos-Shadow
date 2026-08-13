// Chronos-Shadow 全局应用状态 (AppState)
// 所有 Tauri command 通过 tauri::State<AppState> 访问共享引擎

use crate::agent::api_client::ApiClient;
use crate::agent::approval_gate::ApprovalGate;
use crate::agent::billing_engine::ChronosParallelBillingEngine;
use crate::agent::buddy_scan::BuddyScanner;
use crate::agent::collaboration_engine::CollaborationEngine;
use crate::agent::context_cache::ContextCacheEngine;
use crate::agent::context_glue::ContextGlue;
use crate::agent::data_flywheel::DataFlywheel;
use crate::agent::evolution_bus::EvolutionBus;
use crate::agent::evolving::EvolutionEngine;
use crate::agent::evolving::agent_quality::AgentQualityEngine;
use crate::agent::evolving::embedding::EmbeddingEngine;
use crate::agent::local_analytics::LocalAnalytics;
use crate::agent::mcp_client::McpClient;
use crate::agent::orchestrator::Orchestrator;
use crate::agent::predictive_analytics::PredictiveAnalyticsEngine;
use crate::agent::redline::RedlineGuard;
use crate::agent::remote_cluster::RemoteClusterManager;
use crate::agent::remote_proxy::RemoteProxyTunnel;
use crate::agent::resilience::{SystemHealth, CircuitBreaker};
#[allow(deprecated)]
use crate::agent::router::Router;
use crate::agent::router::HybridAgentRouter;
use crate::agent::sandbox::{Sandbox, ChronosVirtualFileSystem};
use crate::agent::security_boundary::SecurityBoundary;
use crate::agent::shadow::ShadowEngine;
use crate::agent::skill_engine::SkillEngine;
use crate::agent::state_manager::StateManager;
use crate::agent::subagents::SubagentPool;
use crate::agent::task_intelligence::TaskIntelligenceEngine;
use crate::agent::user_profile::UserProfile;
use crate::agent::web_intelligence::WebIntelligence;
use crate::agent::workbuddy_engine::WorkBuddyEngine;
use crate::agent::worktree::WorktreeManager;
use crate::vision::VisionEngine;
use std::path::PathBuf;
use std::sync::Mutex;
use tokio::sync::Mutex as TokioMutex;

/// 全局应用状态：每个引擎一个 Mutex/TokioMutex
pub struct AppState {
    pub redline: Mutex<RedlineGuard>,
    pub orchestrator: Mutex<Orchestrator>,
    pub api_client: TokioMutex<ApiClient>,
    pub router: Mutex<Router>,
    #[allow(dead_code)]
    pub sandbox: Mutex<Sandbox>,
    #[allow(dead_code)]
    pub skill_engine: Mutex<SkillEngine>,
    #[allow(dead_code)]
    pub mcp_client: TokioMutex<McpClient>,
    #[allow(dead_code)]
    pub subagent_pool: Mutex<SubagentPool>,
    #[allow(dead_code)]
    pub vision: Mutex<VisionEngine>,
    pub shadow: Mutex<ShadowEngine>,
    pub buddy_scan: Mutex<BuddyScanner>,
    pub context_glue: Mutex<ContextGlue>,
    pub evolution: TokioMutex<EvolutionEngine>,
    pub remote_proxy: TokioMutex<Option<RemoteProxyTunnel>>,
    pub cluster: TokioMutex<RemoteClusterManager>,
    pub cvfs: tokio::sync::Mutex<ChronosVirtualFileSystem>,
    pub billing_engine: ChronosParallelBillingEngine,
    pub worktree: Mutex<WorktreeManager>,
    pub approval_gate: Mutex<ApprovalGate>,
    pub agent_quality: Mutex<AgentQualityEngine>,
    pub embedding: Mutex<EmbeddingEngine>,
    pub hybrid_router: HybridAgentRouter,
    pub analytics: Mutex<LocalAnalytics>,
    pub state_mgr: Mutex<StateManager>,
    pub workbuddy: Mutex<WorkBuddyEngine>,
    pub system_health: SystemHealth,
    pub api_circuit_breaker: CircuitBreaker,
    pub user_profile: Mutex<UserProfile>,
    pub security_boundary: Mutex<SecurityBoundary>,
    pub web_intelligence: TokioMutex<WebIntelligence>,
    pub collaboration: TokioMutex<CollaborationEngine>,
    pub task_intelligence: Mutex<TaskIntelligenceEngine>,
    pub predictive: Mutex<PredictiveAnalyticsEngine>,
    pub evolution_bus: Mutex<EvolutionBus>,
    pub flywheel: Mutex<DataFlywheel>,
    pub context_cache: Mutex<ContextCacheEngine>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            redline: Mutex::new(RedlineGuard::new(PathBuf::from("."))),
            orchestrator: Mutex::new(Orchestrator::new()),
            api_client: TokioMutex::new(ApiClient::new().expect("Failed to create API client")),
            router: Mutex::new(Router::new()),
            sandbox: Mutex::new(Sandbox::new(PathBuf::from("."))),
            skill_engine: Mutex::new(SkillEngine::new()),
            mcp_client: TokioMutex::new(McpClient::new()),
            subagent_pool: Mutex::new(SubagentPool::new()),
            vision: Mutex::new(VisionEngine::new()),
            shadow: Mutex::new(ShadowEngine::new()),
            buddy_scan: Mutex::new(BuddyScanner::new()),
            context_glue: Mutex::new(ContextGlue::new()),
            evolution: TokioMutex::new(EvolutionEngine::new(Some(PathBuf::from(".")))),
            remote_proxy: TokioMutex::new(None),
            cluster: TokioMutex::new(RemoteClusterManager::new()),
            cvfs: tokio::sync::Mutex::new(ChronosVirtualFileSystem::new()),
            billing_engine: ChronosParallelBillingEngine::new(),
            worktree: Mutex::new(WorktreeManager::new(PathBuf::from("."))),
            approval_gate: Mutex::new(ApprovalGate::new()),
            agent_quality: Mutex::new(AgentQualityEngine::new()),
            embedding: Mutex::new(EmbeddingEngine::new()),
            hybrid_router: HybridAgentRouter::new(),
            analytics: Mutex::new(LocalAnalytics::new()),
            state_mgr: Mutex::new(StateManager::new()),
            workbuddy: Mutex::new(WorkBuddyEngine::new()),
            system_health: SystemHealth::new(),
            api_circuit_breaker: CircuitBreaker::new("api"),
            user_profile: Mutex::new(UserProfile::default()),
            security_boundary: Mutex::new(SecurityBoundary::new()),
            web_intelligence: TokioMutex::new(WebIntelligence::new()),
            collaboration: TokioMutex::new(CollaborationEngine::new()),
            task_intelligence: Mutex::new(TaskIntelligenceEngine::new()),
            predictive: Mutex::new(PredictiveAnalyticsEngine::new()),
            evolution_bus: Mutex::new(EvolutionBus::new()),
            flywheel: Mutex::new(DataFlywheel::new()),
            context_cache: Mutex::new(ContextCacheEngine::new()),
        }
    }
}
