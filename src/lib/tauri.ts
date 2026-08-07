// Chronos-Shadow Tauri IPC 服务层
// 封装所有前端→后端 invoke 调用，提供类型安全 + 浏览器降级 mock
//
// 在 Tauri 环境中：调用真实的 Rust backend commands
// 在浏览器中（npm run dev）：返回 mock 数据用于 UI 开发

import type {
  RedlineStatus,
  OrchestratorStats,
  VisionSavings,
  BuddyScanStats,
  BuddyScanResult,
  ContextGlueStats,
  AppBinding,
  SystemSnapshot,
  ChatSessionPayload,
  SessionMetaManifest,
} from "./types";

// ─── 环境检测 ──────────────────────────────────────────────────────

let _tauriInvoke: ((cmd: string, args?: Record<string, unknown>) => Promise<unknown>) | null = null;

async function getInvoke() {
  if (_tauriInvoke) return _tauriInvoke;
  try {
    const mod = await import("@tauri-apps/api/core");
    _tauriInvoke = mod.invoke;
    return _tauriInvoke;
  } catch {
    // 浏览器降级：返回 null
    return null;
  }
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const fn = await getInvoke();
  if (fn) return fn(cmd, args) as Promise<T>;
  throw new Error(`Tauri not available — command '${cmd}' requires Tauri runtime`);
}

// ─── Mock 数据（浏览器降级） ───────────────────────────────────────

const MOCK_REDLINE_STATUS: RedlineStatus = {
  schema_active: true,
  schema_last_check: null,
  sandbox_active: true,
  sandbox_root: ".",
  blocked_paths: 0,
  healing_enabled: true,
  max_loop: 3,
  current_loop: 0,
  fused: false,
};

const MOCK_PIPELINE_STATS: OrchestratorStats = {
  total_tasks: 7,
  completed_tasks: 3,
  failed_tasks: 0,
  fused_tasks: 0,
  pending_tasks: 4,
  active_role: "Coder",
  pipeline_running: true,
};

// ─── Redline Commands ──────────────────────────────────────────────

export async function getRedlineStatus(): Promise<RedlineStatus> {
  try {
    return await invoke<RedlineStatus>("get_redline_status");
  } catch {
    return { ...MOCK_REDLINE_STATUS };
  }
}

export async function validateModelOutput(raw: string): Promise<string> {
  try {
    return await invoke<string>("validate_model_output", { raw });
  } catch {
    return `[Mock] Validated: ${raw.slice(0, 50)}...`;
  }
}

export async function resetFuse(): Promise<string> {
  try {
    return await invoke<string>("reset_fuse");
  } catch {
    return "Fuse reset (mock)";
  }
}

// ─── Orchestrator Commands ─────────────────────────────────────────

export async function getPipelineStats(): Promise<OrchestratorStats> {
  try {
    return await invoke<OrchestratorStats>("get_pipeline_stats");
  } catch {
    return { ...MOCK_PIPELINE_STATS };
  }
}

export async function startPipeline(): Promise<string> {
  try {
    return await invoke<string>("start_pipeline");
  } catch {
    return "Pipeline started (mock)";
  }
}

export async function pausePipeline(): Promise<string> {
  try {
    return await invoke<string>("pause_pipeline");
  } catch {
    return "Pipeline paused (mock)";
  }
}

export async function resumePipeline(): Promise<string> {
  try {
    return await invoke<string>("resume_pipeline");
  } catch {
    return "Pipeline resumed (mock)";
  }
}

export async function advancePipeline(): Promise<string> {
  try {
    return await invoke<string>("advance_pipeline");
  } catch {
    return "Advanced (mock)";
  }
}

export async function createTask(
  title: string,
  description: string,
  dependencies: string[],
  priority: number,
): Promise<string> {
  try {
    return await invoke<string>("create_task", {
      title,
      description,
      dependencies,
      priority,
    });
  } catch {
    return `task-mock-${Date.now()}`;
  }
}

// ─── General Commands ──────────────────────────────────────────────

export async function getSandboxStatus(): Promise<string> {
  try {
    return await invoke<string>("get_sandbox_status");
  } catch {
    return "Protected (Mock)";
  }
}

export async function getSessionCost(): Promise<number> {
  try {
    return await invoke<number>("get_session_cost");
  } catch {
    return 0.342;
  }
}

