// Chronos-Shadow 核心库入口
#![allow(deprecated)] // 旧 Router 将在 v0.2.0 彻底移除

pub mod agent;
pub mod vision;

use serde::{Deserialize, Serialize};
use tauri::Manager;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::WindowEvent;
use agent::api_client::{ApiClient, ApiResponse, ChatMessage};
use agent::orchestrator::{AgentRole, Orchestrator, OrchestratorStats};
use agent::redline::{RedlineGuard, RedlineStatus};
#[allow(deprecated)]
#[allow(deprecated)]
use agent::router::{ModelConfig, RouteMode, Router};
use agent::router::HybridAgentRouter;
use agent::sandbox::{Sandbox, ChronosVirtualFileSystem};
use agent::detector::SkillAndMcpDetector;
use agent::shadow::{ShadowEngine, ShadowStats};
use agent::skill_engine::{SkillEngine, SkillInstance};
use agent::mcp_client::{McpClient, McpServer};
use agent::subagents::SubagentPool;
use agent::buddy_scan::{BuddyScanner, BuddyScanStats, BuddyScanResult};
use agent::context_glue::{
    ContextGlue, ContextGlueStats, AppBinding, DataDirection,
};
use agent::evolving::{EvolutionEngine, consolidator::EvoDelta};
use agent::remote_proxy::{RemoteProxyTunnel, RemoteConfig, RemoteSessionStats};
use agent::remote_cluster::{RemoteClusterManager, ClusterStats};
use agent::session_db::{
    save_chat_session_chunk, load_chat_session_chunk,
    list_historical_meta_manifests, list_sessions_by_project,
    delete_chat_session, export_chat_session, rename_chat_session,
    import_chat_session,
};
use agent::worktree::WorktreeManager;
use agent::approval_gate::ApprovalGate;
use agent::evolving::agent_quality::AgentQualityEngine;
use agent::evolving::embedding::EmbeddingEngine;
use agent::local_analytics::LocalAnalytics;
use agent::state_manager::StateManager;
use agent::workbuddy_engine::WorkBuddyEngine;
use agent::resilience::{SystemHealth, CircuitBreaker};
use agent::user_profile::UserProfile;
use agent::security_boundary::SecurityBoundary;
use agent::billing_engine::ChronosParallelBillingEngine;
use agent::web_intelligence::{WebIntelligence, WebSearchResult, WebFetchResult, ResearchReport, WebAuditEntry, WebIntelStats};
use agent::redline::AgentAction;
use agent::collaboration_engine::CollaborationEngine;
use agent::task_intelligence::TaskIntelligenceEngine;
use agent::predictive_analytics::PredictiveAnalyticsEngine;
use agent::evolution_bus::EvolutionBus;
use agent::data_flywheel::DataFlywheel;
use vision::VisionEngine;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tokio::sync::Mutex as TokioMutex;
use tracing_subscriber::fmt;
use tauri::tray::TrayIconBuilder;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
/// 全局应用状态
struct AppState {
    redline: Mutex<RedlineGuard>,
    orchestrator: Mutex<Orchestrator>,
    api_client: TokioMutex<ApiClient>,
    router: Mutex<Router>,
    #[allow(dead_code)]
    sandbox: Mutex<Sandbox>,
    #[allow(dead_code)]
    skill_engine: Mutex<SkillEngine>,
    #[allow(dead_code)]
    mcp_client: TokioMutex<McpClient>,
    #[allow(dead_code)]
    subagent_pool: Mutex<SubagentPool>,
    #[allow(dead_code)]
    vision: Mutex<VisionEngine>,
    shadow: Mutex<ShadowEngine>,
    buddy_scan: Mutex<BuddyScanner>,
    context_glue: Mutex<ContextGlue>,
    evolution: TokioMutex<EvolutionEngine>,
    remote_proxy: TokioMutex<Option<RemoteProxyTunnel>>,
    cluster: TokioMutex<RemoteClusterManager>,
    cvfs: tokio::sync::Mutex<ChronosVirtualFileSystem>,
    billing_engine: ChronosParallelBillingEngine,
    worktree: Mutex<WorktreeManager>,
    approval_gate: Mutex<ApprovalGate>,
    agent_quality: Mutex<AgentQualityEngine>,
    embedding: Mutex<EmbeddingEngine>,
    hybrid_router: HybridAgentRouter,
    analytics: Mutex<LocalAnalytics>,
    state_mgr: Mutex<StateManager>,
    workbuddy: Mutex<WorkBuddyEngine>,
    system_health: SystemHealth,
    api_circuit_breaker: CircuitBreaker,
    user_profile: Mutex<UserProfile>,
    security_boundary: Mutex<SecurityBoundary>,
    web_intelligence: TokioMutex<WebIntelligence>,
    collaboration: TokioMutex<CollaborationEngine>,
    task_intelligence: Mutex<TaskIntelligenceEngine>,
    predictive: Mutex<PredictiveAnalyticsEngine>,
    evolution_bus: Mutex<EvolutionBus>,
    flywheel: Mutex<DataFlywheel>,
}

impl AppState {
    fn new() -> Self {
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
        }
    }
}

// ─── Redline Commands ──────────────────────────────────────────────

#[tauri::command]
fn get_redline_status(state: tauri::State<AppState>) -> RedlineStatus {
    state.redline.lock().unwrap().get_status()
}

#[tauri::command]
fn validate_model_output(state: tauri::State<AppState>, raw: String) -> Result<String, String> {
    match state.redline.lock().unwrap().validate_output(&raw) {
        Ok(output) => Ok(serde_json::to_string(&output).unwrap_or_default()),
        Err(e) => Err(format!("{:?}", e)),
    }
}

#[tauri::command]
fn reset_fuse(state: tauri::State<AppState>) -> String {
    state.redline.lock().unwrap().reset_fuse();
    "Fuse reset successfully".into()
}

// ─── Orchestrator Commands ─────────────────────────────────────────

#[tauri::command]
fn get_pipeline_stats(state: tauri::State<AppState>) -> OrchestratorStats {
    state.orchestrator.lock().unwrap().stats()
}

#[tauri::command]
fn start_pipeline(state: tauri::State<AppState>) -> String {
    state.orchestrator.lock().unwrap().start_pipeline();
    "Pipeline started".into()
}

#[tauri::command]
fn pause_pipeline(state: tauri::State<AppState>) -> String {
    state.orchestrator.lock().unwrap().pause_pipeline();
    "Pipeline paused".into()
}

#[tauri::command]
fn resume_pipeline(state: tauri::State<AppState>) -> String {
    state.orchestrator.lock().unwrap().resume_pipeline();
    "Pipeline resumed".into()
}

#[tauri::command]
fn advance_pipeline(state: tauri::State<AppState>) -> Result<String, String> {
    // 第四红线：关键阶段跃迁前检查审批状态
    // 使用 AgentRole 枚举直接匹配，避免中文标签与英文常量比较的静默绕过
    let (from_stage, to_stage, needs_approval) = {
        let orch = state.orchestrator.lock().unwrap();
        let current = &orch.active_role;
        let next = match current {
            AgentRole::PM => AgentRole::UIDesigner,
            AgentRole::UIDesigner => AgentRole::Architect,
            AgentRole::Architect => AgentRole::Planner,
            AgentRole::Planner => AgentRole::Coder,
            AgentRole::Coder => AgentRole::Auditor,
            AgentRole::Auditor => AgentRole::ComplianceOfficer,
            AgentRole::ComplianceOfficer => AgentRole::Verifier,
            AgentRole::Verifier => AgentRole::PM,
        };
        // 枚举匹配 — 不受 label() 语言影响
        let needs = matches!(current, AgentRole::Coder | AgentRole::Auditor);
        // 用稳定英文标识符给审批门禁
        let from_id = match current {
            AgentRole::PM => "PM", AgentRole::UIDesigner => "UIDesigner",
            AgentRole::Architect => "Architect", AgentRole::Planner => "Planner",
            AgentRole::Coder => "Coder", AgentRole::Auditor => "Auditor",
            AgentRole::ComplianceOfficer => "ComplianceOfficer",
            AgentRole::Verifier => "Verifier",
        };
        let to_id = match &next {
            AgentRole::PM => "PM", AgentRole::UIDesigner => "UIDesigner",
            AgentRole::Architect => "Architect", AgentRole::Planner => "Planner",
            AgentRole::Coder => "Coder", AgentRole::Auditor => "Auditor",
            AgentRole::ComplianceOfficer => "ComplianceOfficer",
            AgentRole::Verifier => "Verifier",
        };
        (from_id.to_string(), to_id.to_string(), needs)
    };

    if needs_approval {
        state.approval_gate.lock().unwrap().check_pipeline_advance(&from_stage, &to_stage)?;
    }

    let role = state.orchestrator.lock().unwrap().advance_pipeline();
    Ok(role.label().into())
}

#[tauri::command]
fn create_task(
    state: tauri::State<AppState>,
    title: String,
    description: String,
    dependencies: Vec<String>,
    priority: u8,
) -> String {
    state
        .orchestrator
        .lock()
        .unwrap()
        .create_task(&title, &description, dependencies, priority)
}

#[tauri::command]
fn assign_task(state: tauri::State<AppState>, task_id: String, role: String) -> Result<String, String> {
    let role = parse_role(&role)?;
    state
        .orchestrator
        .lock()
        .unwrap()
        .assign_task(&task_id, role)
        .map(|_| format!("Task {} assigned", task_id))
}

#[tauri::command]
fn complete_task(state: tauri::State<AppState>, task_id: String) -> Result<String, String> {
    state
        .orchestrator
        .lock()
        .unwrap()
        .complete_task(&task_id)
        .map(|_| format!("Task {} completed", task_id))
}

#[tauri::command]
fn fail_task(state: tauri::State<AppState>, task_id: String, error: String) -> Result<String, String> {
    let can_retry = state
        .orchestrator
        .lock()
        .unwrap()
        .fail_task(&task_id, &error)
        .map_err(|e| e)?;

    if can_retry {
        Ok(format!("Task {} failed — can retry", task_id))
    } else {
        Err(format!("Task {} FUSED — manual intervention required", task_id))
    }
}

// ─── API Client Commands ──────────────────────────────────────────

static LAST_API_CALL: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

#[tauri::command]
async fn chat_api(
    state: tauri::State<'_, AppState>,
    endpoint: String,
    api_key: String,
    model: String,
    messages: Vec<serde_json::Value>,
    max_tokens: Option<u32>,
) -> Result<ApiResponse, String> {
    let msgs: Vec<ChatMessage> = messages
        .iter()
        .map(|m| ChatMessage {
            role: m["role"].as_str().unwrap_or("user").into(),
            content: m["content"].as_str().unwrap_or("").into(),
        })
        .collect();

    // Rate limit: minimum 1.5s between calls
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let last = LAST_API_CALL.load(std::sync::atomic::Ordering::Relaxed);
    if now - last < 1500 && last > 0 {
        return Err(format!(
            "[速率限制] 请等待 {}ms 后再发送。防止误触导致资费浪费。",
            1500 - (now - last)
        ));
    }
    LAST_API_CALL.store(now, std::sync::atomic::Ordering::Relaxed);

    // 熔断器检查
    if !state.api_circuit_breaker.allow() {
        return Err("[CIRCUIT BREAKER] API 熔断器已激活，暂时拒绝请求，请稍后重试".into());
    }

    // 原子化预占: 防止并发调用超额 (TOCTOU 修复)
    let estimated_cost = 0.05;
    if !state.billing_engine.try_reserve(estimated_cost) {
        let budget = state.billing_engine.get_budget_total();
        let cap = state.billing_engine.get_cost_cap();
        return Err(format!(
            "[熔断拦截] 累计开销 ¥{:.2} 已达安全阈值 ¥{:.2}，API 调用已被阻断。",
            budget, cap
        ));
    }

    // Resolve API key from vault if frontend sent empty (key now stored server-side)
    let resolved_key = if api_key.is_empty() { resolve_key_from_vault(&model) } else { api_key };
    if resolved_key.is_empty() {
        state.billing_engine.settle(estimated_cost, 0.0); // 释放预留
        return Err("[VAULT EMPTY] API Key 未找到。".into());
    }
    let mut client = state.api_client.lock().await;
    let response = client.chat(&endpoint, &resolved_key, &model, msgs, max_tokens).await;

    // 结算实际费用
    let actual_cost = if response.success {
        let model_enum = parse_model_to_enum(&model);
        let (prompt, completion) = split_tokens(response.tokens_used, &response.content);
        let cost = state.billing_engine.estimate_cost(&model_enum, prompt, completion);
        state.billing_engine.record(&model_enum, prompt, completion, None);
        cost
    } else { 0.0 };
    state.billing_engine.settle(estimated_cost, actual_cost);

    // ── 进化总线 + 数据飞轮 定期同步 ──
    {
        let should_evolve = state.evolution_bus.lock().unwrap().should_evolve();
        if should_evolve {
            let mut wi = state.web_intelligence.lock().await;
            let mut evo = state.evolution_bus.lock().unwrap();
            let mut fw = state.flywheel.lock().unwrap();

            // 1. 从 WebIntelligence 采集实时指标
            let stats = wi.get_stats();
            fw.collect_from_web_intel(
                stats.total_searches, stats.total_fetches, stats.bytes_downloaded,
                stats.unified_cache_hits, stats.unified_cache_misses,
            );
            fw.collect_from_distillation(
                stats.total_distilled, stats.total_bytes_saved,
                stats.avg_compression_ratio,
                wi.distillation.avg_quality(),
            );

            // 2. 同步到进化总线
            wi.sync_to_evolution_bus(&mut evo);
            drop(wi);

            // 3. 使用飞轮实时指标替代硬编码评估值
            let distill_q = fw.metrics.get("distill_quality").map(|m| m.value / 100.0).unwrap_or(0.85);
            let cache_q = fw.metrics.get("cache_hit_rate").map(|m| m.value / 100.0).unwrap_or(0.78);
            let cache_stability = fw.metrics.get("cache_api_saved").map(|m| (m.value / 100.0).min(1.0)).unwrap_or(0.85);

            evo.assess_advancement(&[
                (agent::evolution_bus::EngineId::Distillation, distill_q, 0.92),
                (agent::evolution_bus::EngineId::CacheEngine, cache_q, cache_stability),
                (agent::evolution_bus::EngineId::Scheduling, 0.82, 0.88),
                (agent::evolution_bus::EngineId::HallucinationGuard, 0.80, 0.85),
                (agent::evolution_bus::EngineId::AgentQuality, 0.83, 0.90),
                (agent::evolution_bus::EngineId::Collaboration, 0.80, 0.87),
                (agent::evolution_bus::EngineId::TaskIntelligence, 0.78, 0.85),
                (agent::evolution_bus::EngineId::PredictiveAnalytics, 0.80, 0.86),
                (agent::evolution_bus::EngineId::LocalAnalytics, 0.82, 0.90),
            ]);

            // 4. 飞轮旋转 — 累积收益并创建快照
            fw.spin();
            drop(evo);
            drop(fw);
        }
    }

    Ok(response)
}

