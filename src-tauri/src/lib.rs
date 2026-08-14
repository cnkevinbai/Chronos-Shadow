// Chronos-Shadow 核心库入口
#![allow(deprecated)] // 旧 Router 将在 v0.2.0 彻底移除

pub mod agent;
pub mod vision;
mod state;

use state::AppState;

use serde::{Deserialize, Serialize};
use tauri::Manager;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::WindowEvent;
use agent::api_client::{ApiClient, ApiResponse, ChatMessage};
use agent::orchestrator::{AgentRole, Orchestrator, OrchestratorStats};
use agent::redline::{RedlineGuard, RedlineStatus};
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
use agent::key_vault::{cache_key, load_key_file, resolve_key_from_vault};
use vision::VisionEngine;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tokio::sync::Mutex as TokioMutex;
use tracing_subscriber::fmt;
use tauri::tray::TrayIconBuilder;
use tauri::menu::{MenuBuilder, MenuItemBuilder};

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
    #[serde(default = "default_project")]
    current_project: String,
}

fn default_project() -> String { "Chronos-Core-Demo".into() }

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
            current_project: "Chronos-Core-Demo".into(),
        }
    }
}

static SETTINGS: std::sync::Mutex<Option<AppSettings>> = std::sync::Mutex::new(None);
pub(crate) static CONFIG_DIR: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

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

// ─── 永不言弃链接抓取 ──────────────────────────────────────────────

#[tauri::command]
async fn indomitable_fetch_url(
    _state: tauri::State<'_, AppState>,
    url: String,
    follow_depth: Option<u8>,
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build().map_err(|e| e.to_string())?;
    let mut domain_states = std::collections::HashMap::new();
    let result = agent::indomitable_fetcher::indomitable_fetch(
        &url, &client, &mut domain_states, follow_depth.unwrap_or(0),
    ).await;
    Ok(serde_json::json!(result))
}

#[tauri::command]
fn extract_urls_from_text(text: String) -> Vec<String> {
    agent::indomitable_fetcher::extract_urls(&text)
}

// ─── PPT 生成引擎 ──────────────────────────────────────────────────

#[tauri::command]
async fn pptx_generate(
    request_json: String,
) -> Result<serde_json::Value, String> {
    let req: agent::pptx_engine::PptGenerationRequest = serde_json::from_str(&request_json)
        .map_err(|e| format!("Invalid request: {}", e))?;
    let engine = agent::pptx_engine::PptxEngine::new();
    let result = engine.generate(&req);
    Ok(serde_json::json!(result))
}