export async function getSavedCost(): Promise<number> {
  try {
    return await invoke<number>("get_saved_cost");
  } catch {
    return 1.82;
  }
}

export async function getSavingRate(): Promise<number> {
  try {
    return await invoke<number>("get_saving_rate");
  } catch {
    return 84;
  }
}

// ─── Router Commands ──────────────────────────────────────────────

export async function getAvailableModels(): Promise<string[]> {
  try {
    return await invoke<string[]>("get_available_models");
  } catch {
    return ["deepseek-v4-pro", "deepseek-v4-flash", "kimi-k3", "kimi-k2.7-code", "kimi-k2.7-code-highspeed", "glm-5.2", "glm-5v-turbo", "glm-5.1", "glm-4.7"];
  }
}

export async function setRouteMode(modeJson: string): Promise<string> {
  try {
    return await invoke<string>("set_route_mode", { modeJson });
  } catch {
    return "Auto-Matrix (mock)";
  }
}

export async function setModelApiKey(modelKey: string, apiKey: string): Promise<string> {
  try {
    return await invoke<string>("set_model_api_key", { modelKey, apiKey });
  } catch {
    return `API key set for ${modelKey} (mock)`;
  }
}

export async function routeForRole(role: string): Promise<string> {
  try {
    return await invoke<string>("route_for_role", { role });
  } catch {
    return "deepseek-v4-flash";
  }
}

export async function getModelEndpoint(modelKey: string): Promise<string> {
  try {
    return await invoke<string>("get_model_endpoint", { modelKey });
  } catch {
    // fallback endpoint map
    if (modelKey.startsWith("deepseek")) return "https://api.deepseek.com/chat/completions";
    if (modelKey.startsWith("kimi")) return "https://api.moonshot.cn/v1/chat/completions";
    if (modelKey.startsWith("glm")) return "https://open.bigmodel.cn/api/paas/v4/chat/completions";
    return "https://api.deepseek.com/chat/completions";
  }
}

// ─── Shadow Mode Commands ─────────────────────────────────────────

export interface ShadowStats {
  state: string;
  enabled: boolean;
  suggestions_generated: number;
  accepted: number;
  dismissed: number;
}

export async function getShadowStats(): Promise<ShadowStats> {
  try {
    return await invoke<ShadowStats>("get_shadow_stats");
  } catch {
    return { state: "Dormant", enabled: false, suggestions_generated: 0, accepted: 0, dismissed: 0 };
  }
}

export async function toggleShadow(enabled: boolean): Promise<string> {
  try {
    return await invoke<string>("toggle_shadow", { enabled });
  } catch {
    return `Shadow mode: ${enabled ? "ON" : "OFF"} (mock)`;
  }
}

// ─── Sandbox Commands ──────────────────────────────────────────────

export async function initSandbox(tools: string[]): Promise<string> {
  try { return await invoke<string>("init_sandbox", { tools }); }
  catch { return "Sandbox initialized (mock)"; }
}

export async function getCheckpoints(): Promise<SystemSnapshot[]> {
  try { return await invoke<SystemSnapshot[]>("get_checkpoints"); }
  catch { return []; }
}

// ─── MCP Commands ──────────────────────────────────────────────────

export async function mcpConnectAndInit(serverId: string): Promise<string> {
  try { return await invoke<string>("mcp_connect_and_init", { serverId }); }
  catch { return `MCP '${serverId}' connected (mock)`; }
}

export async function mcpFetchTools(serverId: string): Promise<string> {
  try { return await invoke<string>("mcp_fetch_tools", { serverId }); }
  catch { return `Fetched tools from '${serverId}' (mock)`; }
}

// ─── Router (was missing) ───────────────────────────────────────

export async function getRouteMode(): Promise<string> {
  try { return await invoke<string>("get_route_mode"); }
  catch { return '"AutoMatrix"'; }
}

// ─── Orchestrator: task lifecycle (was missing) ──────────────────

export async function assignTask(taskId: string, role: string): Promise<string> {
  try { return await invoke<string>("assign_task", { taskId, role }); }
  catch { return `Task ${taskId} assigned (mock)`; }
}

export async function completeTask(taskId: string): Promise<string> {
  try { return await invoke<string>("complete_task", { taskId }); }
  catch { return `Task ${taskId} completed (mock)`; }
}