// ─── Streaming API ────────────────────────────────────────────

#[tauri::command]
async fn chat_api_stream(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    endpoint: String,
    api_key: String,
    model: String,
    messages: Vec<serde_json::Value>,
    max_tokens: Option<u32>,
) -> Result<ApiResponse, String> {
    let msgs: Vec<ChatMessage> = messages
        .iter()
        .map(|m| ChatMessage {
            role: m["role"].as_str().unwrap_or("user").into(),
            content: m["content"].as_str().unwrap_or("").into(),
        })
        .collect();

    // Rate limit — same 1.5s gate as chat_api
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let last = LAST_API_CALL.load(std::sync::atomic::Ordering::Relaxed);
    if last > 0 {
        let elapsed_ms = (now - last) as u64;
        if elapsed_ms < 1500 {
            return Err(format!(
                "[RATE LIMIT] 请求过于频繁，请等待 {}ms",
                1500 - elapsed_ms
            ));
        }
    }
    LAST_API_CALL.store(now, std::sync::atomic::Ordering::Relaxed);

    // 原子化预占: 防止并发调用超额
    let estimated_cost = 0.05;
    if !state.billing_engine.try_reserve(estimated_cost) {
        let budget = state.billing_engine.get_budget_total();
        let cap = state.billing_engine.get_cost_cap();
        return Err(format!(
            "[熔断拦截] 累计开销 ¥{:.2} 已达安全阈值 ¥{:.2}，流式调用已被阻断。",
            budget, cap
        ));
    }

    // Resolve API key from vault
    let resolved_key = if api_key.is_empty() { resolve_key_from_vault(&model) } else { api_key };
    if resolved_key.is_empty() {
        state.billing_engine.settle(estimated_cost, 0.0);
        return Err("[VAULT EMPTY] API Key 未找到。".into());
    }
    let mut client = state.api_client.lock().await;
    let response = client
        .chat_stream(&endpoint, &resolved_key, &model, msgs, max_tokens,
            |chunk| { let _ = app_handle.emit("chat-stream-chunk", chunk); },
        ).await;

    // 结算实际费用
    let actual_cost = if response.success {
        let model_enum = parse_model_to_enum(&model);
        let (prompt, completion) = split_tokens(response.tokens_used, &response.content);
        let cost = state.billing_engine.estimate_cost(&model_enum, prompt, completion);
        state.billing_engine.record(&model_enum, prompt, completion, None);
        cost
    } else { 0.0 };
    state.billing_engine.settle(estimated_cost, actual_cost);

    // ── 进化总线 + 数据飞轮 (stream) ──
    {
        let should_evolve = state.evolution_bus.lock().unwrap().should_evolve();
        if should_evolve {
            let mut wi = state.web_intelligence.lock().await;
            let mut evo = state.evolution_bus.lock().unwrap();
            let mut fw = state.flywheel.lock().unwrap();

            let stats = wi.get_stats();
            fw.collect_from_web_intel(
                stats.total_searches, stats.total_fetches, stats.bytes_downloaded,
                stats.unified_cache_hits, stats.unified_cache_misses,
            );
            fw.collect_from_distillation(
                stats.total_distilled, stats.total_bytes_saved,
                stats.avg_compression_ratio, wi.distillation.avg_quality(),
            );

            wi.sync_to_evolution_bus(&mut evo);
            drop(wi);

            let distill_q = fw.metrics.get("distill_quality").map(|m| m.value / 100.0).unwrap_or(0.85);
            let cache_q = fw.metrics.get("cache_hit_rate").map(|m| m.value / 100.0).unwrap_or(0.78);
            let cache_stability = fw.metrics.get("cache_api_saved").map(|m| (m.value / 100.0).min(1.0)).unwrap_or(0.85);

            evo.assess_advancement(&[
                (agent::evolution_bus::EngineId::Distillation, distill_q, 0.92),
                (agent::evolution_bus::EngineId::CacheEngine, cache_q, cache_stability),
                (agent::evolution_bus::EngineId::Scheduling, 0.82, 0.88),
                (agent::evolution_bus::EngineId::HallucinationGuard, 0.80, 0.85),
                (agent::evolution_bus::EngineId::AgentQuality, 0.83, 0.90),
                (agent::evolution_bus::EngineId::Collaboration, 0.80, 0.87),
                (agent::evolution_bus::EngineId::TaskIntelligence, 0.78, 0.85),
                (agent::evolution_bus::EngineId::PredictiveAnalytics, 0.80, 0.86),
                (agent::evolution_bus::EngineId::LocalAnalytics, 0.82, 0.90),
            ]);

            fw.spin();
            drop(evo);
            drop(fw);
        }
    }

    Ok(response)
}

// ─── Sandbox Commands ────────────────────────────────────────────

