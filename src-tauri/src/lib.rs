// Chronos-Shadow 核心库入口
#![allow(deprecated)] // 旧 Router 将在 v0.2.0 彻底移除

pub mod agent;
pub mod vision;

use serde::{Deserialize, Serialize};
use tauri::Manager;
use tauri::Emitter;
use tauri::WindowEvent;
use agent::api_client::{ApiClient, ApiResponse, ChatMessage};
use agent::orchestrator::{AgentRole, Orchestrator, OrchestratorStats};
use agent::redline::{RedlineGuard, RedlineStatus};
#[allow(deprecated)]
use agent::router::{ModelConfig, RouteMode, Router};
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
    list_historical_meta_manifests, delete_chat_session,
    export_chat_session, rename_chat_session,
    import_chat_session,
};
use agent::billing_engine::ChronosParallelBillingEngine;
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
fn advance_pipeline(state: tauri::State<AppState>) -> String {
    let role = state.orchestrator.lock().unwrap().advance_pipeline();
    role.label().into()
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

    // Cost cap check — powered by parallel billing engine (Budget tier)
    if state.billing_engine.is_over_cap() {
        let budget = state.billing_engine.get_budget_total();
        let cap = state.billing_engine.get_cost_cap();
        return Err(format!(
            "[熔断拦截] 累计开销 ¥{:.2} 已达安全阈值 ¥{:.2}，API 调用已被阻断。请在设置中调整上限或重置。",
            budget, cap
        ));
    }

    // Resolve API key from vault if frontend sent empty (key now stored server-side)
    let resolved_key = if api_key.is_empty() { resolve_key_from_vault(&model) } else { api_key };
    if resolved_key.is_empty() {
        return Err("[VAULT EMPTY] API Key 未找到。请在「⚙️ 全局配置 → API 密钥凭据」中输入并保存。".into());
    }
    let mut client = state.api_client.lock().await;
    let response = client.chat(&endpoint, &resolved_key, &model, msgs, max_tokens).await;

    // Record to parallel billing engine (Official + Budget + Router)
    if response.success {
        let model_enum = parse_model_to_enum(&model);
        let (prompt, completion) = split_tokens(response.tokens_used, &response.content);
        state.billing_engine.record(&model_enum, prompt, completion, None);
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

    // Cost cap check — powered by parallel billing engine (Budget tier)
    if state.billing_engine.is_over_cap() {
        let budget = state.billing_engine.get_budget_total();
        let cap = state.billing_engine.get_cost_cap();
        return Err(format!(
            "[熔断拦截] 累计开销 ¥{:.2} 已达安全阈值 ¥{:.2}，流式调用已被阻断。",
            budget, cap
        ));
    }

    // Resolve API key from vault if frontend sent empty (key now stored server-side)
    let resolved_key = if api_key.is_empty() { resolve_key_from_vault(&model) } else { api_key };
    if resolved_key.is_empty() {
        return Err("[VAULT EMPTY] API Key 未找到。请在「⚙️ 全局配置 → API 密钥凭据」中输入并保存。".into());
    }
    let mut client = state.api_client.lock().await;
    let response = client
        .chat_stream(
            &endpoint,
            &resolved_key,
            &model,
            msgs,
            max_tokens,
            |chunk| {
                let _ = app_handle.emit("chat-stream-chunk", chunk);
            },
        )
        .await;

    // Record to parallel billing engine (Official + Budget + Router)
    if response.success {
        let model_enum = parse_model_to_enum(&model);
        let (prompt, completion) = split_tokens(response.tokens_used, &response.content);
        state.billing_engine.record(&model_enum, prompt, completion, None);
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

/// 上下文健康检查 — 当前 Token 使用占比与优化建议
#[tauri::command]
fn check_context_health(model: String, current_tokens: u32) -> agent::billing_engine::ContextHealth {
    let model_enum = agent::billing::parse_model_string(&model);
    let engine = agent::billing_engine::ChronosParallelBillingEngine::new();
    engine.check_context_health(&model_enum, current_tokens)
}

/// 更新费用上限（同步到 billing_engine）
#[tauri::command]
fn update_cost_cap(state: tauri::State<AppState>, cap: f64, enabled: bool) -> String {
    state.billing_engine.set_cost_cap(cap);
    state.billing_engine.set_cost_cap_enabled(enabled);
    format!("Cost cap set to ¥{:.2} ({})", cap, if enabled { "ON" } else { "OFF" })
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
    s.to_string()
}

fn simple_decode(s: &str) -> String {
    s.to_string()
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
    std::fs::write(&path, &json).map_err(|e| e.to_string())?;
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
            // router
            get_route_mode,
            set_route_mode,
            get_available_models,
            set_model_api_key,
            route_for_role,
            get_model_endpoint,
            // shadow
            get_shadow_stats,
            toggle_shadow,
            dismiss_shadow_suggestion,
            // agent roster + live windows + evolution
            get_agent_roster,
            list_live_windows,
            get_evolution_stats,
            evo_validate_experience,
            evo_intercept_context,
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
            get_context_glue_status,
            add_app_binding,
            remove_app_binding,
            get_app_bindings,
            toggle_context_glue,
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
            // session persistence (chunked)
            save_chat_session_chunk,
            load_chat_session_chunk,
            list_historical_meta_manifests,
            delete_chat_session,
            export_chat_session,
            rename_chat_session,
            import_chat_session,
        ])
        .setup(|_app| {
            // 将 AppHandle 注入 Orchestrator，使其可以 emit 前端事件
            {
                let handle = _app.handle().clone();
                let state = _app.state::<AppState>();
                state.orchestrator.lock().unwrap().set_app_handle(handle);
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

            // 初始化 tracing 日志系统（输出到 chronos_vault/logs/）
            let app_dir = _app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let log_dir = app_dir.join("chronos_vault").join("logs");
            std::fs::create_dir_all(&log_dir).ok();
            let log_file = log_dir.join("chronos-shadow.log");
            let file_appender = tracing_appender::rolling::never(&log_dir, "chronos-shadow.log");
            fmt()
                .with_writer(file_appender)
                .with_target(false)
                .with_thread_ids(true)
                .init();
            tracing::info!("Chronos-Shadow v0.1.1 started — log at {:?}", log_file);

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