#[tauri::command]
async fn pptx_analyze_reference(
    url: String,
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build().map_err(|e| e.to_string())?;
    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    let html = resp.text().await.map_err(|e| e.to_string())?;
    let engine = agent::pptx_engine::PptxEngine::new();
    let analysis = engine.analyze_reference(&url, &html).await;
    Ok(serde_json::json!(analysis))
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

// ─── 统一行动调度引擎 (Action Dispatch) ────────────────────────────

/// 从 LLM 响应文本中提取所有 JSON 动作块
fn extract_action_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut depth = 0i32;
    let mut start = None;
    let mut in_string = false;
    let mut escape_next = false;

    for (i, ch) in text.char_indices() {
        if escape_next { escape_next = false; continue; }
        if ch == '\\' && in_string { escape_next = true; continue; }
        if ch == '"' { in_string = !in_string; continue; }
        if in_string { continue; } // 跳过字符串内容

        match ch {
            '{' => {
                if depth == 0 { start = Some(i); }
                depth += 1;
            }
            '}' => {
                if depth > 0 { depth -= 1; }
                if depth == 0 {
                    if let Some(s) = start {
                        let block = text[s..=i].to_string();
                        if block.contains("\"action\"") || block.contains("\"actions\"") {
                            // 验证是否为合法 JSON
                            if serde_json::from_str::<serde_json::Value>(&block).is_ok() {
                                blocks.push(block);
                            }
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
            // 用户显式要求抓取 → 用永不言弃抓取器 (无白名单限制, 多策略降级)
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(20))
                .build().map_err(|e| e.to_string())?;
            let mut domain_states = std::collections::HashMap::new();
            let result = agent::indomitable_fetcher::indomitable_fetch(
                url, &client, &mut domain_states, 0,
            ).await;
            if result.success {
                let _ = distill;
                Ok(format!(
                    "✅ 抓取成功: {}\n标题: {}\n\n{}",
                    result.url,
                    result.title.as_deref().unwrap_or("(无标题)"),
                    &result.main_content.chars().take(4000).collect::<String>()
                ))
            } else {
                // 回退到白名单 web_intelligence 抓取
                let mut wi = state.web_intelligence.lock().await;
                let r = wi.fetch(url, distill.unwrap_or(true)).await?;
                Ok(format_fetch_for_llm(&r))
            }
        }
        AgentAction::FileRead { path, range: _ } => {
            let guard = state.cvfs.lock().await;
            let projects = guard.get_projects().await;
            let project_root = projects.first().map(|(_, r)| r.clone())
                .unwrap_or_else(|| state.sandbox.lock().unwrap().project_root.clone());
            drop(guard);
            let full_path = project_root.join(path);
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
            let _args_c = args.clone();
            let result = tokio::task::spawn_blocking(move || {
                // Use a fresh SkillEngine for this blocking call
                let _engine = agent::skill_engine::SkillEngine::new();
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
        AgentAction::CheckEnvironment => {
            let profile = agent::env_checker::get_environment_profile();
            let tool_status: Vec<String> = profile.tools.iter().map(|t| {
                format!("{} {}: {}", if t.installed { "✅" } else { "❌" }, t.name,
                    t.version.as_deref().unwrap_or(if t.installed { "已安装" } else { "未安装" }))
            }).collect();
            Ok(format!(
                "🖥️ 环境剖面:\n\
                OS: {} ({})\n\
                主机: {} @ {}\n\
                主目录: {}\n\
                CPU核心: {} | 磁盘剩余: {:.1}GB\n\n\
                🔧 工具:\n{}\n\n\
                💡 缺失 {} 项工具, 需要安装请告诉我。",
                profile.os, profile.arch, profile.user, profile.hostname,
                profile.home_dir, profile.cpu_cores, profile.disk_free_gb,
                tool_status.join("\n"),
                profile.tools.iter().filter(|t| !t.installed).count()
            ))
        }
        AgentAction::AutoInstallDeps => {
            let report = agent::env_checker::check_environment();
            let results = agent::env_checker::auto_install_missing(&report);
            Ok(format!("🔧 自动安装:\n{}", results.join("\n")))
        }
        AgentAction::PptxGenerate { title, subtitle, author, template, slides } => {
            tracing::info!("[DISPATCH] PptxGenerate: {} ({} slides)", title, slides.as_array().map(|a| a.len()).unwrap_or(0));
            let req = agent::pptx_engine::PptGenerationRequest {
                title: title.clone(),
                subtitle: subtitle.clone(),
                author: author.clone(),
                template: template.as_deref().map(|t| match t {
                    "Corporate"|"企业商务" => agent::pptx_engine::PptTemplate::Corporate,
                    "TechMinimal"|"科技极简" => agent::pptx_engine::PptTemplate::TechMinimal,
                    "Creative"|"创意设计" => agent::pptx_engine::PptTemplate::Creative,
                    "Academic"|"学术答辩" => agent::pptx_engine::PptTemplate::Academic,
                    "MinimalWhite"|"极简白" => agent::pptx_engine::PptTemplate::MinimalWhite,
                    "DarkMode"|"暗夜模式" => agent::pptx_engine::PptTemplate::DarkMode,
                    "vercel_monochrome"|"Vercel"|"VercelMonochrome" => agent::pptx_engine::PptTemplate::VercelMonochrome,
                    "linear_dark_neon"|"Linear"|"LinearDarkNeon" => agent::pptx_engine::PptTemplate::LinearDarkNeon,
                    "apple_minimalist"|"Apple"|"AppleMinimalist" => agent::pptx_engine::PptTemplate::AppleMinimalist,
                    _ => agent::pptx_engine::PptTemplate::Corporate,
                }),
                slides: serde_json::from_value(slides.clone()).unwrap_or_default(),
                reference_url: None,
                output_path: {
                    let cvfs = state.cvfs.lock().await;
                    cvfs.get_projects().await.first().map(|(_, r)| {
                        let safe_name: String = title.chars()
                            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
                            .collect();
                        r.join(format!("{}.pptx", safe_name.trim())).to_string_lossy().to_string()
                    })
                },
            };
            let engine = agent::pptx_engine::PptxEngine::new();
            let result = engine.generate(&req);
            if result.success {
                Ok(format!("✅ PPT 已生成!\n📄 文件: {}\n📊 模板: {}\n📝 幻灯片: {} 页\n💡 安装 python-pptx 后自动生成 .pptx 文件: pip install python-pptx",
                    result.file_path.as_deref().unwrap_or("output.pptx"),
                    result.template_used, result.slide_count))
            } else {
                Err(result.error.unwrap_or_else(|| "PPT 生成失败".into()))
            }
        }
        AgentAction::FileEdit { path, content } => {
            tracing::info!("[DISPATCH] FileEdit: {} ({} bytes)", path, content.len());
            // Get project root from C-VFS, fallback to sandbox
            let project_root = {
                let guard = state.cvfs.lock().await;
                let projects = guard.get_projects().await;
                projects.first().map(|(_, r)| r.clone())
                    .unwrap_or_else(|| state.sandbox.lock().unwrap().project_root.clone())
            };
            let full_path = project_root.join(path);
            if content.len() > 10 * 1024 * 1024 {
                return Err(format!("File too large: {} bytes", content.len()));
            }
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {}", e))?;
            }
            std::fs::write(&full_path, content)
                .map_err(|e| format!("Write failed: {}", e))?;
            tracing::info!("[DISPATCH] File written to {:?}", full_path);
            Ok(format!("✅ 文件已写入: {} ({} bytes)", path, content.len()))
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

/// 从 LLM 响应中提取代码块并自动保存为文件
async fn extract_and_save_code_blocks(
    _state: &AppState,
    text: &str,
) -> Vec<String> {
    let mut files = Vec::new();
    let re = regex::Regex::new(r"```(\w+)?(?:\s+(.+))?\n([\s\S]*?)```").unwrap();

    for cap in re.captures_iter(text) {
        let lang = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let hint = cap.get(2).map(|m| m.as_str()).unwrap_or("").trim();
        let code = cap[3].trim();

        if code.len() < 20 { continue; } // 跳过太短的片段

        // 推断文件名
        let filename = infer_filename(lang, hint, code);
        let path = std::path::Path::new(&filename);
        // 获取 C-VFS 项目根目录 (首个项目路径)
        let project_root = {
            let cvfs = _state.cvfs.lock().await;
            cvfs.get_projects().await.first()
                .map(|(_, r)| r.clone())
                .unwrap_or_else(|| std::path::PathBuf::from("."))
        };
        let full_path = project_root.join(path);

        if let Some(parent) = full_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if std::fs::write(&full_path, code).is_ok() {
            // 返回相对路径 (相对于项目根), 便于后续 cvfs_read_file 回读
            let rel = filename.clone();
            tracing::info!("[AutoSave] Code block → {} (full: {})", rel, full_path.to_string_lossy());
            files.push(rel);
        }
    }
    files
}

/// 根据语言和内容推断文件名 — 支持任意文件类型，无格式限制
fn infer_filename(lang: &str, hint: &str, code: &str) -> String {
    // 1. 用户明确指定文件名 (含扩展名) → 直接使用
    if !hint.is_empty() && hint.contains('.') {
        return hint.to_string();
    }
    // 2. 用户指定文件名但无扩展名 → 用语言作为扩展名
    if !hint.is_empty() && !hint.contains('.') {
        let ext = if lang.is_empty() { "txt" } else { lang };
        return format!("{}.{}", hint, ext);
    }

    // 3. 常见语言映射 (仅作友好别名)
    let ext = match lang {
        "rust" | "rs" => "rs",
        "python" | "py" => "py",
        "javascript" | "js" => "js",
        "typescript" | "ts" => "ts",
        "tsx" => "tsx",
        "jsx" => "jsx",
        "html" => "html",
        "css" => "css",
        "json" => "json",
        "toml" => "toml",
        "yaml" | "yml" => "yml",
        "markdown" | "md" => "md",
        "sql" => "sql",
        "sh" | "bash" => "sh",
        "powershell" | "ps1" => "ps1",
        "java" => "java",
        "go" => "go",
        "cpp" | "c++" => "cpp",
        "c" => "c",
        "svg" => "svg",
        "xml" => "xml",
        "ini" | "conf" => "ini",
        "csv" => "csv",
        "log" => "log",
        "txt" | "text" | "" => "txt",
        // 未知语言 → 直接用语言名作为扩展名，不强制限制
        _ => lang,
    };

    // 4. 尝试从代码首行注释推断具体文件名
    let first_line = code.lines().next().unwrap_or("");
    if first_line.starts_with("//") || first_line.starts_with("#") {
        let comment = first_line.trim_start_matches("//").trim_start_matches("#").trim();
        if comment.len() > 3 && comment.len() < 60 && !comment.contains(' ') {
            return format!("{}.{}", comment, ext);
        }
    }

    format!("generated_{}.{}", chrono::Utc::now().format("%Y%m%d_%H%M%S"), ext)
}

/// Tauri Command: 从 LLM 响应中提取并执行所有动作，返回纯文本结果
/// 如果响应中不含动作块，返回原始文本
#[tauri::command]
async fn extract_and_execute_actions(
    state: tauri::State<'_, AppState>,
    llm_response: String,
) -> Result<serde_json::Value, String> {
    let blocks = extract_action_blocks(&llm_response);

    // 🔬 代码块自动保存 — 无论是否有 JSON 动作都执行
    let auto_files = extract_and_save_code_blocks(&state, &llm_response).await;

    if blocks.is_empty() {
        // 无 JSON 动作, 但可能已保存代码块
        if auto_files.is_empty() {
            return Ok(serde_json::json!({
                "has_actions": false,
                "text_response": llm_response,
                "action_results": []
            }));
        }
        let summary = auto_files.iter()
            .map(|f| format!("  ✅ {}", f))
            .collect::<Vec<_>>()
            .join("\n");
        return Ok(serde_json::json!({
            "has_actions": true,
            "text_response": llm_response,
            "action_results": [],
            "combined_context": format!("✅ 自动保存文件:\n{}", summary),
            "files_created": auto_files,
            "files_summary": format!("📁 自动保存 {} 个代码文件:\n{}", auto_files.len(), summary),
        }));
    }

    let mut action_results = Vec::new();
    let mut combined_results = String::new();
    let mut files_created: Vec<String> = Vec::new();
    let mut files_read: Vec<String> = Vec::new();

    for block in &blocks {
        let action_json = block.clone();
        match execute_agent_action_inner(&state, &action_json).await {
            Ok(result_text) => {
                combined_results.push_str(&result_text);
                combined_results.push_str("\n");
                // Track file operations
                if let Ok(action) = serde_json::from_str::<serde_json::Value>(&action_json) {
                    if action["action"] == "file_edit" {
                        if let Some(path) = action["params"]["path"].as_str() {
                            files_created.push(path.to_string());
                        }
                    } else if action["action"] == "pptx_generate" {
                        let name = action["params"]["title"].as_str().unwrap_or("presentation");
                        let safe: String = name.chars()
                            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
                            .collect();
                        files_created.push(format!("{}.pptx", safe.trim()));
                    } else if action["action"] == "check_environment" || action["action"] == "auto_install_deps" {
                        // 环境检测/安装不产生文件
                    } else if action["action"] == "file_read" {
                        if let Some(path) = action["params"]["path"].as_str() {
                            files_read.push(path.to_string());
                        }
                    }
                }
                action_results.push(serde_json::json!({
                    "action": block, "success": true, "result": result_text
                }));
            }
            Err(e) => {
                tracing::warn!("[DISPATCH] Action failed: {}", e);
                action_results.push(serde_json::json!({
                    "action": block, "success": false, "error": e
                }));
            }
        }
    }

    // 代码块已在函数开头自动保存, 这里合并到 files_created
    for f in &auto_files { files_created.push(f.clone()); }

    // Build file operations summary
    let mut summary = String::new();
    if !files_created.is_empty() {
        summary.push_str(&format!("📁 Created {} files:\n", files_created.len()));
        for f in &files_created { summary.push_str(&format!("  ✅ {}\n", f)); }
    }
    if !files_read.is_empty() {
        summary.push_str(&format!("📖 Read {} files:\n", files_read.len()));
        for f in &files_read { summary.push_str(&format!("  📄 {}\n", f)); }
    }

    Ok(serde_json::json!({
        "has_actions": true,
        "text_response": llm_response,
        "action_results": action_results,
        "combined_context": combined_results,
        "files_created": files_created,
        "files_read": files_read,
        "files_summary": summary,
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
    _state: tauri::State<AppState>,
    message: String,
) -> Result<serde_json::Value, String> {
    let engine = agent::scheduling_engine::AgentSchedulingEngine::new();
    Ok(engine.analyze_enhanced(&message))
}

// ─── Build Status ───────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct BuildFileStatus {
    path: String,
    name: String,
    ext: String,
    size_bytes: u64,
    gzip_size: Option<u64>,
    status: String,
    warnings_count: u32,
    errors_count: u32,
}

#[derive(serde::Serialize)]
struct BuildSummary {
    total_files: usize,
    compiled_files: usize,
    warning_files: usize,
    error_files: usize,
    total_size_bytes: u64,
    total_gzip_bytes: u64,
    total_compile_time_ms: u64,
    build_timestamp: String,
    files: Vec<BuildFileStatus>,
}

#[tauri::command]
fn get_build_status() -> Result<BuildSummary, String> {
    let mut files = Vec::new();
    let dist_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().map(|p| p.join("dist"));
    let target_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release");

    // Scan dist/
    if let Some(ref dist) = dist_dir {
        if dist.exists() {
            scan_dir(dist, "dist", &mut files);
        }
    }

    // Scan target/release for exe/msi
    if target_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&target_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.ends_with(".exe") || name.ends_with(".msi") {
                    let meta = entry.metadata().ok();
                    files.push(BuildFileStatus {
                        path: format!("target/release/{}", name),
                        name: name.into(),
                        ext: path.extension().and_then(|e| e.to_str()).unwrap_or("").into(),
                        size_bytes: meta.as_ref().map(|m| m.len()).unwrap_or(0),
                        gzip_size: None,
                        status: "ok".into(),
                        warnings_count: 0,
                        errors_count: 0,
                    });
                }
            }
        }
    }

    let total = files.len();
    let warnings = files.iter().filter(|f| f.status == "warning").count();
    let errors = files.iter().filter(|f| f.status == "error").count();
    let total_size: u64 = files.iter().map(|f| f.size_bytes).sum();
    let total_gzip: u64 = files.iter().filter_map(|f| f.gzip_size).sum();

    Ok(BuildSummary {
        total_files: total,
        compiled_files: total - errors,
        warning_files: warnings,
        error_files: errors,
        total_size_bytes: total_size,
        total_gzip_bytes: total_gzip,
        total_compile_time_ms: 0,
        build_timestamp: chrono::Utc::now().to_rfc3339(),
        files,
    })
}

fn scan_dir(dir: &std::path::Path, prefix: &str, out: &mut Vec<BuildFileStatus>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").into();
            if path.is_dir() {
                scan_dir(&path, &format!("{}/{}", prefix, name), out);
            } else if let Ok(meta) = entry.metadata() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").into();
                let rel = format!("{}/{}", prefix, name);
                out.push(BuildFileStatus {
                    path: rel,
                    name,
                    ext,
                    size_bytes: meta.len(),
                    gzip_size: None,
                    status: "ok".into(),
                    warnings_count: 0,
                    errors_count: 0,
                });
            }
        }
    }
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
            agent::redline::get_redline_status,
            agent::redline::validate_model_output,
            agent::redline::reset_fuse,
            // orchestrator
            agent::orchestrator::get_pipeline_stats,
            agent::orchestrator::orch_topological_sort,
            agent::orchestrator::orch_parallel_groups,
            agent::orchestrator::orch_executable_tasks,
            agent::orchestrator::orch_schedule_quality,
            agent::orchestrator::orch_smart_retry,
            agent::orchestrator::start_pipeline,
            agent::orchestrator::pause_pipeline,
            agent::orchestrator::resume_pipeline,
            agent::orchestrator::advance_pipeline,
            agent::orchestrator::create_task,
            agent::orchestrator::assign_task,
            agent::orchestrator::complete_task,
            agent::orchestrator::fail_task,
            // api client
            agent::api_client::chat_api,
            agent::api_client::chat_api_stream,
            agent::api_client::cancel_chat_stream,
            agent::api_client::get_cache_hit_stats,
            agent::api_client::check_dev_environment,
            agent::api_client::auto_install_deps,
            // sandbox
            agent::sandbox::init_sandbox,
            agent::sandbox::get_checkpoints,
            // mcp
            agent::mcp_client::mcp_connect_and_init,
            agent::mcp_client::mcp_fetch_tools,
            agent::mcp_client::mcp_disconnect,
            agent::mcp_client::mcp_cleanup_stale,
            // orchestrator management
            agent::orchestrator::prune_orchestrator_tasks,
            agent::orchestrator::flush_dead_letters,
            agent::orchestrator::get_event_metrics,
            // router
            agent::router::get_route_mode,
            agent::router::set_route_mode,
            agent::router::get_available_models,
            agent::router::set_model_api_key,
            agent::router::route_for_role,
            agent::router::get_model_endpoint,
            // hybrid router
            agent::router::hrouter_select_model,
            agent::router::hrouter_get_cluster_status,
            // local analytics
            agent::local_analytics::analytics_record,
            agent::local_analytics::analytics_snapshot,
            agent::local_analytics::analytics_window_metrics,
            agent::local_analytics::analytics_detect_anomalies,
            agent::local_analytics::analytics_correlation,
            agent::local_analytics::analytics_health_score,
            agent::local_analytics::analytics_change_point,
            // state manager
            agent::state_manager::state_save_all,
            agent::state_manager::state_health_report,
            // workbuddy engine
            agent::workbuddy_engine::wb_add_rule,
            agent::workbuddy_engine::wb_record_activity,
            agent::workbuddy_engine::wb_generate_report,
            agent::workbuddy_engine::wb_generate_suggestions,
            // system health
            agent::resilience::get_system_health,
            agent::resilience::get_circuit_breaker_status,
            // user profile
            agent::user_profile::get_user_profile,
            agent::user_profile::update_user_profile,
            agent::user_profile::get_greeting,
            agent::user_profile::get_personalization,
            agent::user_profile::get_heartbeat,
            agent::user_profile::get_achievements,
            agent::user_profile::touch_interaction,
            // security boundary
            agent::security_boundary::check_permission,
            agent::security_boundary::scan_llm_boundary,
            agent::security_boundary::get_security_report,
            // shadow
            agent::shadow::get_shadow_stats,
            agent::shadow::toggle_shadow,
            agent::shadow::dismiss_shadow_suggestion,
            agent::shadow::save_shadow_state,
            agent::shadow::load_shadow_state,
            // agent roster + live windows + evolution
            get_agent_roster,
            list_live_windows,
            indomitable_fetch_url,
            extract_urls_from_text,
            pptx_generate,
            pptx_analyze_reference,
            get_evolution_stats,
            evo_validate_experience,
            evo_intercept_context,
            // agent quality
            agent::evolving::agent_quality::get_agent_quality_scores,
            agent::evolving::agent_quality::record_agent_task_quality,
            agent::evolving::agent_quality::get_global_quality_report,
            // embedding engine
            agent::evolving::embedding::embedding_search,
            agent::evolving::embedding::embedding_add,
            agent::evolving::embedding::embedding_stats,
            // settings persistence
            load_settings,
            save_settings,
            // skill & mcp listing
            list_skills,
            list_mcp_servers,
            // remote cluster
            agent::remote_cluster::cluster_register_server,
            agent::remote_cluster::cluster_unregister_server,
            agent::remote_cluster::cluster_bind_project,
            agent::remote_cluster::cluster_edit_file,
            agent::remote_cluster::cluster_compile,
            agent::remote_cluster::cluster_ping,
            agent::remote_cluster::get_cluster_stats,
            // cvfs
            agent::sandbox::cvfs_create_project,
            agent::sandbox::cvfs_verify_scope,
            agent::sandbox::cvfs_read_file,
            agent::sandbox::cvfs_capture_checkpoint,
            agent::sandbox::cvfs_get_checkpoints,
            agent::sandbox::cvfs_get_projects,
            agent::sandbox::cvfs_delete_project,
            agent::sandbox::cvfs_list_project_files,
            agent::sandbox::cvfs_capture_checkpoint_v2,
            agent::sandbox::cvfs_restore_checkpoint,
            agent::sandbox::cvfs_delete_checkpoint,
            agent::sandbox::cvfs_get_project_health,
            // remote proxy
            agent::remote_proxy::remote_connect,
            agent::remote_proxy::remote_disconnect,
            agent::remote_proxy::remote_list_files,
            agent::remote_proxy::remote_read_file,
            agent::remote_proxy::remote_write_file,
            agent::remote_proxy::remote_compile,
            agent::remote_proxy::remote_snapshot,
            agent::remote_proxy::remote_rewind,
            agent::remote_proxy::get_remote_stats,
            // workbuddy
            agent::buddy_scan::get_buddy_scan_stats,
            agent::buddy_scan::run_buddy_scan,
            agent::buddy_scan::toggle_buddy_scan,
            agent::buddy_scan::get_buddy_saved_cost,
            agent::billing_engine::get_billing_stats,
            agent::billing_engine::get_billing_dashboard,
            agent::billing_engine::update_cost_cap,
            agent::billing_engine::get_model_recommendation,
            agent::billing_engine::check_context_health,
            agent::scheduling_engine::analyze_task,
            agent::hallucination_guard::audit_hallucination,
            agent::context_glue::get_context_glue_status,
            agent::context_glue::add_app_binding,
            agent::context_glue::remove_app_binding,
            agent::context_glue::get_app_bindings,
            agent::context_glue::toggle_context_glue,
            agent::context_glue::save_context_glue_bindings,
            agent::context_glue::load_context_glue_bindings,
            // general
            agent::sandbox::get_sandbox_status,
            agent::sandbox::sandbox_health_check,
            agent::sandbox::sandbox_audit_stats,
            agent::sandbox::sandbox_check_file_size,
            agent::sandbox::sandbox_cleanup_temp,
            agent::billing_engine::get_session_cost,
            agent::buddy_scan::get_saved_cost,
            agent::buddy_scan::get_saving_rate,
            // security vault
            agent::security_vault::get_vault_status,
            agent::security_vault::vault_api_key,
            agent::security_vault::fetch_api_key,
            agent::security_vault::delete_api_key,
            agent::detector::get_detector_stats,
            // lan health
            agent::router::check_lan_health,
            // worktree commands
            agent::worktree::create_worktree,
            agent::worktree::activate_worktree,
            agent::worktree::complete_worktree,
            agent::worktree::merge_worktree,
            agent::worktree::prune_worktree,
            agent::worktree::list_worktrees,
            agent::worktree::get_worktree_stats,
            // approval gate (第四红线)
            agent::approval_gate::submit_for_approval,
            agent::approval_gate::submit_for_approval_with_cost,
            agent::approval_gate::auditor_pre_screen_approval,
            agent::approval_gate::decide_approval,
            agent::approval_gate::list_pending_approvals,
            agent::approval_gate::get_approval_audit_log,
            agent::approval_gate::add_approval_rule,
            agent::approval_gate::remove_approval_rule,
            agent::approval_gate::get_approval_rules,
            agent::approval_gate::get_approval_suggestions,
            agent::approval_gate::expire_stale_approvals,
            agent::approval_gate::save_approval_state,
            agent::approval_gate::load_approval_state,
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
            agent::web_intelligence::web_intel_search,
            agent::web_intelligence::web_intel_fetch,
            agent::web_intelligence::web_intel_research,
            agent::web_intelligence::web_intel_add_domain,
            agent::web_intelligence::web_intel_remove_domain,
            agent::web_intelligence::web_intel_list_domains,
            agent::web_intelligence::web_intel_get_audit_log,
            agent::web_intelligence::web_intel_get_stats,
            agent::web_intelligence::web_intel_save_state,
            agent::web_intelligence::web_intel_load_state,
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
            // build status
            get_build_status,
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

            // 自动恢复进化引擎状态 (EvolutionBus + DataFlywheel + Distillation)
            {
                if let Ok(dir) = _app.handle().path().app_data_dir() {
                    let _ = _app.state::<AppState>().evolution_bus.lock().unwrap().load_state(&dir);
                    let _ = _app.state::<AppState>().flywheel.lock().unwrap().load_state(&dir);
                    let _ = std::fs::create_dir_all(&dir);
                    // Distillation + cache state restore
                    {
                        let app_state = _app.state::<AppState>();
                        let mut wi = app_state.web_intelligence.blocking_lock();
                        let _ = wi.distillation.load_state(&dir);
                        let _ = wi.cache.load_from_disk(&dir);
                    }
                    tracing::info!("[SETUP] Evolution state restored");
                }
            }

            // 自动恢复 C-VFS 项目池
            {
                if let Ok(app_data) = _app.handle().path().app_data_dir() {
                    let state = _app.state::<AppState>();
                    // 使用 Tauri 的 async_runtime::block_on 确保正确阻塞
                    let result: Result<(), String> = tauri::async_runtime::block_on(async {
                        let cvfs = state.cvfs.lock().await;
                        cvfs.load_state(&app_data).await
                    });
                    match result {
                        Ok(()) => {
                            let count = tauri::async_runtime::block_on(async {
                                let cvfs = state.cvfs.lock().await;
                                cvfs.get_projects().await.len()
                            });
                            tracing::info!("[SETUP] C-VFS state loaded with {} projects", count);
                        }
                        Err(e) => tracing::warn!("[SETUP] C-VFS load_state failed: {}", e),
                    }
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