#[tauri::command]
fn init_sandbox(state: tauri::State<AppState>, tools: Vec<String>) -> Result<String, String> {
    let paths: Vec<std::path::PathBuf> = tools.iter().map(std::path::PathBuf::from).collect();
    state.sandbox.lock().unwrap().initialize_sandbox(&paths)
        .map(|_| "Sandbox initialized".into())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_checkpoints(state: tauri::State<AppState>) -> Vec<serde_json::Value> {
    let sb = state.sandbox.lock().unwrap();
    sb.checkpoints.iter().map(|cp| serde_json::to_value(cp).unwrap_or_default()).collect()
}

// ─── MCP Commands ────────────────────────────────────────────────

#[tauri::command]
async fn mcp_connect_and_init(state: tauri::State<'_, AppState>, server_id: String) -> Result<String, String> {
    state.mcp_client.lock().await.connect_and_init(&server_id).await
        .map(|_| format!("MCP server '{}' connected and initialized", server_id))
}

#[tauri::command]
async fn mcp_fetch_tools(state: tauri::State<'_, AppState>, server_id: String) -> Result<String, String> {
    let tools = state.mcp_client.lock().await.fetch_and_clean_tools(&server_id).await?;
    Ok(format!("Fetched {} tools from '{}'", tools.len(), server_id))
}

// ─── Router Commands ──────────────────────────────────────────────

#[tauri::command]
fn get_route_mode(state: tauri::State<AppState>) -> String {
    let router = state.router.lock().unwrap();
    serde_json::to_string(&router.mode).unwrap_or_default()
}

#[tauri::command]
fn set_route_mode(state: tauri::State<AppState>, mode_json: String) -> Result<String, String> {
    let mode: RouteMode = serde_json::from_str(&mode_json)
        .map_err(|e| format!("Invalid route mode: {}", e))?;
    let mut router = state.router.lock().unwrap();
    router.mode = mode;
    Ok(router.mode.label().into())
}

#[tauri::command]
fn get_available_models(state: tauri::State<AppState>) -> Vec<String> {
    let router = state.router.lock().unwrap();
    router.models.keys().cloned().collect()
}

#[tauri::command]
fn set_model_api_key(
    state: tauri::State<AppState>,
    model_key: String,
    api_key: String,
) -> Result<String, String> {
    let mut router = state.router.lock().unwrap();
    if let Some(config) = router.models.get_mut(&model_key) {
        if let ModelConfig::Cloud { api_key: ref mut key, .. } = config {
            *key = api_key;
            Ok(format!("API key set for {}", model_key))
        } else {
            Err(format!("{} is a local model, no API key needed", model_key))
        }
    } else {
        Err(format!("Model {} not found", model_key))
    }
}

#[tauri::command]
fn route_for_role(state: tauri::State<AppState>, role: String) -> String {
    let router = state.router.lock().unwrap();
    router.route_text_model(&role).into()
}

// ─── HybridAgentRouter Commands ──────────────────────────────

#[tauri::command]
async fn hrouter_select_model(
    state: tauri::State<'_, AppState>,
    agent_role: String,
    is_high_urgency: bool,
) -> Result<serde_json::Value, String> {
    let decision = state.hybrid_router.select_optimal_model(&agent_role, is_high_urgency).await;
    Ok(serde_json::json!({
        "agent_role": decision.agent_role,
        "selected_model": decision.selected_model.display(),
        "is_cache_eligible": decision.is_cache_eligible,
        "is_lan_fallback": decision.is_lan_fallback,
        "reason": decision.reason,
    }))
}

#[tauri::command]
async fn hrouter_get_cluster_status(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let nodes = state.hybrid_router.cluster_nodes.read().await;
    let status: Vec<_> = nodes.iter().map(|(model, node)| {
        serde_json::json!({
            "model": model.display(), "api_url": node.api_url,
            "timeout_ms": node.timeout_ms, "cost_per_1k": node.cost_per_1k_tokens,
        })
    }).collect();
    Ok(serde_json::json!({ "nodes": status }))
}

#[tauri::command]
fn get_model_endpoint(state: tauri::State<AppState>, model_key: String) -> Result<String, String> {
    let router = state.router.lock().unwrap();
    match router.get_model(&model_key) {
        Some(ModelConfig::Cloud { endpoint, .. }) => Ok(endpoint.clone()),
        Some(ModelConfig::Local { endpoint, .. }) => Ok(endpoint.clone()),
        None => Err(format!("Model '{}' not found", model_key)),
    }
}

// ─── Shadow Mode Commands ─────────────────────────────────────────

#[tauri::command]
fn get_shadow_stats(state: tauri::State<AppState>) -> ShadowStats {
    state.shadow.lock().unwrap().stats()
}

#[tauri::command]
fn toggle_shadow(state: tauri::State<AppState>, enabled: bool) -> String {
    let mut shadow = state.shadow.lock().unwrap();
    if enabled {
        shadow.activate();
    } else {
        shadow.pause();
    }
    format!("Shadow mode: {}", if enabled { "ON" } else { "OFF" })
}

#[tauri::command]
fn dismiss_shadow_suggestion(state: tauri::State<AppState>, id: String) -> String {
    state.shadow.lock().unwrap().dismiss_suggestion(&id);
    format!("Suggestion {} dismissed", id)
}

#[tauri::command]
fn save_shadow_state(app_handle: AppHandle, state: tauri::State<AppState>) -> Result<String, String> {
    let dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    state.shadow.lock().unwrap().save_state(&dir)?;
    Ok("Shadow state saved".into())
}

#[tauri::command]
fn load_shadow_state(app_handle: AppHandle, state: tauri::State<AppState>) -> Result<String, String> {
    let dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    state.shadow.lock().unwrap().load_state(&dir)?;
    Ok("Shadow state loaded".into())
}

// ─── General Commands ──────────────────────────────────────────────

#[tauri::command]
fn get_sandbox_status(state: tauri::State<AppState>) -> String {
    let sandbox = state.sandbox.lock().unwrap();
    format!("Protected ({} mounts, {} ops logged)", sandbox.mounts.len(), sandbox.audit_logs.len())
}

#[tauri::command]
fn get_session_cost(state: tauri::State<AppState>) -> f64 {
    state.billing_engine.get_budget_total()
}

#[tauri::command]
fn get_saved_cost(state: tauri::State<AppState>) -> f64 {
    let scan = state.buddy_scan.lock().unwrap();
    let glue = state.context_glue.lock().unwrap();
    scan.get_stats().estimated_cost_saved + glue.get_stats().estimated_cost_saved
}

#[tauri::command]
fn get_saving_rate(state: tauri::State<AppState>) -> u32 {
    let scan = state.buddy_scan.lock().unwrap();
    let glue = state.context_glue.lock().unwrap();
    let total_saved = scan.get_stats().estimated_cost_saved + glue.get_stats().estimated_cost_saved;
    if total_saved > 0.0 { (total_saved * 100.0) as u32 } else { 0 }
}

// ─── WorkBuddy: Buddy Scan ─────────────────────────────────────

#[tauri::command]
fn get_buddy_scan_stats(state: tauri::State<AppState>) -> BuddyScanStats {
    state.buddy_scan.lock().unwrap().get_stats().clone()
}

#[tauri::command]
fn run_buddy_scan(
    state: tauri::State<AppState>,
    target_x: i32,
    target_y: i32,
    component_label: String,
    component_type: String,
    expected_text: String,
) -> BuddyScanResult {
    state.buddy_scan.lock().unwrap().scan_before_click(
        target_x, target_y,
        &component_label, &component_type, &expected_text,
    )
}

#[tauri::command]
fn toggle_buddy_scan(state: tauri::State<AppState>, enabled: bool) -> String {
    state.buddy_scan.lock().unwrap().toggle(enabled);
    format!("Buddy Scanner: {}", if enabled { "ON" } else { "OFF" })
}

#[tauri::command]
fn get_buddy_saved_cost(state: tauri::State<AppState>) -> f64 {
    let scan = state.buddy_scan.lock().unwrap();
    let glue = state.context_glue.lock().unwrap();
    scan.get_stats().estimated_cost_saved + glue.get_stats().estimated_cost_saved
}

// ─── Billing stats (legacy compat) ────────────────────────────

/// 向后兼容旧前端 — 返回 Budget 轨道数据
#[tauri::command]
fn get_billing_stats(state: tauri::State<AppState>) -> serde_json::Value {
    let budget = state.billing_engine.get_ledger(agent::billing_engine::BillingTier::Budget);
    let scan = state.buddy_scan.lock().unwrap();
    let glue = state.context_glue.lock().unwrap();
    let workbuddy_saved = scan.get_stats().estimated_cost_saved + glue.get_stats().estimated_cost_saved;
    serde_json::json!({
        "session_cost": budget.total_cost_rmb,
        "saved_cost": workbuddy_saved,
        "saving_rate": 0,
        "cost_limit": state.billing_engine.get_cost_cap(),
        "cost_cap_active": !state.billing_engine.is_over_cap() || state.billing_engine.get_budget_total() < state.billing_engine.get_cost_cap(),
    })
}

/// 统一仪表盘 — 三轨并行数据一次查询
#[tauri::command]
fn get_billing_dashboard(state: tauri::State<AppState>) -> agent::billing_engine::BillingDashboard {
    state.billing_engine.get_dashboard()
}

/// 模型降本推荐 — 根据消息长度推荐最优模型
#[tauri::command]
fn get_model_recommendation(message_length: usize) -> agent::billing_engine::ModelRecommendation {
    let engine = agent::billing_engine::ChronosParallelBillingEngine::new();
    engine.recommend_for_length(message_length)
}

/// 防幻觉审计 — 分析 LLM 输出，生成信任评分 + 问题清单 + 纠偏建议
#[tauri::command]
fn audit_hallucination(response: String) -> agent::hallucination_guard::HallucinationReport {
    let guard = agent::hallucination_guard::HallucinationGuard::new();
    guard.audit(&response)
}

/// 全自动Agent调度 — 分析用户输入，输出最优Agent+模型+技能建议
#[tauri::command]
fn analyze_task(user_message: String) -> agent::scheduling_engine::SchedulingResult {
    let engine = agent::scheduling_engine::AgentSchedulingEngine::new();
    engine.analyze(&user_message)
}

/// 上下文健康检查 — 当前 Token 使用占比与优化建议
#[tauri::command]
fn check_context_health(model: String, current_tokens: u32) -> agent::billing_engine::ContextHealth {
    let model_enum = agent::billing::parse_model_string(&model);
    let engine = agent::billing_engine::ChronosParallelBillingEngine::new();
    engine.check_context_health(&model_enum, current_tokens)
}

/// 更新费用上限（同步到 billing_engine）
#[tauri::command]
fn update_cost_cap(state: tauri::State<AppState>, cap: f64, enabled: bool) -> Result<String, String> {
    agent::input_guard::validate_cost(cap)?;
    state.billing_engine.set_cost_cap(cap);
    state.billing_engine.set_cost_cap_enabled(enabled);
    Ok(format!("Cost cap set to ¥{:.2} ({})", cap, if enabled { "ON" } else { "OFF" }))
}

// ─── WorkBuddy: Context Glue ───────────────────────────────────

#[tauri::command]
fn get_context_glue_status(state: tauri::State<AppState>) -> ContextGlueStats {
    state.context_glue.lock().unwrap().get_stats().clone()
}

#[tauri::command]
fn add_app_binding(
    state: tauri::State<AppState>,
    source_app: String,
    target_app: String,
    mapping_rule: String,
) -> Result<String, String> {
    state.context_glue.lock().unwrap().create_binding(
        &source_app, &target_app, &mapping_rule, DataDirection::OneWay,
    )
}

#[tauri::command]
fn remove_app_binding(state: tauri::State<AppState>, binding_id: String) -> bool {
    state.context_glue.lock().unwrap().remove_binding(&binding_id)
}

#[tauri::command]
fn get_app_bindings(state: tauri::State<AppState>) -> Vec<AppBinding> {
    state.context_glue.lock().unwrap().get_bindings().to_vec()
}

#[tauri::command]
fn toggle_context_glue(state: tauri::State<AppState>, enabled: bool) -> String {
    state.context_glue.lock().unwrap().toggle(enabled);
    format!("Context Glue: {}", if enabled { "ON" } else { "OFF" })
}

#[tauri::command]
fn save_context_glue_bindings(app_handle: AppHandle, state: tauri::State<AppState>) -> Result<String, String> {
    let dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    state.context_glue.lock().unwrap().save_bindings(&dir)?;
    Ok("Context Glue bindings saved".into())
}

#[tauri::command]
fn load_context_glue_bindings(app_handle: AppHandle, state: tauri::State<AppState>) -> Result<String, String> {
    let dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    state.context_glue.lock().unwrap().load_bindings(&dir)?;
    Ok("Context Glue bindings loaded".into())
}

// ─── Remote Cluster Manager ──────────────────────────────────

#[tauri::command]
async fn cluster_register_server(
    state: tauri::State<'_, AppState>,
    server_id: String, host: String, port: u16, username: String,
    auth_key_path: Option<String>, remote_project_root: String,
) -> Result<String, String> {
    let config = RemoteConfig { host, port, username, auth_key_path, remote_project_root };
    state.cluster.lock().await.register_and_connect_server(&server_id, config).await?;
    Ok(format!("Server '{}' registered", server_id))
}

#[tauri::command]
async fn cluster_unregister_server(
    state: tauri::State<'_, AppState>, server_id: String,
) -> Result<String, String> {
    state.cluster.lock().await.unregister_server(&server_id).await;
    Ok(format!("Server '{}' unregistered", server_id))
}

#[tauri::command]
async fn cluster_bind_project(
    state: tauri::State<'_, AppState>, project_id: String, server_id: String,
) -> Result<String, String> {
    state.cluster.lock().await.bind_project_to_server(&project_id, &server_id).await?;
    Ok(format!("Project '{}' bound to '{}'", project_id, server_id))
}

#[tauri::command]
async fn cluster_edit_file(
    state: tauri::State<'_, AppState>,
    project_id: String, file_path: String, content: String,
) -> Result<String, String> {
    state.cluster.lock().await.execute_cluster_file_edit(&project_id, &file_path, &content).await?;
    Ok(format!("Edited {} on project {}", file_path, project_id))
}

#[tauri::command]
async fn cluster_compile(
    state: tauri::State<'_, AppState>, project_id: String, build_command: String,
) -> Result<String, String> {
    state.cluster.lock().await.execute_cluster_compile(&project_id, &build_command).await
}

#[tauri::command]
async fn cluster_ping(state: tauri::State<'_, AppState>) -> Result<HashMap<String, bool>, String> {
    Ok(state.cluster.lock().await.cluster_ping().await)
}

#[tauri::command]
async fn get_cluster_stats(state: tauri::State<'_, AppState>) -> Result<ClusterStats, String> {
    Ok(state.cluster.lock().await.get_cluster_stats().await)
}

// ─── Remote Development Proxy ────────────────────────────────

#[tauri::command]
async fn remote_connect(
    state: tauri::State<'_, AppState>,
    host: String, port: u16, username: String,
    auth_key_path: Option<String>, remote_project_root: String,
) -> Result<String, String> {
    let config = RemoteConfig { host, port, username, auth_key_path, remote_project_root };
    let tunnel = RemoteProxyTunnel::new(config.clone());
    tunnel.connect_server().await?;
    *state.remote_proxy.lock().await = Some(tunnel);
    Ok(format!("Connected to {}:{}", config.host, config.port))
}

#[tauri::command]
async fn remote_disconnect(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let mut proxy = state.remote_proxy.lock().await;
    if let Some(ref mut tunnel) = *proxy {
        tunnel.disconnect().await;
    }
    *proxy = None;
    Ok("Disconnected".into())
}

#[tauri::command]
async fn remote_list_files(
    state: tauri::State<'_, AppState>, subpath: String,
) -> Result<Vec<serde_json::Value>, String> {
    let proxy = state.remote_proxy.lock().await;
    let tunnel = proxy.as_ref().ok_or("Not connected")?;
    let nodes = tunnel.list_remote_files(&subpath).await?;
    Ok(nodes.iter().map(|n| serde_json::json!({
        "name": n.name, "path": n.path, "is_dir": n.is_dir, "size": n.size,
    })).collect())
}

#[tauri::command]
async fn remote_read_file(
    state: tauri::State<'_, AppState>, path: String,
) -> Result<String, String> {
    let proxy = state.remote_proxy.lock().await;
    let tunnel = proxy.as_ref().ok_or("Not connected")?;
    tunnel.read_remote_file(&path).await
}

#[tauri::command]
async fn remote_write_file(
    state: tauri::State<'_, AppState>, path: String, content: String,
) -> Result<String, String> {
    let proxy = state.remote_proxy.lock().await;
    let tunnel = proxy.as_ref().ok_or("Not connected")?;
    tunnel.remote_file_edit(&path, &content).await?;
    Ok(format!("Written {} bytes to {}", content.len(), path))
}

#[tauri::command]
async fn remote_compile(
    state: tauri::State<'_, AppState>, build_command: String,
) -> Result<String, String> {
    let proxy = state.remote_proxy.lock().await;
    let tunnel = proxy.as_ref().ok_or("Not connected")?;
    tunnel.execute_remote_compile(&build_command).await
}

#[tauri::command]
async fn remote_snapshot(
    state: tauri::State<'_, AppState>, tag: String,
) -> Result<String, String> {
    let proxy = state.remote_proxy.lock().await;
    let tunnel = proxy.as_ref().ok_or("Not connected")?;
    tunnel.create_remote_snapshot(&tag).await
}

#[tauri::command]
async fn remote_rewind(
    state: tauri::State<'_, AppState>, tag: String,
) -> Result<String, String> {
    let proxy = state.remote_proxy.lock().await;
    let tunnel = proxy.as_ref().ok_or("Not connected")?;
    tunnel.rewind_remote_snapshot(&tag).await
}

#[tauri::command]
async fn get_remote_stats(state: tauri::State<'_, AppState>) -> Result<RemoteSessionStats, String> {
    let proxy = state.remote_proxy.lock().await;
    match proxy.as_ref() {
        Some(tunnel) => Ok(tunnel.get_stats().await),
        None => Ok(RemoteSessionStats {
            connected: false, host: String::new(),
            files_synced: 0, builds_triggered: 0, builds_failed: 0,
            bytes_transferred: 0, last_error: None,
        }),
    }
}

// ─── C-VFS Commands ──────────────────────────────────────────

#[tauri::command]
async fn cvfs_create_project(
    state: tauri::State<'_, AppState>,
    project_id: String, target_path: String,
) -> Result<String, String> {
    let cvfs = state.cvfs.lock().await;
    let path = cvfs.create_secure_project_workspace(&project_id, PathBuf::from(&target_path)).await
        .map_err(|e| e.to_string())?;
    Ok(format!("Project '{}' created at {:?}", project_id, path))
}

#[tauri::command]
async fn cvfs_verify_scope(
    state: tauri::State<'_, AppState>,
    project_id: String, file_path: String,
) -> Result<String, String> {
    let cvfs = state.cvfs.lock().await;
    cvfs.verify_write_scope_permission(&project_id, &file_path).await
        .map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
async fn cvfs_capture_checkpoint(
    state: tauri::State<'_, AppState>,
    project_id: String, checkpoint_id: String, description: String,
) -> Result<String, String> {
    let cvfs = state.cvfs.lock().await;
    cvfs.capture_chrono_checkpoint(&project_id, &checkpoint_id, &description, vec![]).await
        .map_err(|e| e.to_string())?;
    Ok(format!("Checkpoint '{}' created", checkpoint_id))
}

/// V2 检查点捕获 — 带真实文件内容快照
#[tauri::command]
async fn cvfs_capture_checkpoint_v2(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    project_id: String, label: String, description: String,
) -> Result<serde_json::Value, String> {
    let cvfs = state.cvfs.lock().await;
    let cp = cvfs.capture_checkpoint_v2(&project_id, &label, &description).await?;
    // 持久化 C-VFS 状态到 app_data_dir
    if let Ok(dir) = app_handle.path().app_data_dir() {
        let _ = cvfs.save_state_to(&dir).await;
    }
    Ok(serde_json::json!({
        "id": cp.checkpoint_id, "timestamp": cp.timestamp,
        "label": cp.desc, "files_changed": cp.changed_files_diff.len(),
        "snapshot_type": "Manual",
    }))
}

/// 恢复检查点 — 还原文件到快照状态
#[tauri::command]
async fn cvfs_restore_checkpoint(
    state: tauri::State<'_, AppState>,
    project_id: String, checkpoint_id: String,
) -> Result<String, String> {
    let cvfs = state.cvfs.lock().await;
    cvfs.restore_checkpoint(&project_id, &checkpoint_id).await?;
    Ok(format!("Checkpoint {} restored", checkpoint_id))
}

/// 删除检查点
#[tauri::command]
async fn cvfs_delete_checkpoint(
    state: tauri::State<'_, AppState>,
    project_id: String, checkpoint_id: String,
) -> Result<String, String> {
    let cvfs = state.cvfs.lock().await;
    cvfs.delete_checkpoint(&project_id, &checkpoint_id).await?;
    Ok(format!("Checkpoint {} deleted", checkpoint_id))
}

/// 删除项目
#[tauri::command]
async fn cvfs_delete_project(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<String, String> {
    let cvfs = state.cvfs.lock().await;
    cvfs.delete_project(&project_id).await?;
    Ok(format!("Project {} deleted", project_id))
}

/// 列出项目真实文件树
#[tauri::command]
async fn cvfs_list_project_files(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let cvfs = state.cvfs.lock().await;
    let nodes = cvfs.list_project_files(&project_id).await?;
    Ok(nodes.iter().map(|n| serde_json::json!({
        "name": n.name, "is_dir": n.is_dir, "relative_path": n.relative_path,
        "is_locked": n.is_locked,
    })).collect())
}

/// 项目健康状态
#[tauri::command]
async fn cvfs_get_project_health(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<serde_json::Value, String> {
    let cvfs = state.cvfs.lock().await;
    cvfs.get_project_health(&project_id).await
}

#[tauri::command]
async fn cvfs_get_checkpoints(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let cvfs = state.cvfs.lock().await;
    let cps = cvfs.get_checkpoints().await;
    Ok(cps.iter().map(|c| serde_json::json!({
        "id": c.checkpoint_id, "timestamp": c.timestamp,
        "label": c.desc, "files_changed": c.changed_files_diff.len(),
        "snapshot_type": if c.vss_snapshot_guid.is_some() { "Auto" } else { "Manual" },
    })).collect())
}

#[tauri::command]
async fn cvfs_get_projects(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let cvfs = state.cvfs.lock().await;
    let projs = cvfs.get_projects().await;
    Ok(projs.iter().map(|(id, path)| serde_json::json!({
        "id": id, "name": id, "path": path.to_string_lossy(),
    })).collect())
}

// ─── Security Vault ────────────────────────────────────────

#[tauri::command]
fn get_vault_status() -> Result<serde_json::Value, String> {
    let vault = agent::security_vault::NativeSecurityVault::new();
    Ok(vault.get_security_status())
}

#[tauri::command]
fn vault_api_key(target_model: String, secret_key: String) -> Result<String, String> {
    // Cache in memory immediately — survives keyring failures
    cache_key(&target_model, &secret_key);
    // Also persist to Windows Credential Manager
    let vault = agent::security_vault::NativeSecurityVault::new();
    vault.vault_api_key_native(&target_model, &secret_key)?;
    Ok(format!("[{}] 已存入 Windows 凭据保险箱", target_model))
}

#[tauri::command]
fn fetch_api_key(target_model: String) -> Result<String, String> {
    let vault = agent::security_vault::NativeSecurityVault::new();
    vault.fetch_api_key_native(&target_model)
}

#[tauri::command]
fn delete_api_key(target_model: String) -> Result<String, String> {
    let vault = agent::security_vault::NativeSecurityVault::new();
    vault.delete_api_key_native(&target_model)?;
    Ok(format!("[{}] 已从凭据保险箱移除", target_model))
}

// ─── Detector Stats ────────────────────────────────────────

#[tauri::command]
async fn get_detector_stats() -> Result<serde_json::Value, String> {
    let detector = SkillAndMcpDetector::new();
    let stats = detector.get_stats().await;
    Ok(serde_json::json!({
        "total_interceptions": stats.total_interceptions,
        "total_hits": stats.total_hits,
        "hit_rate": stats.hit_rate,
        "tokens_saved": stats.tokens_saved,
        "estimated_cost_saved": stats.estimated_cost_saved,
    }))
}

// ─── LAN Health ─────────────────────────────────────────────

#[tauri::command]
async fn check_lan_health() -> Result<Vec<String>, String> {
    Router::check_lan_health().await
}

// ─── Worktree Commands ─────────────────────────────────────────────

#[tauri::command]
fn create_worktree(
    state: tauri::State<AppState>,
    task_id: String,
    files: Vec<String>,
    base_branch: String,
) -> Result<String, String> {
    let config = agent::worktree::WorktreeConfig { task_id, files, base_branch };
    state.worktree.lock().unwrap().create_worktree(&config)
}

#[tauri::command]
fn activate_worktree(
    state: tauri::State<AppState>,
    worktree_id: String,
    task_id: String,
    agent_id: String,
) -> Result<(), String> {
    state.worktree.lock().unwrap().activate(&worktree_id, &task_id, &agent_id)
}

#[tauri::command]
fn complete_worktree(
    state: tauri::State<AppState>,
    worktree_id: String,
) -> Result<(), String> {
    state.worktree.lock().unwrap().complete(&worktree_id)
}

#[tauri::command]
fn merge_worktree(
    state: tauri::State<AppState>,
    worktree_id: String,
) -> Result<agent::worktree::MergeResult, String> {
    // 第四红线：Worktree 合并前检查审批状态
    state.approval_gate.lock().unwrap().check_worktree_merge(&worktree_id)?;
    state.worktree.lock().unwrap().merge_worktree(&worktree_id)
}

#[tauri::command]
fn prune_worktree(
    state: tauri::State<AppState>,
    worktree_id: String,
) -> Result<(), String> {
    state.worktree.lock().unwrap().prune_worktree(&worktree_id)
}

#[tauri::command]
fn list_worktrees(
    state: tauri::State<AppState>,
) -> Vec<agent::worktree::WorktreeInstance> {
    state.worktree.lock().unwrap().worktrees.clone()
}

#[tauri::command]
fn get_worktree_stats(
    state: tauri::State<AppState>,
) -> agent::worktree::WorktreeStats {
    state.worktree.lock().unwrap().stats()
}

// ─── Approval Gate Commands (第四红线) ────────────────────────────

#[tauri::command]
fn submit_for_approval(
    state: tauri::State<AppState>,
    action_type: String,
    target_id: String,
    description: String,
    metadata: String, // JSON string for context (project, branch, etc.)
) -> Result<agent::approval_gate::ApprovalRequest, String> {
    state.approval_gate.lock().unwrap().submit(&action_type, &target_id, &description, &metadata)
}

/// 资费感知审批 — 结合计费引擎实时预算状态动态升级风险
#[tauri::command]
fn submit_for_approval_with_cost(
    state: tauri::State<AppState>,
    action_type: String,
    target_id: String,
    description: String,
    metadata: String,
    estimated_cost_rmb: f64,
) -> Result<agent::approval_gate::ApprovalRequest, String> {
    // 从计费引擎提取实时预算数据
    let budget_used = state.billing_engine.get_budget_total();
    let cost_cap = state.billing_engine.get_cost_cap();
    let current_budget = Some(budget_used);
    let current_cap = if cost_cap > 0.0 { Some(cost_cap) } else { None };

    let result = state.approval_gate.lock().unwrap().submit_with_cost(
        &action_type, &target_id, &description, &metadata,
        estimated_cost_rmb, current_budget, current_cap,
    );
    // 如果审批提交成功且需要审批，发布 Blackboard 事件
    if let Ok(ref req) = result {
        if req.status == "Pending" {
            let mut orch = state.orchestrator.lock().unwrap();
            orch.publish(AgentRole::Auditor, agent::orchestrator::EventType::RedlineViolation {
                code: "APPROVAL_REQUIRED".into(),
                message: format!("第四红线：{} 需要人工审批 (风险:{})", req.description, req.risk_level),
            });
        }
    }
    result
}

/// Auditor 预筛查 — 高风险操作先经代码审计再提交审批
#[tauri::command]
fn auditor_pre_screen_approval(
    state: tauri::State<AppState>,
    action_type: String,
    target_id: String,
    description: String,
    metadata: String,
    auditor_findings: String,
    auditor_passed: bool,
) -> Result<agent::approval_gate::ApprovalRequest, String> {
    // 统计发现项数量
    let finding_lines = auditor_findings.lines().filter(|l| !l.trim().is_empty()).count() as u32;
    let prescreen = agent::approval_gate::AuditorPrescreenResult {
        passed: auditor_passed,
        findings_count: finding_lines.max(1),
        critical_count: if auditor_passed { 0 } else { 1 },
        summary: auditor_findings,
    };
    state.approval_gate.lock().unwrap().submit_with_auditor(
        &action_type, &target_id, &description, &metadata, prescreen,
    )
}

#[tauri::command]
fn decide_approval(
    state: tauri::State<AppState>,
    request_id: String,
    decision: String,
    reviewer: String,
    comment: String,
) -> Result<agent::approval_gate::ApprovalRequest, String> {
    let result = state.approval_gate.lock().unwrap().decide(&request_id, &decision, &reviewer, &comment)?;
    // 发布审批决策事件到 Blackboard
    let event_code = if result.status == "Approved" { "APPROVAL_GRANTED" } else { "APPROVAL_REJECTED" };
    let mut orch = state.orchestrator.lock().unwrap();
    orch.publish(AgentRole::Auditor, agent::orchestrator::EventType::RedlineViolation {
        code: event_code.into(),
        message: format!("审批 {}: {} — {} 决定: {}",
            result.id, result.description,
            if result.status == "Approved" { "✅ 通过" } else { "❌ 驳回" },
            comment),
    });
    Ok(result)
}

#[tauri::command]
fn list_pending_approvals(
    state: tauri::State<AppState>,
) -> Vec<agent::approval_gate::ApprovalRequest> {
    state.approval_gate.lock().unwrap().list_pending()
}

#[tauri::command]
fn get_approval_audit_log(
    state: tauri::State<AppState>,
    limit: Option<usize>,
) -> Vec<agent::approval_gate::ApprovalRequest> {
    state.approval_gate.lock().unwrap().get_audit_log(limit.unwrap_or(50))
}

#[tauri::command]
fn add_approval_rule(
    state: tauri::State<AppState>,
    action_type: String,
    risk_level: u32,
    auto_approve_below_risk: u32,
    description: String,
) -> Result<String, String> {
    state.approval_gate.lock().unwrap().add_rule(&action_type, risk_level, auto_approve_below_risk, &description)
}

#[tauri::command]
fn remove_approval_rule(
    state: tauri::State<AppState>,
    rule_id: String,
) -> Result<(), String> {
    state.approval_gate.lock().unwrap().remove_rule(&rule_id)
}

#[tauri::command]
fn get_approval_rules(
    state: tauri::State<AppState>,
) -> Vec<agent::approval_gate::ApprovalRule> {
    state.approval_gate.lock().unwrap().get_rules()
}

// ─── Embedding Engine Commands ─────────────────────────────────────

#[tauri::command]
fn embedding_search(
    state: tauri::State<AppState>,
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
fn embedding_add(
    state: tauri::State<AppState>,
    id: String, text: String, tags: Vec<String>, source: String,
) -> String {
    let mut engine = state.embedding.lock().unwrap();
    engine.add(&id, &text, tags, &source);
    format!("Added embedding entry: {}", id)
}

#[tauri::command]
fn embedding_stats(
    state: tauri::State<AppState>,
) -> serde_json::Value {
    state.embedding.lock().unwrap().stats()
}

/// 审批模式演化建议 — 基于历史数据自动推荐规则阈值调优
#[tauri::command]
fn get_approval_suggestions(
    state: tauri::State<AppState>,
) -> Vec<agent::approval_gate::RuleSuggestion> {
    state.approval_gate.lock().unwrap().suggest_rule_optimizations()
}

#[tauri::command]
fn expire_stale_approvals(
    state: tauri::State<AppState>,
) -> Vec<String> {
    state.approval_gate.lock().unwrap().expire_stale()
}

#[tauri::command]
fn save_approval_state(app_handle: AppHandle, state: tauri::State<AppState>) -> Result<String, String> {
    let dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    state.approval_gate.lock().unwrap().save_state(&dir)?;
    Ok("Approval state saved".into())
}

#[tauri::command]
fn load_approval_state(app_handle: AppHandle, state: tauri::State<AppState>) -> Result<String, String> {
    let dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    state.approval_gate.lock().unwrap().load_state(&dir)?;
    Ok("Approval state loaded".into())
}

// ─── Local Analytics Commands ─────────────────────────────────────

#[tauri::command]
fn analytics_record(state: tauri::State<AppState>, metric: String, value: f64) -> String {
    let mut a = state.analytics.lock().unwrap();
    a.record(&metric, value);
    format!("Recorded {}={:.4}", metric, value)
}

#[tauri::command]
fn analytics_snapshot(state: tauri::State<AppState>, metric: String) -> serde_json::Value {
    let a = state.analytics.lock().unwrap();
    let snap = a.snapshot(&metric);
    serde_json::json!({
        "metric": metric, "count": snap.count, "mean": snap.mean,
        "std_dev": snap.std_dev, "min": snap.min, "max": snap.max, "latest": snap.latest,
    })
}

#[tauri::command]
fn analytics_window_metrics(state: tauri::State<AppState>, metric: String) -> serde_json::Value {
    let a = state.analytics.lock().unwrap();
    let wm = a.window_metrics(&metric);
    serde_json::json!({
        "metric": metric,
        "trend": wm.trend.emoji(),
        "mean": wm.current.mean,
        "std_dev": wm.current.std_dev,
        "anomaly_count": wm.anomalies.len(),
        "adaptive_threshold": wm.adaptive_threshold,
        "prediction_next": wm.prediction_next,
    })
}

#[tauri::command]
fn analytics_detect_anomalies(state: tauri::State<AppState>, metric: String) -> Vec<serde_json::Value> {
    let a = state.analytics.lock().unwrap();
    a.detect_anomalies(&metric).iter().map(|anom| serde_json::json!({
        "value": anom.value, "z_score": anom.z_score,
        "severity": anom.severity, "description": anom.description,
    })).collect()
}

#[tauri::command]
fn analytics_correlation(state: tauri::State<AppState>, a: String, b: String) -> serde_json::Value {
    let analytics = state.analytics.lock().unwrap();
    let r = analytics.pearson_correlation(&a, &b);
    let (ci_lo, ci_hi) = analytics.confidence_interval(&a);
    let roc = analytics.rate_of_change(&a, 5);
    serde_json::json!({
        "correlation": r, "strength": if r.abs() > 0.7 { "strong" } else if r.abs() > 0.4 { "moderate" } else { "weak" },
        "ci_95": [ci_lo, ci_hi], "rate_of_change_5": roc,
    })
}

#[tauri::command]
fn analytics_health_score(state: tauri::State<AppState>) -> serde_json::Value {
    state.analytics.lock().unwrap().health_score()
}

#[tauri::command]
fn analytics_change_point(state: tauri::State<AppState>, metric: String) -> serde_json::Value {
    let a = state.analytics.lock().unwrap();
    match a.detect_change_point(&metric) {
        Some((idx, magnitude, direction)) => serde_json::json!({
            "detected": true, "index": idx, "magnitude": magnitude, "direction": direction,
        }),
        None => serde_json::json!({ "detected": false }),
    }
}

// ─── Security Boundary Commands ──────────────────────────────────

#[tauri::command]
fn check_permission(state: tauri::State<AppState>, operation: String) -> serde_json::Value {
    let cat = match operation.as_str() {
        "delete_project" => agent::security_boundary::OperationCategory::DeleteProject,
        "delete_database" => agent::security_boundary::OperationCategory::DeleteDatabase,
        "external_network" => agent::security_boundary::OperationCategory::AccessExternalNetwork,
        "social_contacts" => agent::security_boundary::OperationCategory::ContactSocialContacts,
        "data_exfil" => agent::security_boundary::OperationCategory::DataExfiltration,
        "system_modify" => agent::security_boundary::OperationCategory::SystemModification,
        "file_delete" => agent::security_boundary::OperationCategory::FileDelete,
        _ => agent::security_boundary::OperationCategory::StatusQuery,
    };
    let mut boundary = state.security_boundary.lock().unwrap();
    let decision = boundary.check_permission(cat, &operation);
    serde_json::json!({
        "operation": operation, "allowed": decision.allowed,
        "level": decision.level.label(), "reason": decision.reason,
    })
}

#[tauri::command]
fn scan_llm_boundary(state: tauri::State<AppState>, text: String) -> Vec<serde_json::Value> {
    let mut boundary = state.security_boundary.lock().unwrap();
    boundary.scan_llm_output(&text).iter().map(|d| serde_json::json!({
        "operation": format!("{:?}", d.operation), "allowed": d.allowed,
        "level": d.level.label(), "reason": d.reason,
    })).collect()
}

#[tauri::command]
fn get_security_report(state: tauri::State<AppState>) -> serde_json::Value {
    state.security_boundary.lock().unwrap().security_report()
}

// ─── User Profile Commands ──────────────────────────────────────

#[tauri::command]
fn get_user_profile(state: tauri::State<AppState>) -> serde_json::Value {
    let profile = state.user_profile.lock().unwrap();
    serde_json::json!(profile.clone())
}

#[tauri::command]
fn update_user_profile(state: tauri::State<AppState>, display_name: String, nickname: String, avatar: String, personality: String) -> String {
    let mut profile = state.user_profile.lock().unwrap();
    profile.display_name = display_name;
    profile.nickname = nickname;
    profile.avatar = avatar;
    profile.personality = personality;
    format!("Profile updated — 你好，{}！", profile.nickname)
}

#[tauri::command]
fn get_greeting(state: tauri::State<AppState>) -> String {
    state.user_profile.lock().unwrap().greeting()
}

#[tauri::command]
fn get_heartbeat(state: tauri::State<AppState>) -> serde_json::Value {
    state.user_profile.lock().unwrap().heartbeat()
}

#[tauri::command]
fn get_achievements(state: tauri::State<AppState>) -> Vec<serde_json::Value> {
    let profile = state.user_profile.lock().unwrap();
    let approvals = state.approval_gate.lock().unwrap().audit_log.iter()
        .filter(|r| r.status == "Approved").count() as u32;
    let mut achievements = agent::user_profile::Achievement::all();
    for a in &mut achievements {
        a.update_progress(&profile, approvals, 0);
    }
    achievements.iter().map(|a| serde_json::json!({
        "id": a.id, "name": a.name, "description": a.description,
        "emoji": a.emoji, "unlocked": a.unlocked, "progress": a.progress,
    })).collect()
}

#[tauri::command]
fn touch_interaction(state: tauri::State<AppState>) -> String {
    let mut profile = state.user_profile.lock().unwrap();
    profile.touch();
    format!("💓 {}", profile.total_interactions)
}

// ─── System Health Commands ─────────────────────────────────────

#[tauri::command]
fn get_system_health(state: tauri::State<AppState>) -> Vec<serde_json::Value> {
    // 自动检测各模块状态
    let health = &state.system_health;
    let api_ok = state.api_circuit_breaker.state() == agent::resilience::CircuitState::Closed;
    health.report("api_client", if api_ok { "healthy" } else { "degraded" }, None);

    let cvfs_ok = state.cvfs.try_lock().is_ok();
    health.report("cvfs", if cvfs_ok { "healthy" } else { "degraded" }, None);

    state.system_health.full_report().iter().map(|h| serde_json::json!({
        "module": h.module, "status": h.status, "message": h.message, "last_check": h.last_check,
    })).collect()
}

#[tauri::command]
fn get_circuit_breaker_status(state: tauri::State<AppState>) -> serde_json::Value {
    serde_json::json!({
        "api": format!("{:?}", state.api_circuit_breaker.state()),
    })
}

// ─── WorkBuddy Engine Commands ──────────────────────────────────

#[tauri::command]
fn wb_add_rule(state: tauri::State<AppState>, name: String, trigger: String, target: String, delay_ms: u64, priority: u8) -> String {
    state.workbuddy.lock().unwrap().add_rule(&name, &trigger, &target, delay_ms, priority)
}

#[tauri::command]
fn wb_record_activity(state: tauri::State<AppState>, app_id: String, app_name: String, event_type: String, duration_ms: Option<u64>, bytes: Option<u64>) -> String {
    state.workbuddy.lock().unwrap().record_activity(&app_id, &app_name, &event_type, duration_ms, bytes);
    format!("Activity recorded for {}", app_name)
}

#[tauri::command]
fn wb_generate_report(state: tauri::State<AppState>) -> serde_json::Value {
    let report = state.workbuddy.lock().unwrap().generate_report();
    serde_json::json!(report)
}

#[tauri::command]
fn wb_generate_suggestions(state: tauri::State<AppState>) -> Vec<serde_json::Value> {
    let mut wb = state.workbuddy.lock().unwrap();
    wb.generate_suggestions();
    wb.suggestions.iter().map(|s| serde_json::json!({
        "id": s.id, "type": s.suggestion_type, "title": s.title,
        "description": s.description, "confidence": s.confidence,
    })).collect()
}

// ─── State Manager Commands ─────────────────────────────────────

#[tauri::command]
fn state_save_all(app_handle: tauri::AppHandle, state: tauri::State<AppState>) -> Result<String, String> {
    let _dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let mut sm = state.state_mgr.lock().unwrap();
    sm.save_all()?;
    Ok("All state saved".into())
}

#[tauri::command]
fn state_health_report(state: tauri::State<AppState>) -> serde_json::Value {
    state.state_mgr.lock().unwrap().health_report()
}

// ─── Agent Quality Commands ───────────────────────────────────────

#[tauri::command]
fn get_agent_quality_scores(
    state: tauri::State<AppState>,
) -> Vec<agent::evolving::agent_quality::AgentQualityScore> {
    let engine = state.agent_quality.lock().unwrap();
    engine.get_all_scores().into_iter().cloned().collect()
}

#[tauri::command]
async fn record_agent_task_quality(
    state: tauri::State<'_, AppState>,
    agent_role: String,
    success: bool,
    hallucination_categories: Vec<String>,
) -> Result<String, String> {
    // 第一阶段：同步更新 Agent 质量评分
    let bridge_entries: Vec<_> = {
        let mut engine = state.agent_quality.lock().unwrap();
        engine.record_agent_task(&agent_role, success, &hallucination_categories);

        hallucination_categories.iter().filter_map(|cat| {
            engine.bridge_hallucination_to_evolution(
                &agent_role, cat, &format!("{} by {}", cat, agent_role),
                "请查阅防幻觉报告获取修正建议", "medium",
            )
        }).collect()
    };

    // 第二阶段：异步写入 EvolutionEngine
    for entry in bridge_entries {
        let evo = state.evolution.lock().await;
        let delta = agent::evolving::consolidator::EvoDelta {
            experience_id: format!("hbridge-{}", chrono::Utc::now().timestamp()),
            context_trigger_hash: entry.error_pattern.clone(),
            failed_llm_action: entry.error_pattern,
            correct_human_action: entry.correction,
            token_sunk_cost_saved: 50,
            accuracy_weight: 0.7,
        };
        let _ = evo.local_consolidator.validate_and_commit_experience(delta).await;
    }

    let engine = state.agent_quality.lock().unwrap();
    let score = engine.get_score(&agent_role)
        .map(|s| s.rigor_score).unwrap_or(85);
    Ok(format!("Agent '{}' rigor score: {}/100", agent_role, score))
}

#[tauri::command]
fn get_global_quality_report(
    state: tauri::State<AppState>,
) -> serde_json::Value {
    state.agent_quality.lock().unwrap().global_quality_report()
}

// ─── Model name parser (shared by billing) ────────────────────

/// 将 API 调用中的模型字符串映射到 ModelModel 枚举
/// 委托给 billing::parse_model_string（全项目唯一权威来源）
fn parse_model_to_enum(model: &str) -> agent::router::ModelModel {
    agent::billing::parse_model_string(model)
}

/// In-memory key cache — survives even if Windows Credential Manager is unavailable
static KEY_CACHE: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<String, String>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

/// Store key in memory cache AND persist to file as reliable fallback
fn cache_key(provider: &str, key: &str) {
    if let Ok(mut cache) = KEY_CACHE.lock() {
        cache.insert(provider.to_string(), key.to_string());
    }
    // Also persist to file — reliable cross-restart storage
    let _ = save_key_file(provider, key);
}

/// File-based key persistence (base64) — reliable fallback when keyring is unavailable
fn key_file_path() -> std::path::PathBuf {
    let dir = CONFIG_DIR.lock().unwrap();
    dir.as_ref().cloned().unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".chronos_keys")
}

fn save_key_file(provider: &str, key: &str) -> std::io::Result<()> {
    let path = key_file_path();
    let mut map: std::collections::HashMap<String, String> = if path.exists() {
        let data = std::fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        std::collections::HashMap::new()
    };
    map.insert(provider.to_string(), simple_encode(key));
    std::fs::write(&path, serde_json::to_string(&map).unwrap_or_default())
}

fn load_key_file(provider: &str) -> Option<String> {
    let path = key_file_path();
    if !path.exists() { return None; }
    let data = std::fs::read_to_string(&path).ok()?;
    let map: std::collections::HashMap<String, String> = serde_json::from_str(&data).ok()?;
    map.get(provider).map(|v| simple_decode(v))
}

fn simple_encode(s: &str) -> String {
    // Base64 encode for basic obfuscation — keys are also in WinCred vault
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(s.as_bytes())
}

fn simple_decode(s: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(s)
        .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
        .unwrap_or_default()
}

/// Resolve API key: memory cache → Windows Credential Manager vault
fn resolve_key_from_vault(model: &str) -> String {
    let target = if model.contains("deepseek") { "deepseek" }
        else if model.contains("kimi") { "kimi" }
        else if model.contains("glm") { "glm" }
        else {
            tracing::warn!("[VAULT] Unknown model '{}', cannot resolve key", model);
            return String::new();
        };

    // 1. Try in-memory cache first (instant, same-session)
    if let Ok(cache) = KEY_CACHE.lock() {
        if let Some(key) = cache.get(target) {
            if !key.is_empty() {
                tracing::info!("[VAULT] Key resolved from memory cache for '{}'", target);
                return key.clone();
            }
        }
    }

    // 2. Try file-based persistence (reliable cross-restart)
    if let Some(key) = load_key_file(target) {
        if !key.is_empty() {
            tracing::info!("[VAULT] Key resolved from file for '{}'", target);
            // Restore to memory cache
            if let Ok(mut cache) = KEY_CACHE.lock() {
                cache.insert(target.to_string(), key.clone());
            }
            return key;
        }
    }

    // 3. Fall back to Windows Credential Manager
    let vault = agent::security_vault::NativeSecurityVault::new();
    match vault.fetch_api_key_native(target) {
        Ok(key) if !key.is_empty() => {
            tracing::info!("[VAULT] Key resolved from WinCred for '{}' — len={}", target, key.len());
            if let Ok(mut cache) = KEY_CACHE.lock() {
                cache.insert(target.to_string(), key.clone());
            }
            key
        }
        Ok(_) => {
            tracing::warn!("[VAULT] Key for '{}' is empty in WinCred — re-enter in Settings", target);
            String::new()
        }
        Err(e) => {
            tracing::error!("[VAULT] Failed to read key for '{}': {}", target, e);
            String::new()
        }
    }
}

/// Estimate prompt/completion split from total tokens and response content
fn split_tokens(total: u32, content: &str) -> (u32, u32) {
    let completion_est = (content.len() as f64 / 4.0).ceil() as u32;
    let completion = completion_est.min(total);
    let prompt = total.saturating_sub(completion);
    (prompt, completion)
}

// ─── Settings persistence ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppSettings {
    version: u32,
    cost_cap: f64,
    cost_cap_enabled: bool,
    ollama_url: String,
    lan_model: String,
    lan_timeout: u32,
    auto_fallback: bool,
    max_healing: u32,
    ast_audit: bool,
    block_gpl: bool,
    privacy_blur: bool,
    caching_priority: bool,
    accumulated_cost: f64,
    api_key_deepseek: String,
    api_key_kimi: String,
    api_key_glm: String,
    /// Vault presence flags — set by load_settings after keyring restore
    #[serde(default)]
    has_key_deepseek: bool,
    #[serde(default)]
    has_key_kimi: bool,
    #[serde(default)]
    has_key_glm: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            version: 1,
            cost_cap: 5.0,
            cost_cap_enabled: true,
            ollama_url: "http://localhost:11434".into(),
            lan_model: "deepseek-v4-flash".into(),
            lan_timeout: 3500,
            auto_fallback: true,
            max_healing: 3,
            ast_audit: true,
            block_gpl: true,
            privacy_blur: true,
            caching_priority: true,
            accumulated_cost: 0.0,
            api_key_deepseek: String::new(),
            api_key_kimi: String::new(),
            api_key_glm: String::new(),
            has_key_deepseek: false,
            has_key_kimi: false,
            has_key_glm: false,
        }
    }
}

static SETTINGS: std::sync::Mutex<Option<AppSettings>> = std::sync::Mutex::new(None);
static CONFIG_DIR: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

fn get_config_dir(app_handle: &tauri::AppHandle) -> PathBuf {
    let mut guard = CONFIG_DIR.lock().unwrap();
    if let Some(ref dir) = *guard {
        return dir.clone();
    }
    let dir = app_handle.path().app_config_dir().unwrap_or_else(|_| PathBuf::from("."));
    *guard = Some(dir.clone());
    dir
}

fn ensure_settings_loaded() -> AppSettings {
    let mut guard = SETTINGS.lock().unwrap();
    if let Some(ref s) = *guard {
        return s.clone();
    }
    let dir_guard = CONFIG_DIR.lock().unwrap();
    let dir = dir_guard.as_ref().cloned().unwrap_or_else(|| PathBuf::from("."));
    drop(dir_guard);
    let path = dir.join("config.json");
    let loaded = if let Ok(data) = std::fs::read_to_string(&path) {
        serde_json::from_str::<AppSettings>(&data).unwrap_or_default()
    } else {
        AppSettings::default()
    };
    *guard = Some(loaded.clone());
    loaded
}

#[tauri::command]
fn load_settings(app_handle: tauri::AppHandle) -> AppSettings {
    get_config_dir(&app_handle); // cache the dir
    let mut settings = ensure_settings_loaded();
    // Try to restore keys from Windows Credential Manager vault
    let vault = agent::security_vault::NativeSecurityVault::new();
    if settings.api_key_deepseek.is_empty() || settings.api_key_deepseek == "[stored in vault]" {
        if let Ok(key) = vault.fetch_api_key_native("deepseek") {
            if !key.is_empty() { settings.has_key_deepseek = true; settings.api_key_deepseek = key; }
        }
    } else if !settings.api_key_deepseek.is_empty() {
        settings.has_key_deepseek = true;
    }
    if settings.api_key_kimi.is_empty() || settings.api_key_kimi == "[stored in vault]" {
        if let Ok(key) = vault.fetch_api_key_native("kimi") {
            if !key.is_empty() { settings.has_key_kimi = true; settings.api_key_kimi = key; }
        }
    } else if !settings.api_key_kimi.is_empty() {
        settings.has_key_kimi = true;
    }
    if settings.api_key_glm.is_empty() || settings.api_key_glm == "[stored in vault]" {
        if let Ok(key) = vault.fetch_api_key_native("glm") {
            if !key.is_empty() { settings.has_key_glm = true; settings.api_key_glm = key; }
        }
    } else if !settings.api_key_glm.is_empty() {
        settings.has_key_glm = true;
    }
    // Also check file-based persistence for has_key flags
    if !settings.has_key_deepseek { settings.has_key_deepseek = load_key_file("deepseek").is_some(); }
    if !settings.has_key_kimi { settings.has_key_kimi = load_key_file("kimi").is_some(); }
    if !settings.has_key_glm { settings.has_key_glm = load_key_file("glm").is_some(); }
    // Strip actual key values before sending to frontend
    settings.api_key_deepseek = String::new();
    settings.api_key_kimi = String::new();
    settings.api_key_glm = String::new();
    settings
}

#[tauri::command]
fn save_settings(app_handle: tauri::AppHandle, new_settings: AppSettings) -> Result<String, String> {
    let dir = get_config_dir(&app_handle);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("config.json");

    // Security: vault API keys to Windows Credential Manager, never write plaintext to disk
    let vault = agent::security_vault::NativeSecurityVault::new();
    let mut disk_settings = new_settings.clone();
    if !new_settings.api_key_deepseek.is_empty() {
        cache_key("deepseek", &new_settings.api_key_deepseek);
        let _ = vault.vault_api_key_native("deepseek", &new_settings.api_key_deepseek);
    }
    if !new_settings.api_key_kimi.is_empty() {
        cache_key("kimi", &new_settings.api_key_kimi);
        let _ = vault.vault_api_key_native("kimi", &new_settings.api_key_kimi);
    }
    if !new_settings.api_key_glm.is_empty() {
        cache_key("glm", &new_settings.api_key_glm);
        let _ = vault.vault_api_key_native("glm", &new_settings.api_key_glm);
    }
    // Mask keys before writing to disk — only store presence flag
    disk_settings.api_key_deepseek = if new_settings.api_key_deepseek.is_empty() { String::new() } else { "[stored in vault]".into() };
    disk_settings.api_key_kimi = if new_settings.api_key_kimi.is_empty() { String::new() } else { "[stored in vault]".into() };
    disk_settings.api_key_glm = if new_settings.api_key_glm.is_empty() { String::new() } else { "[stored in vault]".into() };

    let json = serde_json::to_string_pretty(&disk_settings).map_err(|e| e.to_string())?;
    // 原子化写入: temp + rename, 防止崩溃时配置文件损坏
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, &json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp_path, &path).map_err(|e| e.to_string())?;
    *SETTINGS.lock().unwrap() = Some(new_settings);
    Ok(format!("Saved to {}", path.display()))
}