export async function failTask(taskId: string, error: string): Promise<string> {
  try { return await invoke<string>("fail_task", { taskId, error }); }
  catch { return `Task ${taskId} failed (mock)`; }
}

// ─── Shadow (was missing) ────────────────────────────────────────

export async function dismissShadowSuggestion(id: string): Promise<string> {
  try { return await invoke<string>("dismiss_shadow_suggestion", { id }); }
  catch { return `Suggestion ${id} dismissed (mock)`; }
}

// ─── Vision (估算) ────────────────────────────────────────────────

export async function getVisionSavings(): Promise<VisionSavings> {
  try {
    // NOTE: uses estimated_cost from buddy_scan — no dedicated vision cmd yet
    const buddyCost = await invoke<number>("get_buddy_saved_cost");
    return { blocked_requests: 0, tokens_saved: 0, estimated_cost_saved: buddyCost };
  } catch {
    return { blocked_requests: 12, tokens_saved: 6000, estimated_cost_saved: 0.6 };
  }
}

// ─── Tauri Event Listener ──────────────────────────────────────────

/**
 * 监听后端推送的 pipeline 事件
 * 在 Tauri 环境中实时接收 orchestrator 状态变更
 */
export async function onPipelineEvent(
  callback: (event: { event_type: string; payload: unknown }) => void,
): Promise<() => void> {
  try {
    const { listen } = await import("@tauri-apps/api/event");
    const unlisten = await listen<{ event_type: string; payload: unknown }>(
      "pipeline-event",
      (event) => callback(event.payload),
    );
    return unlisten;
  } catch {
    // Browser fallback — no events
    return () => {};
  }
}

// ─── Chat API ──────────────────────────────────────────────────

interface ChatMessage {
  role: string;
  content: string;
}

interface ApiResponse {
  success: boolean;
  content: string;
  tokens_used: number;
  cost: number;
  cached: boolean;
  error?: string;
}

export async function chatApi(
  endpoint: string,
  apiKey: string,
  model: string,
  messages: ChatMessage[],
  maxTokens?: number,
): Promise<ApiResponse> {
  try {
    return await invoke<ApiResponse>("chat_api", {
      endpoint,
      apiKey,
      model,
      messages: messages.map((m) => ({ role: m.role, content: m.content })),
      maxTokens: maxTokens ?? null,
    });
  } catch {
    return {
      success: false,
      content: "",
      tokens_used: 0,
      cost: 0,
      cached: false,
      error: "API call failed — check endpoint and key",
    };
  }
}

// ─── WorkBuddy: Buddy Scan ──────────────────────────────────────

export async function getBuddyScanStats(): Promise<BuddyScanStats> {
  try {
    return await invoke<BuddyScanStats>("get_buddy_scan_stats");
  } catch {
    return {
      total_scans: 0,
      corrections_applied: 0,
      text_verifications: 0,
      verification_pass_rate: 1.0,
      hallucination_prevented: 0,
      vlm_tokens_saved: 0,
      estimated_cost_saved: 0.0,
      active: true,
    };
  }
}

export async function runBuddyScan(
  targetX: number,
  targetY: number,
  componentLabel: string,
  componentType: string,
  expectedText: string,
): Promise<BuddyScanResult> {
  try {
    return await invoke<BuddyScanResult>("run_buddy_scan", {
      targetX, targetY, componentLabel, componentType, expectedText,
    });
  } catch {
    return {
      safe_to_click: true,
      location: null,
      correction: null,
      verification: null,
      skip_reason: "IPC unavailable (mock)",
    };
  }
}

export async function toggleBuddyScan(enabled: boolean): Promise<string> {
  try { return await invoke<string>("toggle_buddy_scan", { enabled }); }
  catch { return `Buddy Scanner: ${enabled ? "ON" : "OFF"} (mock)`; }
}

export async function getBuddySaved(): Promise<number> {
  try { return await invoke<number>("get_buddy_saved_cost"); }
  catch { return 0.52; }
}

export async function getBillingStats(): Promise<{
  session_cost: number; saved_cost: number; saving_rate: number;
  cost_limit: number; cost_cap_active: boolean;
}> {
  try { return await invoke("get_billing_stats"); }
  catch { return { session_cost: 0.0, saved_cost: 0.0, saving_rate: 0, cost_limit: 5.0, cost_cap_active: true }; }
}