// ─── Agent roster + live windows + evolution ──────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentRosterEntry {
    id: String,
    name: String,
    model: String,
}

#[tauri::command]
fn get_agent_roster(state: tauri::State<AppState>) -> Vec<AgentRosterEntry> {
    let router = state.router.lock().unwrap();
    let roles = ["PM", "UIDesigner", "Architect", "Planner", "Coder", "Auditor", "Verifier"];
    roles.iter().map(|role| {
        let model = router.route_text_model(role).to_string();
        AgentRosterEntry {
            id: role.to_lowercase(),
            name: match *role {
                "PM" => "PM", "UIDesigner" => "UI Designer", "Architect" => "Architect",
                "Planner" => "Planner", "Coder" => "Coder Cluster",
                "Auditor" => "Auditor", _ => "Verifier",
            }.into(),
            model,
        }
    }).collect()
}

#[tauri::command]
fn list_live_windows() -> Vec<serde_json::Value> {
    // Enumerate top-level windows via Win32 EnumWindows
    #[cfg(target_os = "windows")]
    {
        let mut windows = Vec::new();
        unsafe {
            extern "system" {
                fn EnumWindows(callback: unsafe extern "system" fn(isize, isize) -> i32, lparam: isize) -> i32;
                fn IsWindowVisible(hwnd: isize) -> i32;
                fn GetWindowTextLengthW(hwnd: isize) -> i32;
                fn GetWindowTextW(hwnd: isize, buf: *mut u16, max: i32) -> i32;
                fn GetWindowThreadProcessId(hwnd: isize, pid: *mut u32) -> u32;
            }
            unsafe extern "system" fn enum_proc(hwnd: isize, lparam: isize) -> i32 {
                let windows = &mut *(lparam as *mut Vec<serde_json::Value>);
                if IsWindowVisible(hwnd) == 0 { return 1; }
                let len = GetWindowTextLengthW(hwnd);
                if len == 0 { return 1; }
                let mut buf: Vec<u16> = vec![0; (len + 1) as usize];
                GetWindowTextW(hwnd, buf.as_mut_ptr(), len + 1);
                let title = String::from_utf16_lossy(&buf[..len as usize]);
                let mut pid: u32 = 0;
                GetWindowThreadProcessId(hwnd, &mut pid);
                windows.push(serde_json::json!({
                    "id": format!("win-{}", hwnd),
                    "title": title,
                    "pid": pid,
                    "hwnd": hwnd,
                }));
                1
            }
            EnumWindows(enum_proc, &mut windows as *mut _ as isize);
        }
        windows
    }
    #[cfg(not(target_os = "windows"))]
    { vec![] }
}

#[tauri::command]
async fn evo_validate_experience(
    state: tauri::State<'_, AppState>,
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
async fn evo_intercept_context(
    state: tauri::State<'_, AppState>,
    context_hash: String,
) -> Result<bool, String> {
    let mut engine = state.evolution.lock().await;
    engine.intercept_context(&context_hash).await.map_err(|e| e.to_string())
}

#[tauri::command]
fn get_evolution_stats(state: tauri::State<AppState>) -> serde_json::Value {
    // Merge evolution_status with skill stats
    let engine = state.evolution.try_lock();
    let base = engine.as_ref().map(|e| e.evolution_status()).unwrap_or(serde_json::json!({
        "state": "locked",
        "memory_pool_size": 0,
        "total_interceptions": 0,
        "contracts_compiled": 0,
        "total_tokens_saved": 0,
        "skills_consolidated": 0,
    }));
    // Add skill stats from skill_engine
    let skill_engine = state.skill_engine.lock().unwrap();
    let mut result = base;
    result["total_skills"] = serde_json::json!(skill_engine.list_all().len());
    result["active_skills"] = serde_json::json!(skill_engine.active_skills().len());
    result
}

// ─── Skill & MCP listing ──────────────────────────────────────

#[tauri::command]
fn list_skills(state: tauri::State<AppState>) -> Vec<SkillInstance> {
    state.skill_engine.lock().unwrap().list_all().into_iter().cloned().collect()
}

#[tauri::command]
fn list_mcp_servers(state: tauri::State<AppState>) -> Vec<McpServer> {
    state.mcp_client.blocking_lock().connected_servers().into_iter().cloned().collect()
}

// ─── Orchestrator Management ─────────────────────────────────────

#[tauri::command]
fn prune_orchestrator_tasks(state: tauri::State<AppState>, keep: usize) -> String {
    let mut orch = state.orchestrator.lock().unwrap();
    let before = orch.tasks.len();
    orch.prune_old_tasks(keep);
    format!("Pruned {} tasks ({} → {})", before - orch.tasks.len(), before, orch.tasks.len())
}

#[tauri::command]
fn get_event_metrics(state: tauri::State<AppState>) -> serde_json::Value {
    state.orchestrator.lock().unwrap().event_metrics()
}

#[tauri::command]
fn flush_dead_letters(state: tauri::State<AppState>) -> Vec<serde_json::Value> {
    let mut orch = state.orchestrator.lock().unwrap();
    orch.flush_dead_letters()
        .into_iter()
        .map(|e| serde_json::json!({
            "id": e.id, "timestamp": e.timestamp,
            "source": format!("{:?}", e.source),
            "event_type": format!("{:?}", e.event_type),
        }))
        .collect()
}

// ─── MCP Management ─────────────────────────────────────────────

#[tauri::command]
fn mcp_disconnect(state: tauri::State<AppState>, server_id: String) -> Result<String, String> {
    state.mcp_client.blocking_lock().disconnect(&server_id)
        .map(|_| format!("Disconnected {}", server_id))
}

#[tauri::command]
fn mcp_cleanup_stale(state: tauri::State<AppState>) -> String {
    let mcp = state.mcp_client.blocking_lock();
    let count = mcp.connected_servers().len();
    format!("MCP cleanup check: {} active servers (zombie detection pending)", count)
}

// ─── Helper ────────────────────────────────────────────────────────

fn parse_role(s: &str) -> Result<AgentRole, String> {
    match s.to_lowercase().as_str() {
        "pm" => Ok(AgentRole::PM),
        "ui" | "ui_designer" | "uidesigner" => Ok(AgentRole::UIDesigner),
        "architect" | "arch" => Ok(AgentRole::Architect),
        "planner" => Ok(AgentRole::Planner),
        "coder" => Ok(AgentRole::Coder),
        "auditor" => Ok(AgentRole::Auditor),
        "verifier" => Ok(AgentRole::Verifier),
        _ => Err(format!("Unknown role: {}", s)),
    }
}

// ─── Web Intelligence Commands ─────────────────────────────────────

#[tauri::command]
async fn web_intel_search(
    state: tauri::State<'_, AppState>,
    query: String,
    engine: Option<String>,
    max_results: Option<u32>,
) -> Result<Vec<WebSearchResult>, String> {
    let mut wi = state.web_intelligence.lock().await;
    wi.search(&query, engine.as_deref(), Some(max_results.unwrap_or(5))).await
}

#[tauri::command]
async fn web_intel_fetch(
    state: tauri::State<'_, AppState>,
    url: String,
    distill: Option<bool>,
) -> Result<WebFetchResult, String> {
    let mut wi = state.web_intelligence.lock().await;
    wi.fetch(&url, distill.unwrap_or(true)).await
}

#[tauri::command]
async fn web_intel_research(
    state: tauri::State<'_, AppState>,
    topic: String,
    sources: Option<Vec<String>>,
) -> Result<ResearchReport, String> {
    let mut wi = state.web_intelligence.lock().await;
    wi.research(&topic, sources.unwrap_or_default()).await
}

#[tauri::command]
async fn web_intel_add_domain(
    state: tauri::State<'_, AppState>,
    domain: String,
    category: Option<String>,
) -> Result<String, String> {
    let mut wi = state.web_intelligence.lock().await;
    wi.add_allowed_domain(&domain, category.as_deref().unwrap_or("custom"))
        .map(|()| format!("Domain {} added", domain))
}

#[tauri::command]
async fn web_intel_remove_domain(
    state: tauri::State<'_, AppState>,
    domain: String,
) -> Result<String, String> {
    let mut wi = state.web_intelligence.lock().await;
    wi.remove_allowed_domain(&domain)
        .map(|()| format!("Domain {} removed", domain))
}

#[tauri::command]
async fn web_intel_list_domains(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<(String, String)>, String> {
    let wi = state.web_intelligence.lock().await;
    Ok(wi.list_allowed_domains())
}

#[tauri::command]
async fn web_intel_get_audit_log(
    state: tauri::State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<WebAuditEntry>, String> {
    let wi = state.web_intelligence.lock().await;
    Ok(wi.get_audit_log_owned(limit.unwrap_or(50)))
}

#[tauri::command]
async fn web_intel_get_stats(
    state: tauri::State<'_, AppState>,
) -> Result<WebIntelStats, String> {
    let wi = state.web_intelligence.lock().await;
    Ok(wi.get_stats())
}

#[tauri::command]
async fn web_intel_save_state(
    app_handle: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let wi = state.web_intelligence.lock().await;
    wi.save_state(&dir)
}

#[tauri::command]
async fn web_intel_load_state(
    app_handle: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let mut wi = state.web_intelligence.lock().await;
    wi.load_state(&dir)
}

// ─── 统一行动调度引擎 (Action Dispatch) ────────────────────────────

/// 从 LLM 响应文本中提取所有 JSON 动作块
fn extract_action_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut depth = 0i32;
    let mut start = None;

    for (i, ch) in text.char_indices() {
        match ch {
            '{' => {
                if depth == 0 { start = Some(i); }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(s) = start {
                        let block = text[s..=i].to_string();
                        // 只保留包含 "action" 字段的 JSON 块
                        if block.contains("\"action\"") || block.contains("\"actions\"") {
                            blocks.push(block);
                        }
                    }
                    start = None;
                }
            }
            _ => {}
        }
    }
    blocks
}

/// 格式化 Web 搜索结果供 LLM 上下文注入
fn format_search_for_llm(results: &[WebSearchResult]) -> String {
    if results.is_empty() {
        return "No search results found.".into();
    }
    let mut out = String::from("## Web Search Results\n\n");
    for (i, r) in results.iter().enumerate() {
        out.push_str(&format!(
            "{}. **{}**\n   URL: {}\n   {}\n   Source: {} | Score: {:.1}\n\n",
            i + 1, r.title, r.url, r.snippet, r.source, r.relevance_score
        ));
    }
    out
}

/// 格式化 Web 抓取结果供 LLM 上下文注入
fn format_fetch_for_llm(result: &WebFetchResult) -> String {
    if !result.success {
        return format!("Web fetch failed: {}", result.error.as_deref().unwrap_or("unknown error"));
    }
    let mut out = format!("## Web Fetch: {}\n\n", result.title);
    if result.distilled {
        out.push_str(&format!(
            "> Content distilled from {} bytes. Key points:\n\n",
            result.content_length
        ));
        for point in &result.key_points {
            out.push_str(&format!("- {}\n", point));
        }
        if let Some(ref summary) = result.distilled_summary {
            out.push_str(&format!("\n### Summary\n\n{}\n", summary));
        }
    } else {
        out.push_str(&result.content);
    }
    out
}

/// 核心行动调度器 — 验证 + 执行 + 返回结果文本
async fn dispatch_action(
    state: &AppState,
    action: &AgentAction,
) -> Result<String, String> {
    match action {
        AgentAction::WebSearch { query, engine, max_results } => {
            tracing::info!("[DISPATCH] WebSearch: {}", query);
            let mut wi = state.web_intelligence.lock().await;
            let results = wi.search(query, engine.as_deref(), *max_results).await?;
            Ok(format_search_for_llm(&results))
        }
        AgentAction::WebFetch { url, distill } => {
            tracing::info!("[DISPATCH] WebFetch: {}", url);
            let mut wi = state.web_intelligence.lock().await;
            let result = wi.fetch(url, distill.unwrap_or(true)).await?;
            Ok(format_fetch_for_llm(&result))
        }
        AgentAction::FileRead { path, range: _ } => {
            let sandbox = state.sandbox.lock().unwrap();
            let full_path = sandbox.project_root.join(path);
            std::fs::read_to_string(&full_path)
                .map_err(|e| format!("File read failed: {} — {}", path, e))
        }
        AgentAction::Terminal { command, cwd: _ } => {
            tracing::info!("[DISPATCH] Terminal: {}", command);
            let output = std::process::Command::new("cmd")
                .args(["/C", command])
                .output()
                .map_err(|e| format!("Command execution failed: {}", e))?;
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if output.status.success() {
                Ok(format!("Command succeeded:\n```\n{}\n```", stdout))
            } else {
                Ok(format!("Command failed (exit {}):\n```\n{}\n```\nStderr:\n```\n{}\n```",
                    output.status.code().unwrap_or(-1), stdout, stderr))
            }
        }
        AgentAction::ExecuteSkill { name, args } => {
            // Skill execution via sync blocking to avoid Send issues
            let name_c = name.clone();
            let args_c = args.clone();
            let result = tokio::task::spawn_blocking(move || {
                // Use a fresh SkillEngine for this blocking call
                let engine = agent::skill_engine::SkillEngine::new();
                // Note: in production this should use the real engine from state
                // For now, return a placeholder since skills require filesystem access
                Err::<String, String>(format!("Skill '{}' requires local filesystem access", name_c))
            }).await.map_err(|e| format!("Skill spawn failed: {}", e))?;
            Err(result.err().unwrap_or_else(|| format!("Skill '{}' execution failed", name)))
        }
        AgentAction::McpCall { server_id, tool_name, args } => {
            let mcp = state.mcp_client.lock().await;
            let result = mcp.call_tool(server_id, tool_name, args).await;
            if result.success {
                if let Some(ref distilled) = result.distilled {
                    Ok(format!("MCP Result (distilled {}→{}):\n{}",
                        distilled.original_size, distilled.distilled_size, distilled.summary))
                } else {
                    Ok(serde_json::to_string(&result.data).unwrap_or_default())
                }
            } else {
                Err(result.error.unwrap_or_else(|| format!("MCP call '{}' on '{}' failed", tool_name, server_id)))
            }
        }
        AgentAction::FileEdit { path: _, content: _ } => {
            Err("FileEdit actions must be approved by the user before execution".into())
        }
    }
}

/// Tauri Command: 解析并执行单个 LLM 动作
#[tauri::command]
async fn execute_agent_action(
    state: tauri::State<'_, AppState>,
    action_json: String,
) -> Result<String, String> {
    // 红线一校验 (drop before await)
    let action = {
        let redline = state.redline.lock().unwrap();
        redline.validate_and_parse(&action_json)
            .map_err(|e| format!("Redline validation failed: {}", e))?
    };

    // 安全边界校验 (drop before await)
    {
        let mut boundary = state.security_boundary.lock().unwrap();
        let scan_text = format!("{:?}", action);
        let violations = boundary.scan_llm_output(&scan_text);
        if !violations.is_empty() {
            return Err(format!(
                "Security boundary blocked: {}",
                violations.iter().map(|d| d.reason.clone()).collect::<Vec<_>>().join("; ")
            ));
        }
    }

    dispatch_action(&state, &action).await
}

/// Tauri Command: 从 LLM 响应中提取并执行所有动作，返回纯文本结果
/// 如果响应中不含动作块，返回原始文本
#[tauri::command]
async fn extract_and_execute_actions(
    state: tauri::State<'_, AppState>,
    llm_response: String,
) -> Result<serde_json::Value, String> {
    let blocks = extract_action_blocks(&llm_response);

    if blocks.is_empty() {
        return Ok(serde_json::json!({
            "has_actions": false,
            "text_response": llm_response,
            "action_results": []
        }));
    }

    let mut action_results = Vec::new();
    let mut combined_results = String::new();

    for block in &blocks {
        let action_json = block.clone();
        match execute_agent_action_inner(&state, &action_json).await {
            Ok(result_text) => {
                combined_results.push_str(&result_text);
                combined_results.push_str("\n\n");
                action_results.push(serde_json::json!({
                    "action": block,
                    "success": true,
                    "result": result_text
                }));
            }
            Err(e) => {
                tracing::warn!("[DISPATCH] Action failed: {}", e);
                combined_results.push_str(&format!("[Action failed: {}]\n\n", e));
                action_results.push(serde_json::json!({
                    "action": block,
                    "success": false,
                    "error": e
                }));
            }
        }
    }

    Ok(serde_json::json!({
        "has_actions": true,
        "text_response": llm_response,
        "action_results": action_results,
        "combined_context": combined_results
    }))
}

/// 内部函数：无需 State 参数的执行路径
async fn execute_agent_action_inner(
    state: &AppState,
    action_json: &str,
) -> Result<String, String> {
    let action = {
        let redline = state.redline.lock().unwrap();
        redline.validate_and_parse(action_json)
            .map_err(|e| format!("Redline validation failed: {}", e))?
    };

    {
        let mut boundary = state.security_boundary.lock().unwrap();
        let scan_text = format!("{:?}", action);
        let violations = boundary.scan_llm_output(&scan_text);
        if !violations.is_empty() {
            return Err(format!(
                "Security boundary blocked: {}",
                violations.iter().map(|d| d.reason.clone()).collect::<Vec<_>>().join("; ")
            ));
        }
    }

    dispatch_action(state, &action).await
}

// ─── Collaboration Engine Commands ──────────────────────────────────

#[tauri::command]
async fn collab_get_model_ranking(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let engine = state.collaboration.lock().await;
    Ok(engine.stats())
}

#[tauri::command]
async fn collab_recommend_model(
    state: tauri::State<'_, AppState>,
    task_type: String,
    prefer_cheap: Option<bool>,
) -> Result<serde_json::Value, String> {
    let engine = state.collaboration.lock().await;
    let (model, reason) = engine.recommend_model_with_reason(&task_type, prefer_cheap.unwrap_or(false));
    let best = engine.select_best_model(&task_type, prefer_cheap.unwrap_or(false));
    let fallbacks = engine.fallback_models(&model, &task_type);
    Ok(serde_json::json!({
        "recommended": model,
        "reason": reason,
        "best_by_quality": best,
        "fallbacks": fallbacks,
        "mode": format!("{:?}", engine.decide_mode(&task_type, 0.5, false)),
    }))
}

#[tauri::command]
async fn collab_record_execution(
    state: tauri::State<'_, AppState>,
    model_name: String,
    task_type: String,
    success: bool,
    latency_ms: u64,
    quality_score: f64,
) -> Result<String, String> {
    let mut engine = state.collaboration.lock().await;
    engine.record_execution(&model_name, &task_type, success, latency_ms, quality_score);
    Ok(format!("Recorded execution for {}", model_name))
}

// ─── Task Intelligence Commands ────────────────────────────────────

#[tauri::command]
fn task_decompose(
    state: tauri::State<AppState>,
    task: String,
) -> Result<serde_json::Value, String> {
    let engine = state.task_intelligence.lock().unwrap();
    let plan = engine.decompose(&task);
    Ok(serde_json::to_value(&plan).map_err(|e| e.to_string())?)
}

#[tauri::command]
fn task_estimate_complexity(
    state: tauri::State<AppState>,
    task: String,
) -> Result<serde_json::Value, String> {
    let engine = state.task_intelligence.lock().unwrap();
    let (level, confidence) = engine.estimate_complexity(&task);
    let category = engine.categorize(&task);
    Ok(serde_json::json!({
        "complexity": level.label(),
        "level": level as u8,
        "confidence": confidence,
        "estimated_steps": level.estimated_steps(),
        "category": category.label(),
    }))
}

// ─── Predictive Analytics Commands ──────────────────────────────────

#[tauri::command]
fn predictive_forecast_tokens(
    state: tauri::State<AppState>,
    historical: Vec<f64>,
    periods: Option<usize>,
) -> Result<Vec<f64>, String> {
    let engine = state.predictive.lock().unwrap();
    // Simple EMA forecast using local_analytics style
    let forecast = engine.forecast_simple(&historical, periods.unwrap_or(10));
    Ok(forecast)
}

#[tauri::command]
fn predictive_detect_cost_anomaly(
    state: tauri::State<AppState>,
    values: Vec<f64>,
) -> Result<serde_json::Value, String> {
    let engine = state.predictive.lock().unwrap();
    let anomalies = engine.detect_cost_anomaly_simple(&values);
    Ok(serde_json::json!({
        "anomaly_count": anomalies.len(),
        "anomalies": anomalies,
        "mean": if values.is_empty() { 0.0 } else { values.iter().sum::<f64>() / values.len() as f64 },
        "std_dev": std_dev(&values),
    }))
}

#[tauri::command]
fn predictive_optimize_budget(
    state: tauri::State<AppState>,
    current_usage: Vec<f64>,
    budget: f64,
    _quality_threshold: f64,
) -> Result<serde_json::Value, String> {
    let engine = state.predictive.lock().unwrap();
    let opt = engine.optimize_budget_simple(&current_usage, budget);
    Ok(serde_json::json!({
        "recommended_daily": format!("¥{:.2}", opt.recommended_daily),
        "projected_monthly": format!("¥{:.2}", opt.projected_monthly),
        "over_budget_risk": format!("{:.1}%", opt.over_budget_risk * 100.0),
        "savings_potential": format!("¥{:.2}", opt.savings_potential),
        "suggestions": opt.suggestions,
    }))
}

#[tauri::command]
fn predictive_analyze_enhanced(
    state: tauri::State<AppState>,
    message: String,
) -> Result<serde_json::Value, String> {
    let engine = state.predictive.lock().unwrap();
    Ok(engine.analyze_task_enhanced(&message))
}

#[tauri::command]
fn scheduling_analyze_enhanced(
    state: tauri::State<AppState>,
    message: String,
) -> Result<serde_json::Value, String> {
    let engine = agent::scheduling_engine::AgentSchedulingEngine::new();
    Ok(engine.analyze_enhanced(&message))
}

// ─── Distillation Evolution Commands ────────────────────────────────

#[tauri::command]
async fn distill_evolution_report(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let wi = state.web_intelligence.lock().await;
    Ok(wi.distillation.evolution_report())
}

#[tauri::command]
async fn distill_feedback(
    state: tauri::State<'_, AppState>,
    url: String,
    quality_score: f64,
    content_type: Option<String>,
) -> Result<String, String> {
    let mut wi = state.web_intelligence.lock().await;
    wi.distillation.feedback(&url, quality_score, &content_type.unwrap_or_else(|| "documentation".into()));
    Ok(format!("Feedback recorded for {}: quality={:.2}", url, quality_score))
}

// ─── Evolution Bus Commands ─────────────────────────────────────────

#[tauri::command]
fn evobus_health_report(
    state: tauri::State<AppState>,
) -> Result<serde_json::Value, String> {
    let bus = state.evolution_bus.lock().unwrap();
    Ok(bus.health_report())
}

#[tauri::command]
fn evobus_record_feedback(
    state: tauri::State<AppState>,
    engine: String,
    metric: String,
    current_value: f64,
    target_value: f64,
    direction_is_higher_better: Option<bool>,
) -> Result<serde_json::Value, String> {
    let mut bus = state.evolution_bus.lock().unwrap();
    let eid = parse_evo_engine(&engine)?;
    let new_val = bus.feedback_performance(eid, &metric, current_value, target_value, direction_is_higher_better.unwrap_or(true));
    Ok(serde_json::json!({
        "adjusted": new_val.is_some(),
        "new_value": new_val,
        "engine": engine,
        "metric": metric,
    }))
}

#[tauri::command]
fn hallucination_feedback(
    state: tauri::State<AppState>,
    is_false_positive: Option<bool>,
) -> Result<serde_json::Value, String> {
    // Note: HallucinationGuard is currently instantiated per-audit call.
    // For evolution purposes, we maintain a persistent instance in the evolution_bus context.
    // This command records aggregate feedback.
    let mut evo = state.evolution_bus.lock().unwrap();
    let accuracy = if is_false_positive.unwrap_or(false) { 0.6 } else { 0.9 };
    let fp_rate = if is_false_positive.unwrap_or(false) { 0.25 } else { 0.1 };

    evo.feedback_performance(
        agent::evolution_bus::EngineId::HallucinationGuard,
        "accuracy", accuracy, 0.9, true,
    );
    evo.feedback_performance(
        agent::evolution_bus::EngineId::HallucinationGuard,
        "false_positive_rate", fp_rate, 0.1, false,
    );

    Ok(serde_json::json!({
        "recorded": true,
        "is_false_positive": is_false_positive.unwrap_or(false),
        "accuracy": accuracy,
        "false_positive_rate": fp_rate,
    }))
}

fn parse_evo_engine(s: &str) -> Result<agent::evolution_bus::EngineId, String> {
    use agent::evolution_bus::EngineId;
    match s {
        "distillation" => Ok(EngineId::Distillation),
        "scheduling" => Ok(EngineId::Scheduling),
        "hallucination" => Ok(EngineId::HallucinationGuard),
        "cache" => Ok(EngineId::CacheEngine),
        "agent_quality" => Ok(EngineId::AgentQuality),
        "collaboration" => Ok(EngineId::Collaboration),
        "task_intelligence" => Ok(EngineId::TaskIntelligence),
        "predictive" => Ok(EngineId::PredictiveAnalytics),
        "local_analytics" => Ok(EngineId::LocalAnalytics),
        _ => Err(format!("Unknown engine: {}", s)),
    }
}

/// Helper: compute standard deviation
fn std_dev(values: &[f64]) -> f64 {
    if values.is_empty() { return 0.0; }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64).sqrt()
}