// ─── Parallel Billing Dashboard (v0.2.0) ───────────────────────

export interface CostSnapshot {
  tier: string;
  total_cost_rmb: number;
  tokens_used: number;
  call_count: number;
}

export interface BillingDashboard {
  official: CostSnapshot;
  budget: CostSnapshot;
  router: CostSnapshot;
  cost_cap: number;
  cost_cap_active: boolean;
}

export async function getBillingDashboard(): Promise<BillingDashboard> {
  try { return await invoke<BillingDashboard>("get_billing_dashboard"); }
  catch {
    return {
      official: { tier: "official", total_cost_rmb: 0, tokens_used: 0, call_count: 0 },
      budget: { tier: "budget", total_cost_rmb: 0, tokens_used: 0, call_count: 0 },
      router: { tier: "router", total_cost_rmb: 0, tokens_used: 0, call_count: 0 },
      cost_cap: 5.0, cost_cap_active: true,
    };
  }
}

export async function updateCostCap(cap: number, enabled: boolean): Promise<string> {
  try { return await invoke<string>("update_cost_cap", { cap, enabled }); }
  catch { return `Cost cap mock: ¥${cap.toFixed(2)}`; }
}

// ─── Agent roster ──────────────────────────────────────────────

export interface AgentRosterEntry {
  id: string;
  name: string;
  model: string;
}

export async function getAgentRoster(): Promise<AgentRosterEntry[]> {
  try { return await invoke<AgentRosterEntry[]>("get_agent_roster"); }
  catch {
    return [
      { id: "pm", name: "PM", model: "kimi-k3" },
      { id: "ui", name: "UI Designer", model: "glm-5v-turbo" },
      { id: "arch", name: "Architect", model: "deepseek-v4-pro" },
      { id: "plan", name: "Planner", model: "glm-5.2" },
      { id: "coder", name: "Coder Cluster", model: "deepseek-v4-flash" },
      { id: "audit", name: "Auditor", model: "deepseek-v4-flash" },
      { id: "verify", name: "Verifier", model: "glm-5.2" },
    ];
  }
}

// ─── Live windows ──────────────────────────────────────────────

export interface LiveWindow {
  id: string;
  title: string;
  pid: number;
  hwnd: number;
}

export async function listLiveWindows(): Promise<LiveWindow[]> {
  try { return await invoke<LiveWindow[]>("list_live_windows"); }
  catch { return []; }
}

// ─── Evolution stats ───────────────────────────────────────────

export async function getEvolutionStats(): Promise<Record<string, unknown>> {
  try { return await invoke<Record<string, unknown>>("get_evolution_stats"); }
  catch { return { total_skills: 0, active_skills: 0, skills: [] }; }
}

export async function evoValidateExperience(
  experienceId: string, contextHash: string,
  failedAction: string, correctAction: string, tokenSaved: number,
): Promise<boolean> {
  try {
    return await invoke<boolean>("evo_validate_experience", {
      experienceId, contextHash, failedAction, correctAction, tokenSaved,
    });
  } catch { return false; }
}

export async function evoInterceptContext(contextHash: string): Promise<boolean> {
  try { return await invoke<boolean>("evo_intercept_context", { contextHash }); }
  catch { return false; }
}

// ─── WorkBuddy: Context Glue ────────────────────────────────────

export async function getContextGlueStatus(): Promise<ContextGlueStats> {
  try {
    return await invoke<ContextGlueStats>("get_context_glue_status");
  } catch {
    return {
      apps_bound: 3,
      active_bindings: 2,
      bytes_transferred: 45600,
      tokens_saved: 850,
      estimated_cost_saved: 0.085,
      active: true,
      clipboard_managed: true,
    };
  }
}

export async function addAppBinding(
  sourceApp: string,
  targetApp: string,
  mappingRule: string,
): Promise<string> {
  try {
    return await invoke<string>("add_app_binding", {
      sourceApp, targetApp, mappingRule,
    });
  } catch {
    return `bind-${sourceApp}-${targetApp}`;
  }
}

export async function removeAppBinding(bindingId: string): Promise<boolean> {
  try { return await invoke<boolean>("remove_app_binding", { bindingId }); }
  catch { return true; }
}

export async function getAppBindings(): Promise<AppBinding[]> {
  try { return await invoke<AppBinding[]>("get_app_bindings"); }
  catch {
    return [
      { id: "bind-1", source_app: "excel", target_app: "web", mapping_rule: "direct", active: true, direction: "OneWay", stream_status: { Streaming: 0 } },
      { id: "bind-2", source_app: "web", target_app: "erp", mapping_rule: "field_map", active: true, direction: "OneWay", stream_status: { Streaming: 0 } },
    ];
  }
}

export async function toggleContextGlue(enabled: boolean): Promise<string> {
  try { return await invoke<string>("toggle_context_glue", { enabled }); }
  catch { return `Context Glue: ${enabled ? "ON" : "OFF"} (mock)`; }
}

// ─── Remote Cluster Manager ────────────────────────────────────

export interface ClusterNodeInfo {
  server_id: string;
  host: string;
  connected: boolean;
  projects: string[];
  files_synced: number;
  builds_triggered: number;
}

export interface ClusterStats {
  total_servers: number;
  connected_servers: number;
  total_projects: number;
  active_tunnels: ClusterNodeInfo[];
}

export async function clusterRegisterServer(
  serverId: string, host: string, port: number, username: string,
  authKeyPath?: string, projectRoot?: string,
): Promise<string> {
  return await invoke<string>("cluster_register_server", {
    serverId, host, port, username,
    authKeyPath: authKeyPath ?? null,
    remoteProjectRoot: projectRoot ?? "/tmp",
  });
}

export async function clusterUnregisterServer(serverId: string): Promise<string> {
  try { return await invoke<string>("cluster_unregister_server", { serverId }); }
  catch { return "Unregistered (mock)"; }
}

export async function clusterBindProject(projectId: string, serverId: string): Promise<string> {
  try { return await invoke<string>("cluster_bind_project", { projectId, serverId }); }
  catch { return "Bound (mock)"; }
}

export async function clusterEditFile(projectId: string, filePath: string, content: string): Promise<string> {
  try { return await invoke<string>("cluster_edit_file", { projectId, filePath, content }); }
  catch { return "Edit failed"; }
}

export async function clusterCompile(projectId: string, buildCommand: string): Promise<string> {
  try { return await invoke<string>("cluster_compile", { projectId, buildCommand }); }
  catch { return "Compile failed"; }
}

export async function clusterPing(): Promise<Record<string, boolean>> {
  try { return await invoke<Record<string, boolean>>("cluster_ping"); }
  catch { return {}; }
}

export async function getClusterStats(): Promise<ClusterStats> {
  try { return await invoke<ClusterStats>("get_cluster_stats"); }
  catch { return { total_servers: 0, connected_servers: 0, total_projects: 0, active_tunnels: [] }; }
}

// ─── Remote Development Proxy ──────────────────────────────────

export interface RemoteConfig {
  host: string;
  port: number;
  username: string;
  authKeyPath?: string;
  remoteProjectRoot: string;
}

export interface RemoteSessionStats {
  connected: boolean;
  host: string;
  files_synced: number;
  builds_triggered: number;
  builds_failed: number;
  bytes_transferred: number;
  last_error?: string;
}

export interface RemoteFileNode {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
}

export async function remoteConnect(config: RemoteConfig): Promise<string> {
  return await invoke<string>("remote_connect", {
    host: config.host, port: config.port, username: config.username,
    authKeyPath: config.authKeyPath ?? null, remoteProjectRoot: config.remoteProjectRoot,
  });
}

export async function remoteDisconnect(): Promise<string> {
  try { return await invoke<string>("remote_disconnect"); }
  catch { return "Disconnected (mock)"; }
}

export async function remoteListFiles(subpath: string): Promise<RemoteFileNode[]> {
  try { return await invoke<RemoteFileNode[]>("remote_list_files", { subpath }); }
  catch { return []; }
}

export async function remoteReadFile(path: string): Promise<string> {
  try { return await invoke<string>("remote_read_file", { path }); }
  catch { return ""; }
}

export async function remoteWriteFile(path: string, content: string): Promise<string> {
  try { return await invoke<string>("remote_write_file", { path, content }); }
  catch { return "Write failed"; }
}