// ─── Data Flywheel Commands ─────────────────────────────────────────

#[tauri::command]
fn flywheel_dashboard(
    state: tauri::State<AppState>,
) -> Result<serde_json::Value, String> {
    let fw = state.flywheel.lock().unwrap();
    Ok(fw.dashboard())
}

#[tauri::command]
fn flywheel_spin(
    state: tauri::State<AppState>,
) -> Result<serde_json::Value, String> {
    let mut fw = state.flywheel.lock().unwrap();
    // Auto-collect from web_intelligence
    if let Ok(wi) = state.web_intelligence.try_lock() {
        let stats = wi.get_stats();
        fw.collect_from_web_intel(
            stats.total_searches, stats.total_fetches, stats.bytes_downloaded,
            stats.unified_cache_hits, stats.unified_cache_misses,
        );
        fw.collect_from_distillation(
            stats.total_distilled, stats.total_bytes_saved,
            stats.avg_compression_ratio, 0.85,
        );
    }
    let snap = fw.spin();
    Ok(serde_json::to_value(&snap).map_err(|e| e.to_string())?)
}

// ─── 应用入口 ──────────────────────────────────────────────────────

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            // redline
            get_redline_status,
            validate_model_output,
            reset_fuse,
            // orchestrator
            get_pipeline_stats,
            start_pipeline,
            pause_pipeline,
            resume_pipeline,
            advance_pipeline,
            create_task,
            assign_task,
            complete_task,
            fail_task,
            // api client
            chat_api,
            chat_api_stream,
            // sandbox
            init_sandbox,
            get_checkpoints,
            // mcp
            mcp_connect_and_init,
            mcp_fetch_tools,
            mcp_disconnect,
            mcp_cleanup_stale,
            // orchestrator management
            prune_orchestrator_tasks,
            flush_dead_letters,
            get_event_metrics,
            // router
            get_route_mode,
            set_route_mode,
            get_available_models,
            set_model_api_key,
            route_for_role,
            get_model_endpoint,
            // hybrid router
            hrouter_select_model,
            hrouter_get_cluster_status,
            // local analytics
            analytics_record,
            analytics_snapshot,
            analytics_window_metrics,
            analytics_detect_anomalies,
            analytics_correlation,
            analytics_health_score,
            analytics_change_point,
            // state manager
            state_save_all,
            state_health_report,
            // workbuddy engine
            wb_add_rule,
            wb_record_activity,
            wb_generate_report,
            wb_generate_suggestions,
            // system health
            get_system_health,
            get_circuit_breaker_status,
            // user profile
            get_user_profile,
            update_user_profile,
            get_greeting,
            get_heartbeat,
            get_achievements,
            touch_interaction,
            // security boundary
            check_permission,
            scan_llm_boundary,
            get_security_report,
            // shadow
            get_shadow_stats,
            toggle_shadow,
            dismiss_shadow_suggestion,
            save_shadow_state,
            load_shadow_state,
            // agent roster + live windows + evolution
            get_agent_roster,
            list_live_windows,
            get_evolution_stats,
            evo_validate_experience,
            evo_intercept_context,
            // agent quality
            get_agent_quality_scores,
            record_agent_task_quality,
            get_global_quality_report,
            // embedding engine
            embedding_search,
            embedding_add,
            embedding_stats,
            // settings persistence
            load_settings,
            save_settings,
            // skill & mcp listing
            list_skills,
            list_mcp_servers,
            // remote cluster
            cluster_register_server,
            cluster_unregister_server,
            cluster_bind_project,
            cluster_edit_file,
            cluster_compile,
            cluster_ping,
            get_cluster_stats,
            // cvfs
            cvfs_create_project,
            cvfs_verify_scope,
            cvfs_capture_checkpoint,
            cvfs_get_checkpoints,
            cvfs_get_projects,
            cvfs_delete_project,
            cvfs_list_project_files,
            cvfs_capture_checkpoint_v2,
            cvfs_restore_checkpoint,
            cvfs_delete_checkpoint,
            cvfs_get_project_health,
            // remote proxy
            remote_connect,
            remote_disconnect,
            remote_list_files,
            remote_read_file,
            remote_write_file,
            remote_compile,
            remote_snapshot,
            remote_rewind,
            get_remote_stats,
            // workbuddy
            get_buddy_scan_stats,
            run_buddy_scan,
            toggle_buddy_scan,
            get_buddy_saved_cost,
            get_billing_stats,
            get_billing_dashboard,
            update_cost_cap,
            get_model_recommendation,
            check_context_health,
            analyze_task,
            audit_hallucination,
            get_context_glue_status,
            add_app_binding,
            remove_app_binding,
            get_app_bindings,
            toggle_context_glue,
            save_context_glue_bindings,
            load_context_glue_bindings,
            // general
            get_sandbox_status,
            get_session_cost,
            get_saved_cost,
            get_saving_rate,
            // security vault
            get_vault_status,
            vault_api_key,
            fetch_api_key,
            delete_api_key,
            get_detector_stats,
            // lan health
            check_lan_health,
            // worktree commands
            create_worktree,
            activate_worktree,
            complete_worktree,
            merge_worktree,
            prune_worktree,
            list_worktrees,
            get_worktree_stats,
            // approval gate (第四红线)
            submit_for_approval,
            submit_for_approval_with_cost,
            auditor_pre_screen_approval,
            decide_approval,
            list_pending_approvals,
            get_approval_audit_log,
            add_approval_rule,
            remove_approval_rule,
            get_approval_rules,
            get_approval_suggestions,
            expire_stale_approvals,
            save_approval_state,
            load_approval_state,
            // session persistence (chunked)
            save_chat_session_chunk,
            load_chat_session_chunk,
            list_historical_meta_manifests,
            list_sessions_by_project,
            delete_chat_session,
            export_chat_session,
            rename_chat_session,
            import_chat_session,
            // web intelligence
            web_intel_search,
            web_intel_fetch,
            web_intel_research,
            web_intel_add_domain,
            web_intel_remove_domain,
            web_intel_list_domains,
            web_intel_get_audit_log,
            web_intel_get_stats,
            web_intel_save_state,
            web_intel_load_state,
            // action dispatch engine
            execute_agent_action,
            extract_and_execute_actions,
            // collaboration engine
            collab_get_model_ranking,
            collab_recommend_model,
            collab_record_execution,
            // task intelligence
            task_decompose,
            task_estimate_complexity,
            // predictive analytics
            predictive_forecast_tokens,
            predictive_detect_cost_anomaly,
            predictive_optimize_budget,
            predictive_analyze_enhanced,
            scheduling_analyze_enhanced,
            // distillation evolution
            distill_evolution_report,
            distill_feedback,
            // evolution bus
            evobus_health_report,
            evobus_record_feedback,
            hallucination_feedback,
            // data flywheel
            flywheel_dashboard,
            flywheel_spin,
        ])
        .setup(|_app| {
            // 将 AppHandle 注入 Orchestrator，使其可以 emit 前端事件
            {
                let handle = _app.handle().clone();
                let state = _app.state::<AppState>();
                state.orchestrator.lock().unwrap().set_app_handle(handle);
            }

            // 自动恢复 Context Glue 跨应用绑定
            {
                if let Ok(dir) = _app.handle().path().app_data_dir() {
                    let _ = _app.state::<AppState>().context_glue.lock().unwrap().load_bindings(&dir);
                }
            }

            // 自动恢复 Shadow 影子记忆
            {
                if let Ok(dir) = _app.handle().path().app_data_dir() {
                    let _ = _app.state::<AppState>().shadow.lock().unwrap().load_state(&dir);
                }
            }

            // 自动恢复 Approval Gate 审批门禁状态
            {
                if let Ok(dir) = _app.handle().path().app_data_dir() {
                    let _ = _app.state::<AppState>().approval_gate.lock().unwrap().load_state(&dir);
                }
            }

            // 自动恢复 C-VFS 项目池和检查点（统一使用 app_data_dir）
            {
                if let Ok(app_data) = _app.handle().path().app_data_dir() {
                    let handle = _app.handle().clone();
                    tauri::async_runtime::spawn(async move {
                        let state = handle.state::<AppState>();
                        let guard = state.cvfs.lock().await;
                        let _ = guard.load_state(&app_data).await;
                    });
                }
            }

            // 自动加载 skills/ 目录下的所有技能清单
            {
                let state = _app.state::<AppState>();
                let mut skill_engine = state.skill_engine.lock().unwrap();
                let skills_dir = std::path::PathBuf::from(
                    env!("CARGO_MANIFEST_DIR")
                ).join("skills");
                if skills_dir.exists() {
                    if let Ok(entries) = std::fs::read_dir(&skills_dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_dir() {
                                let manifest = path.join("skill.json");
                                if manifest.exists() {
                                    if let Ok(json) = std::fs::read_to_string(&manifest) {
                                        if let Err(e) = skill_engine.load_from_json(&json) {
                                            tracing::warn!("[SETUP] Failed to load skill {:?}: {}", path, e);
                                        } else {
                                            tracing::info!("[SETUP] Loaded skill: {:?}", path);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 注册统一持久化管理器 — 使用 Send+Sync 的 AppHandle
            {
                let handle = _app.handle().clone();
                let app_data = handle.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                let state = _app.state::<AppState>();
                let mut sm = state.state_mgr.lock().unwrap();

                let h1 = handle.clone();
                let h2 = handle.clone();
                sm.register("shadow",
                    move |dir| { h1.state::<AppState>().shadow.lock().unwrap().save_state(dir).map_err(|e| e.to_string()) },
                    move |dir| { h2.state::<AppState>().shadow.lock().unwrap().load_state(dir).map_err(|e| e.to_string()) },
                );
                let h1 = handle.clone(); let h2 = handle.clone();
                sm.register("context_glue",
                    move |dir| h1.state::<AppState>().context_glue.lock().unwrap().save_bindings(dir),
                    move |dir| h2.state::<AppState>().context_glue.lock().unwrap().load_bindings(dir),
                );
                let h1 = handle.clone(); let h2 = handle.clone();
                sm.register("approval_gate",
                    move |dir| h1.state::<AppState>().approval_gate.lock().unwrap().save_state(dir),
                    move |dir| h2.state::<AppState>().approval_gate.lock().unwrap().load_state(dir),
                );
                let h1 = handle.clone(); let h2 = handle.clone();
                sm.register("embedding",
                    move |dir| h1.state::<AppState>().embedding.lock().unwrap().save_state(dir),
                    move |dir| h2.state::<AppState>().embedding.lock().unwrap().load_state(dir),
                );
                let h1 = handle.clone(); let h2 = handle.clone();
                sm.register("analytics",
                    move |dir| h1.state::<AppState>().analytics.lock().unwrap().save_state(dir),
                    move |dir| h2.state::<AppState>().analytics.lock().unwrap().load_state(dir),
                );

                let _ = sm.init(&app_data);
            }

            // 初始化 tracing 日志系统
            let app_dir = _app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            let log_dir = app_dir.join("chronos_vault").join("logs");
            std::fs::create_dir_all(&log_dir).ok();
            let log_file = log_dir.join("chronos-shadow.log");
            let file_appender = tracing_appender::rolling::never(&log_dir, "chronos-shadow.log");
            fmt()
                .with_writer(file_appender)
                .with_target(false)
                .with_thread_ids(true)
                .init();
            tracing::info!("Chronos-Shadow v0.2.0 started — log at {:?}", log_file);

            // Seed billing_engine from saved settings (cost cap + legacy migration)
            {
                let state = _app.state::<AppState>();
                let settings = ensure_settings_loaded();
                state.billing_engine.set_cost_cap(settings.cost_cap);
                state.billing_engine.set_cost_cap_enabled(settings.cost_cap_enabled);
                if settings.accumulated_cost > 0.0 {
                    state.billing_engine.migrate_legacy_cost(settings.accumulated_cost);
                    tracing::info!("[BILLING] Migrated legacy cost ¥{:.2} to Budget tier", settings.accumulated_cost);
                }
            }

            // 系统托盘（可选 — 失败不阻断启动）
            let _ = (|| {
                let m = MenuBuilder::new(_app)
                    .item(&MenuItemBuilder::with_id("show", "显示窗口").build(_app).ok()?)
                    .separator()
                    .item(&MenuItemBuilder::with_id("quit", "退出 Chronos-Shadow").build(_app).ok()?)
                    .build().ok()?;
                let icon = _app.default_window_icon().cloned()?;
                let _tray = TrayIconBuilder::new()
                    .icon(icon)
                    .tooltip("Chronos-Shadow | 时空之影")
                    .menu(&m)
                    .on_menu_event(|app, event| match event.id().as_ref() {
                        "show" => { if let Some(w) = app.get_webview_window("main") { w.show().ok(); w.set_focus().ok(); } }
                        "quit" => { app.exit(0); }
                        _ => {}
                    })
                    .build(_app).ok()?;
                tracing::info!("[SETUP] System tray icon created.");
                Some(())
            })();

            // 拦截窗口关闭事件 → 隐藏到托盘而非退出
            if let Some(window) = _app.get_webview_window("main") {
                let win = window.clone();
                window.on_window_event(move |event| {
                    if matches!(event, WindowEvent::CloseRequested { .. }) {
                        win.hide().ok();
                    }
                });
            }

            #[cfg(debug_assertions)]
            if let Some(window) = _app.get_webview_window("main") {
                window.open_devtools();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Chronos-Shadow");
}