export async function remoteCompile(buildCommand: string): Promise<string> {
  try { return await invoke<string>("remote_compile", { buildCommand }); }
  catch { return "Compile failed"; }
}

export async function remoteSnapshot(tag: string): Promise<string> {
  try { return await invoke<string>("remote_snapshot", { tag }); }
  catch { return "Snapshot failed"; }
}

export async function remoteRewind(tag: string): Promise<string> {
  try { return await invoke<string>("remote_rewind", { tag }); }
  catch { return "Rewind failed"; }
}

export async function getRemoteStats(): Promise<RemoteSessionStats> {
  try { return await invoke<RemoteSessionStats>("get_remote_stats"); }
  catch { return { connected: false, host: "", files_synced: 0, builds_triggered: 0, builds_failed: 0, bytes_transferred: 0 }; }
}

// ─── Settings persistence ──────────────────────────────────────

export interface AppSettings {
  version?: number;
  cost_cap: number;
  cost_cap_enabled: boolean;
  ollama_url: string;
  lan_model: string;
  lan_timeout: number;
  auto_fallback: boolean;
  max_healing: number;
  ast_audit: boolean;
  block_gpl: boolean;
  privacy_blur: boolean;
  caching_priority?: boolean;
  accumulated_cost?: number;
  api_key_deepseek?: string;
  api_key_kimi?: string;
  api_key_glm?: string;
  has_key_deepseek?: boolean;
  has_key_kimi?: boolean;
  has_key_glm?: boolean;
}

export async function loadSettings(): Promise<AppSettings> {
  try { return await invoke<AppSettings>("load_settings"); }
  catch {
    return {
      version: 1,
      cost_cap: 5.0, cost_cap_enabled: true,
      ollama_url: "http://localhost:11434", lan_model: "deepseek-v4-flash",
      lan_timeout: 3500, auto_fallback: true,
      max_healing: 3, ast_audit: true, block_gpl: true, privacy_blur: true,
      api_key_deepseek: "", api_key_kimi: "", api_key_glm: "",
    };
  }
}

export async function saveSettings(settings: AppSettings): Promise<string> {
  // Tauri auto-converts camelCase → snake_case for command args
  return await invoke<string>("save_settings", { newSettings: settings });
}

// ─── Skill & MCP listing ────────────────────────────────────────

export interface SkillItem {
  manifest: { id: string; name: string; description: string; version: string };
  state: string;
}

export async function listSkills(): Promise<SkillItem[]> {
  try { return await invoke<SkillItem[]>("list_skills"); }
  catch { return []; }
}

export interface McpServerItem {
  id: string;
  name: string;
  transport: { Stdio?: unknown; Sse?: unknown };
  tools_count: number;
  resources_count: number;
  connected: boolean;
}

export async function listMcpServers(): Promise<McpServerItem[]> {
  try { return await invoke<McpServerItem[]>("list_mcp_servers"); }
  catch { return []; }
}

// ─── Streaming Chat ──────────────────────────────────────────

/**
 * 流式 API 调用 — 通过 Tauri 事件逐 chunk 推送
 * 返回最终 ApiResponse；过程中 emit "chat-stream-chunk" 事件
 */
export async function chatApiStream(
  endpoint: string,
  apiKey: string,
  model: string,
  messages: { role: string; content: string }[],
  maxTokens?: number,
): Promise<ApiResponse> {
  return await invoke<ApiResponse>("chat_api_stream", {
    endpoint,
    apiKey,
    model,
    messages: messages.map((m) => ({ role: m.role, content: m.content })),
    maxTokens: maxTokens ?? null,
  });
}

/**
 * 监听流式 chunk 事件
 * 返回 unsubscribe 函数
 */
export async function onChatStreamChunk(
  callback: (chunk: string) => void,
): Promise<() => void> {
  try {
    const { listen } = await import("@tauri-apps/api/event");
    const unlisten = await listen<string>("chat-stream-chunk", (event) => {
      callback(event.payload);
    });
    return unlisten;
  } catch {
    return () => {};
  }
}

// ─── Session Persistence (Chunked V2) ─────────────────────────

export async function saveChatSessionChunk(
  payload: ChatSessionPayload,
): Promise<void> {
  return await invoke<void>("save_chat_session_chunk", { payload });
}

export async function loadChatSessionChunk(
  sessionId: string,
): Promise<ChatSessionPayload> {
  return await invoke<ChatSessionPayload>("load_chat_session_chunk", {
    sessionId,
  });
}

export async function listHistoricalMetaManifests(): Promise<
  SessionMetaManifest[]
> {
  try {
    return await invoke<SessionMetaManifest[]>(
      "list_historical_meta_manifests",
    );
  } catch {
    return [];
  }
}

export async function deleteChatSession(
  sessionId: string,
): Promise<string> {
  return await invoke<string>("delete_chat_session", { sessionId });
}

export async function exportChatSession(
  sessionId: string,
): Promise<string> {
  return await invoke<string>("export_chat_session", { sessionId });
}

export async function renameChatSession(
  sessionId: string,
  newTitle: string,
): Promise<void> {
  return await invoke<void>("rename_chat_session", {
    sessionId,
    newTitle,
  });
}

export async function checkLanHealth(): Promise<string[]> {
  try {
    return await invoke<string[]>("check_lan_health");
  } catch {
    return [];
  }
}

export async function importChatSession(
  jsonStr: string,
): Promise<SessionMetaManifest> {
  return await invoke<SessionMetaManifest>("import_chat_session", {
    jsonStr,
  });
}

// ─── Security Vault ──────────────────────────────────────────

export async function getVaultStatus(): Promise<Record<string, unknown>> {
  try {
    return await invoke<Record<string, unknown>>("get_vault_status");
  } catch {
    return { vault_active: false, encryption: "unavailable" };
  }
}

export async function vaultApiKey(
  targetModel: string,
  secretKey: string,
): Promise<string> {
  return await invoke<string>("vault_api_key", {
    targetModel,
    secretKey,
  });
}

export async function fetchApiKey(targetModel: string): Promise<string> {
  return await invoke<string>("fetch_api_key", { targetModel });
}

export async function deleteApiKey(targetModel: string): Promise<string> {
  return await invoke<string>("delete_api_key", { targetModel });
}

export async function getDetectorStats(): Promise<{
  total_interceptions: number;
  total_hits: number;
  hit_rate: number;
  tokens_saved: number;
  estimated_cost_saved: number;
}> {
  try {
    return await invoke("get_detector_stats");
  } catch {
    return { total_interceptions: 0, total_hits: 0, hit_rate: 0, tokens_saved: 0, estimated_cost_saved: 0 };
  }
}

// ─── 便捷轮询 Hook ─────────────────────────────────────────────────
// NOTE: createPolling exported for external consumers; not used internally

/**
 * 创建自动轮询的数据源
 * 在 Tauri 环境中每 intervalMs 毫秒刷新一次
 */
export function createPolling(
  fetch: () => Promise<void>,
  intervalMs: number = 2000,
): { start: () => ReturnType<typeof setInterval>; stop: (id: ReturnType<typeof setInterval>) => void } {
  return {
    start: () => setInterval(fetch, intervalMs),
    stop: (id) => clearInterval(id),
  };
}

// ─── C-VFS Commands ────────────────────────────────────────────

export async function cvfsCreateProject(projectId: string, targetPath: string): Promise<string> {
  try { return await invoke<string>("cvfs_create_project", { projectId, targetPath }); }
  catch { return "Created (mock)"; }
}

export async function cvfsVerifyScope(projectId: string, filePath: string): Promise<string> {
  try { return await invoke<string>("cvfs_verify_scope", { projectId, filePath }); }
  catch { return "Scope OK (mock)"; }
}

export async function cvfsCaptureCheckpoint(projectId: string, checkpointId: string, description: string): Promise<string> {
  try { return await invoke<string>("cvfs_capture_checkpoint", { projectId, checkpointId, description }); }
  catch { return "Checkpoint captured (mock)"; }
}

export async function cvfsGetCheckpoints(): Promise<SystemSnapshot[]> {
  try { return await invoke<SystemSnapshot[]>("cvfs_get_checkpoints"); }
  catch { return []; }
}

export async function cvfsGetProjects(): Promise<{ id: string; name: string; path: string }[]> {
  try { return await invoke<{ id: string; name: string; path: string }[]>("cvfs_get_projects"); }
  catch { return []; }
}
